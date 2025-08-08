use std::{collections::HashMap, sync::Arc};

use crate::{
    aave::{
        LiquidationExecutor,
        aave::{
            IPool,
            IPool::{IPoolEvents, getUserAccountDataCall},
            decode_event_log,
        },
    },
    db::DatabaseStore,
    multicall::{MULTICALL_ADDRESS, Multicall},
    strategy::Strategy,
};
use alloy::{
    eips::BlockNumberOrTag,
    primitives::{Address, FixedBytes, U256, Uint},
    providers::{
        Identity, Provider, RootProvider,
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
    },
    rpc::types::{Filter, Log},
    sol,
    sol_types::{SolCall, SolValue},
};
use eyre::Result;
use log::info;
use std::str::FromStr;

use super::{
    aave::{IPoolAddressesProvider, IPriceOracle::getAssetPriceCall},
    aave_executor::LiquidationOpportunity,
};

sol!(
    function getReserveConfigurationData(address asset) external view returns (uint256 decimals, uint256 ltv, uint256 liquidationThreshold, uint256 liquidationBonus, uint256 reserveFactor, bool usageAsCollateralEnabled, bool borrowingEnabled, bool stableBorrowRateEnabled, bool isActive, bool isFrozen);
);

pub type StrategyProvider = FillProvider<
    JoinFill<
        Identity,
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
    >,
    RootProvider,
>;

const BATCH_SIZE: usize = 300;
const LIQUIDATION_CLOSE_FACTOR_THRESHOLD: &str = "950000000000000000";
const MAX_LIQUIDATION_CLOSE_FACTOR: u64 = 10000;
const DEFAULT_LIQUIDATION_CLOSE_FACTOR: u64 = 5000;
const _PRICE_ONE: u64 = 100_000_000;

#[derive(Debug)]
struct UserAccountData {
    _total_collateral_base: Uint<256, 4>,
    _total_debt_base: Uint<256, 4>,
    user_address: Address,
    health_factor: Uint<256, 4>,
}
#[derive(Debug)]
struct ReserveConfiguration {
    decimals: Uint<256, 4>,
    _ltv: Uint<256, 4>,
    _liquidation_threshold: Uint<256, 4>,
    liquidation_bonus: Uint<256, 4>,
    _reserve_factor: Uint<256, 4>,
    _usage_as_collateral_enabled: bool,
    _borrowing_enabled: bool,
    _stable_borrow_rate_enabled: bool,
    _is_active: bool,
    _is_frozen: bool,
}

/// Aave Pool Strategy that monitors and processes all Aave pool events
pub struct AavePoolStrategy {
    name: String,
    pool_address: Address,
    store: Arc<DatabaseStore>,
    provider: Arc<StrategyProvider>,
    oracle: Address,
    reserves: Vec<Address>,
    reserve_configurations: HashMap<Address, ReserveConfiguration>,
    executor: LiquidationExecutor,
    gateway_address: Address,
}

impl AavePoolStrategy {
    /// Create a new Aave pool strategy
    pub async fn new(
        pool_address: Address,
        store: Arc<DatabaseStore>,
        provider: Arc<StrategyProvider>,
        executor: LiquidationExecutor,
        gateway_address: Address,
        data_provider:Address
    ) -> Self {
        let pool = IPool::new(pool_address, &*provider);
        let addresses_provider_address = pool.ADDRESSES_PROVIDER().call().await.unwrap();

        let addresses_provider =
            IPoolAddressesProvider::new(addresses_provider_address, &*provider);

        let oracle_address = addresses_provider.getPriceOracle().call().await.unwrap();

        let reserves = pool.getReservesList().call().await.unwrap();

        let multicall = Multicall::new(Address::from_str(MULTICALL_ADDRESS).unwrap(), &*provider);
        let mut calls = vec![];

        for reserve in reserves.clone() {
            let call_data = getReserveConfigurationDataCall { asset: reserve }.abi_encode();
            calls.push(Multicall::Call {
                target: data_provider,
                callData: call_data.into(),
            });
        }

        let aggregate_return = multicall.aggregate(calls).call().await.unwrap();
        let return_data = aggregate_return.returnData;
        let mut reserve_configurations = HashMap::new();
        for (_i, (reserve, result_data)) in reserves.iter().zip(return_data.iter()).enumerate() {
            let (
                decimals,
                _ltv,
                _liquidation_threshold,
                liquidation_bonus,
                _reserve_factor,
                _usage_as_collateral_enabled,
                _borrowing_enabled,
                _stable_borrow_rate_enabled,
                _is_active,
                _is_frozen,
            ) = <(
                Uint<256, 4>,
                Uint<256, 4>,
                Uint<256, 4>,
                Uint<256, 4>,
                Uint<256, 4>,
                bool,
                bool,
                bool,
                bool,
                bool,
            )>::abi_decode(result_data)
            .unwrap_or_default();
            let reserve_configuration = ReserveConfiguration {
                decimals,
                _ltv,
                _liquidation_threshold,
                liquidation_bonus,
                _reserve_factor,
                _usage_as_collateral_enabled,
                _borrowing_enabled,
                _stable_borrow_rate_enabled,
                _is_active,
                _is_frozen,
            };
            info!("reserver address {} , reserve config {:?}" , *reserve , reserve_configuration);
            reserve_configurations.insert(*reserve, reserve_configuration);
        }

        Self {
            name: "AavePoolStrategy".to_string(),
            pool_address,
            store,
            provider,
            oracle: oracle_address,
            reserves,
            reserve_configurations,
            executor,
            gateway_address,
        }
    }

    async fn get_real_user(&self, event_user: Address, txn_hash: FixedBytes<32>) -> Result<Address> {
        let mut real_user = event_user;
        if event_user == self.gateway_address {
            let tx = self.provider.get_transaction_by_hash(txn_hash).await?;
            let tx_info = tx.unwrap().into_recovered();
            real_user = tx_info.signer();
        }
        Ok(real_user)
    }
    /// Process a specific Aave event
    async fn process_aave_event(
        &self,
        event: &IPoolEvents,
        block_number: u64,
        txn_hash: FixedBytes<32>,
    ) -> Result<()> {
        match event {
            IPoolEvents::Supply(supply_event) => {
                info!(
                    "🏦 Supply Event - Block: {}, Reserve: {}, User: {}, Amount: {}",
                    block_number,
                    supply_event.reserve,
                    supply_event.onBehalfOf,
                    supply_event.amount
                );
                let user_collateral = self
                    .store
                    .get_user_collateral_by_asset(
                        &supply_event.onBehalfOf.to_string(),
                        &supply_event.reserve.to_string(),
                    )
                    .await?;
                let mut user_collateral_amount = Uint::ZERO + supply_event.amount;
                if user_collateral.is_some() {
                    let user_collateral = user_collateral.unwrap();
                    user_collateral_amount =
                        user_collateral.scaled_balance_typed()? + supply_event.amount;
                }
                self.store
                    .upsert_user_collateral(
                        supply_event.onBehalfOf,
                        supply_event.reserve,
                        user_collateral_amount,
                        block_number as i64,
                    )
                    .await?;
            }

            IPoolEvents::Borrow(borrow_event) => {
                info!(
                    "💰 Borrow Event - Block: {}, Reserve: {}, User: {}, Amount: {}",
                    block_number, borrow_event.reserve, borrow_event.user, borrow_event.amount
                );
                // Get existing debt and add new borrow amount
                let user_debt = self
                    .store
                    .get_user_debt_by_asset(
                        &borrow_event.user.to_string(),
                        &borrow_event.reserve.to_string(),
                    )
                    .await?;
                let mut total_debt_amount = Uint::ZERO + borrow_event.amount;
                if let Some(existing_debt) = user_debt {
                    total_debt_amount = existing_debt.scaled_balance_typed()? + borrow_event.amount;
                }
                self.store
                    .upsert_user_debt(
                        borrow_event.user,
                        borrow_event.reserve,
                        total_debt_amount,
                        block_number as i64,
                    )
                    .await?;
            }

            IPoolEvents::Repay(repay_event) => {
                info!(
                    "💳 Repay Event - Block: {}, Reserve: {}, User: {}, Amount: {}",
                    block_number, repay_event.reserve, repay_event.user, repay_event.amount
                );
                // Get existing debt and subtract repay amount (or set to 0 if repaid more than
                // owed)
                let user_debt = self
                    .store
                    .get_user_debt_by_asset(
                        &repay_event.user.to_string(),
                        &repay_event.reserve.to_string(),
                    )
                    .await?;
                let remaining_debt_amount = if let Some(existing_debt) = user_debt {
                    let current_debt = existing_debt.scaled_balance_typed()?;
                    if repay_event.amount >= current_debt {
                        Uint::ZERO // Fully repaid or overpaid
                    } else {
                        current_debt - repay_event.amount
                    }
                } else {
                    Uint::ZERO // No existing debt
                };

                if remaining_debt_amount == Uint::ZERO {
                    // Delete debt record if fully repaid
                    self.store
                        .delete_user_debt(
                            &repay_event.user.to_string(),
                            &repay_event.reserve.to_string(),
                        )
                        .await?;
                } else {
                    // Update with remaining debt
                    self.store
                        .upsert_user_debt(
                            repay_event.user,
                            repay_event.reserve,
                            remaining_debt_amount,
                            block_number as i64,
                        )
                        .await?;
                }
            }

            IPoolEvents::Withdraw(withdraw_event) => {
               let real_user = self.get_real_user(withdraw_event.user, txn_hash).await?;

                info!(
                    "🏧 Withdraw Event - Block: {}, Reserve: {}, User: {}, Amount: {}",
                    block_number, withdraw_event.reserve, real_user, withdraw_event.amount
                );

                // Get existing collateral and subtract withdraw amount (or set to 0 if withdrawn
                // more than available)
                let user_collateral = self
                    .store
                    .get_user_collateral_by_asset(
                        &real_user.to_string(),
                        &withdraw_event.reserve.to_string(),
                    )
                    .await?;
                let remaining_collateral_amount = if let Some(existing_collateral) = user_collateral
                {
                    let current_collateral = existing_collateral.scaled_balance_typed()?;
                    if withdraw_event.amount >= current_collateral {
                        Uint::ZERO // Fully withdrawn or over-withdrawn
                    } else {
                        current_collateral - withdraw_event.amount
                    }
                } else {
                    Uint::ZERO // No existing collateral
                };

                if remaining_collateral_amount == Uint::ZERO {
                    // Delete collateral record if fully withdrawn
                    self.store
                        .delete_user_collateral(
                            &real_user.to_string(),
                            &withdraw_event.reserve.to_string(),
                        )
                        .await?;
                } else {
                    // Update with remaining collateral
                    self.store
                        .upsert_user_collateral(
                            real_user,
                            withdraw_event.reserve,
                            remaining_collateral_amount,
                            block_number as i64,
                        )
                        .await?;
                }
            }

            IPoolEvents::LiquidationCall(liquidation_event) => {
                info!(
                    "⚡ Liquidation Event - Block: {}, Collateral: {}, Debt: {}, User: {}, Liquidator: {}, Debt Covered: {}, Liquidated Amount: {}",
                    block_number,
                    liquidation_event.collateralAsset,
                    liquidation_event.debtAsset,
                    liquidation_event.user,
                    liquidation_event.liquidator,
                    liquidation_event.debtToCover,
                    liquidation_event.liquidatedCollateralAmount
                );

                // Reduce debt by the amount covered in liquidation
                let user_debt = self
                    .store
                    .get_user_debt_by_asset(
                        &liquidation_event.user.to_string(),
                        &liquidation_event.debtAsset.to_string(),
                    )
                    .await?;
                let remaining_debt_amount = if let Some(existing_debt) = user_debt {
                    let current_debt = existing_debt.scaled_balance_typed()?;
                    if liquidation_event.debtToCover >= current_debt {
                        Uint::ZERO // Debt fully covered
                    } else {
                        current_debt - liquidation_event.debtToCover
                    }
                } else {
                    Uint::ZERO // No existing debt
                };

                if remaining_debt_amount == Uint::ZERO {
                    // Delete debt record if fully liquidated
                    self.store
                        .delete_user_debt(
                            &liquidation_event.user.to_string(),
                            &liquidation_event.debtAsset.to_string(),
                        )
                        .await?;
                } else {
                    // Update with remaining debt
                    self.store
                        .upsert_user_debt(
                            liquidation_event.user,
                            liquidation_event.debtAsset,
                            remaining_debt_amount,
                            block_number as i64,
                        )
                        .await?;
                }

                // Reduce collateral by the amount liquidated
                let user_collateral = self
                    .store
                    .get_user_collateral_by_asset(
                        &liquidation_event.user.to_string(),
                        &liquidation_event.collateralAsset.to_string(),
                    )
                    .await?;
                let remaining_collateral_amount = if let Some(existing_collateral) = user_collateral
                {
                    let current_collateral = existing_collateral.scaled_balance_typed()?;
                    if liquidation_event.liquidatedCollateralAmount >= current_collateral {
                        Uint::ZERO // Collateral fully liquidated
                    } else {
                        current_collateral - liquidation_event.liquidatedCollateralAmount
                    }
                } else {
                    Uint::ZERO // No existing collateral
                };

                if remaining_collateral_amount == Uint::ZERO {
                    // Delete collateral record if fully liquidated
                    self.store
                        .delete_user_collateral(
                            &liquidation_event.user.to_string(),
                            &liquidation_event.collateralAsset.to_string(),
                        )
                        .await?;
                } else {
                    // Update with remaining collateral
                    self.store
                        .upsert_user_collateral(
                            liquidation_event.user,
                            liquidation_event.collateralAsset,
                            remaining_collateral_amount,
                            block_number as i64,
                        )
                        .await?;
                }
            }

            IPoolEvents::ReserveUsedAsCollateralEnabled(
                reserve_used_as_collateral_enabled_event,
            ) => {
                let real_user = self.get_real_user(reserve_used_as_collateral_enabled_event.user, txn_hash).await?;
                info!(
                    "🔒 Reserve Used as Collateral Enabled - Block: {}, Reserve: {}, User: {}",
                    block_number,
                    reserve_used_as_collateral_enabled_event.reserve,
                    real_user
                );

                let user_collateral = self
                    .store
                    .get_user_collateral_by_asset(
                        &real_user.to_string(),
                        &reserve_used_as_collateral_enabled_event.reserve.to_string(),
                    )
                    .await?;
                if let Some(mut user_collateral) = user_collateral {
                    user_collateral.set_enabled(true);
                    self.store
                        .upsert_user_collateral_with_enabled(
                            real_user,
                            reserve_used_as_collateral_enabled_event.reserve,
                            user_collateral.scaled_balance_typed()?,
                            true,
                            block_number as i64,
                        )
                        .await?;
                }
            }

            IPoolEvents::ReserveUsedAsCollateralDisabled(
                reserve_used_as_collateral_disabled_event,
            ) => {
                let real_user = self.get_real_user(reserve_used_as_collateral_disabled_event.user, txn_hash).await?;
                info!(
                    "🔓 Reserve Used as Collateral Disabled - Block: {}, Reserve: {}, User: {}",
                    block_number,
                    reserve_used_as_collateral_disabled_event.reserve,
                    real_user
                );

                let user_collateral = self
                    .store
                    .get_user_collateral_by_asset(
                        &real_user.to_string(),
                        &reserve_used_as_collateral_disabled_event.reserve.to_string(),
                    )
                    .await?;
                if let Some(mut user_collateral) = user_collateral {
                    user_collateral.set_enabled(false);
                    self.store
                        .upsert_user_collateral_with_enabled(
                            real_user,
                            reserve_used_as_collateral_disabled_event.reserve,
                            user_collateral.scaled_balance_typed()?,
                            false,
                            block_number as i64,
                        )
                        .await?;
                }
            }

            IPoolEvents::ReserveDataUpdated(reserve_event) => {
                info!(
                    "📊 Reserve Data Updated - Block: {}, Reserve: {}, Liquidity Rate: {}, Borrow Rate: {}",
                    block_number,
                    reserve_event.reserve,
                    reserve_event.liquidityRate,
                    reserve_event.variableBorrowRate
                );
            }

            _ => {
                info!("📋 Other Aave Event - Block: {}", block_number);
            }
        }

        Ok(())
    }

    /// Check if a log is from the monitored Aave pool
    fn is_pool_log(&self, log: &Log) -> bool {
        log.address() == self.pool_address
    }

    /// Check for liquidation opportunities
    async fn check_for_liquidation_opportunities(&self) -> Result<Vec<UserAccountData>> {
        let users = self.store.get_all_users().await?;
        let mut users_account_data = Vec::new();

        // Batch users into groups of 100
        let user_batches: Vec<_> = users.chunks(BATCH_SIZE).collect();

        info!(
            "Checking {} users for liquidation opportunities in {} batches",
            users.len(),
            user_batches.len(),
        );

        for (batch_index, batch) in user_batches.iter().enumerate() {
            info!("Processing batch {} with {} users", batch_index + 1, batch.len());

            // Create multicall instance
            let multicall_address = Address::from_str(MULTICALL_ADDRESS)?;
            let multicall = Multicall::new(multicall_address, &*self.provider);

            // Prepare multicall data for getUserAccountData calls
            let mut calls = Vec::new();
            for user_address_str in batch.iter() {
                let user_address = Address::from_str(user_address_str)?;

                // Create the call data for getUserAccountData
                let call_data = getUserAccountDataCall { user: user_address }.abi_encode();

                calls.push(Multicall::Call {
                    target: self.pool_address,
                    callData: call_data.into(),
                });
            }

            // Execute the multicall
            match multicall.aggregate(calls).call().await {
                Ok(aggregate_return) => {
                    let return_data = aggregate_return.returnData;

                    // Process the results
                    for (_i, (user_address_str, result_data)) in
                        batch.iter().zip(return_data.iter()).enumerate()
                    {
                        match self.process_user_account_data(user_address_str, result_data).await {
                            Ok((is_liquidatable, user_account_data)) => {
                                if is_liquidatable {
                                    users_account_data.push(user_account_data);
                                }
                            }
                            Err(e) => {
                                info!("❌ Error processing user {}: {:?}", user_address_str, e);
                            }
                        }
                    }
                }
                Err(e) => {
                    info!("❌ Multicall failed for batch {}: {}", batch_index + 1, e);
                    // Continue with next batch even if this one fails
                }
            }
        }

        info!(
            "Found {} liquidation opportunities out of {} users",
            users_account_data.len(),
            users.len()
        );
        Ok(users_account_data)
    }

    /// Process user account data and determine if user is liquidatable
    async fn process_user_account_data(
        &self,
        user_address: &str,
        result_data: &alloy::primitives::Bytes,
    ) -> Result<(bool, UserAccountData)> {
        // Decode the getUserAccountData return values
        // Returns: (totalCollateralBase, totalDebtBase, availableBorrowsBase,
        // currentLiquidationThreshold, ltv, healthFactor)

        // Each value is uint256 (32 bytes), so we should have exactly 192 bytes (6 * 32)
        if result_data.len() != 192 {
            info!(
                "⚠️ Unexpected result data length for user {}: expected 192 bytes, got {}",
                user_address,
                result_data.len()
            );
            return Ok((
                false,
                UserAccountData {
                    user_address: Address::from_str(user_address)?,
                    _total_collateral_base: Uint::ZERO,
                    _total_debt_base: Uint::ZERO,
                    health_factor: Uint::ZERO,
                },
            ));
        }

        let (total_collateral_base, total_debt_base, _, _, _, health_factor) = <(
            Uint<256, 4>,
            Uint<256, 4>,
            Uint<256, 4>,
            Uint<256, 4>,
            Uint<256, 4>,
            Uint<256, 4>,
        )>::abi_decode(
            result_data
        )?;

        // Health factor is in 18 decimals, value < 1e18 means liquidatable
        // Also check that user has both collateral and debt
        let health_factor_threshold = Uint::from(1000000000000000000u64); // 1e18
        let is_liquidatable = health_factor < health_factor_threshold;

        if is_liquidatable {
            info!(
                "🔍 User {} - Health Factor: {}, Total Collateral: {}, Total Debt: {} - LIQUIDATABLE",
                user_address, health_factor, total_collateral_base, total_debt_base
            );
        }

        Ok((
            is_liquidatable,
            UserAccountData {
                user_address: Address::from_str(user_address)?,
                _total_collateral_base: total_collateral_base,
                _total_debt_base: total_debt_base,
                health_factor,
            },
        ))
    }

    async fn get_fresh_prices(&self) -> Result<HashMap<Address, Uint<256, 4>>> {
        let mut prices = HashMap::new();
        // Create multicall instance
        let multicall_address = Address::from_str(MULTICALL_ADDRESS)?;
        let multicall = Multicall::new(multicall_address, &*self.provider);

        let mut calls = vec![];

        for reserve in self.reserves.clone() {
            let call_data = getAssetPriceCall { asset: reserve }.abi_encode();
            calls.push(Multicall::Call { target: self.oracle, callData: call_data.into() });
        }

        let aggregate_return = multicall.aggregate(calls).call().await?;

        let return_data = aggregate_return.returnData;
        for (_i, (reserve, result_data)) in self.reserves.iter().zip(return_data.iter()).enumerate()
        {
            let price = <Uint<256, 4>>::abi_decode(result_data)?;
            prices.insert(*reserve, price);
        }

        Ok(prices)
    }

    async fn get_best_opportunity(
        &self,
        users: &Vec<UserAccountData>,
    ) -> Result<Option<LiquidationOpportunity>> {
        let prices = self.get_fresh_prices().await?;
        info!("prices are {:?}" , prices);
        let mut best_opportunity = None;
        let mut best_bonus = Uint::ZERO;
        for user in users {
            let user_collaterals =
                self.store.get_user_collateral(&user.user_address.to_string()).await?;

            let user_debts = self.store.get_user_debt(&user.user_address.to_string()).await?;
           
            for user_collateral in user_collaterals {
                for user_debt in user_debts.iter() {
                    let collateral_amount = user_collateral.scaled_balance_typed()?;
                    info!("collateral_amount {:?}" , collateral_amount);
                    let debt_amount = user_debt.scaled_balance_typed()?;

                    let collateral_price = prices
                        .get(&user_collateral.collateral_address.parse::<Address>().unwrap())
                        .unwrap();
                    let debt_price =
                        prices.get(&user_debt.debt_address.parse::<Address>().unwrap()).unwrap();

                    let collateral_config = self
                        .reserve_configurations
                        .get(&user_collateral.collateral_address.parse::<Address>().unwrap())
                        .unwrap();
                    let debt_config = self
                        .reserve_configurations
                        .get(&user_debt.debt_address.parse::<Address>().unwrap())
                        .unwrap();
                    info!("debt_config {:?}" , debt_config);

                    let collateral_unit = U256::from(10).pow(collateral_config.decimals.into());
                    let debt_unit = U256::from(10).pow(debt_config.decimals.into());
                    let liquidation_bonus = collateral_config.liquidation_bonus;
                    

                    let close_factor = if user
                        .health_factor
                        .gt(&Uint::from_str(LIQUIDATION_CLOSE_FACTOR_THRESHOLD)?)
                    {
                        Uint::from(DEFAULT_LIQUIDATION_CLOSE_FACTOR)
                    } else {
                        Uint::from(MAX_LIQUIDATION_CLOSE_FACTOR)
                    };

                    let mut debt_to_cover =
                        debt_amount * close_factor / Uint::from(MAX_LIQUIDATION_CLOSE_FACTOR);
                    let base_collateral = (debt_price * debt_to_cover * debt_unit)
                        / (collateral_price * collateral_unit);
                    let mut collateral_to_liquidate =
                        percent_mul(base_collateral, liquidation_bonus);

                        info!("base_collateral {:?} ,liquidation_bonus:{} " , base_collateral , liquidation_bonus);

                    if collateral_to_liquidate > collateral_amount {
                        collateral_to_liquidate = collateral_amount;
                        debt_to_cover = (collateral_price * collateral_to_liquidate * debt_unit)
                            / percent_div(debt_price * collateral_unit, liquidation_bonus);
                    }


                    info!("collateral_to_liquidate:{},  collateral_price: {} ,debt_to_cover * debt_price: {}  " ,  collateral_to_liquidate , collateral_price, debt_to_cover * debt_price);
                    if collateral_to_liquidate * collateral_price < debt_to_cover * debt_price {
                        continue;
                    }
                    let _profit_usd =
                        collateral_to_liquidate * collateral_price - debt_to_cover * debt_price;
                        info!("profit usd for user: {} , is {}:" ,user.user_address, _profit_usd);
                    if _profit_usd > best_bonus.into() {
                        best_bonus = _profit_usd;
                        best_opportunity = Some(LiquidationOpportunity {
                            user: user.user_address,
                            collateral: user_collateral
                                .collateral_address
                                .parse::<Address>()
                                .unwrap(),
                            _collateral_amount: collateral_to_liquidate,
                            debt: user_debt.debt_address.parse::<Address>().unwrap(),
                            debt_amount: debt_to_cover,
                            _profit_usd,
                        });
                    }
                }
            }
        }

        Ok(best_opportunity)
    }
}

#[async_trait::async_trait]
impl Strategy<Log> for AavePoolStrategy {
    async fn execute(&self, log: &Log) -> Result<()> {
        if self.is_pool_log(log) {
            if let Some(event) = decode_event_log(log) {
                self.process_aave_event(
                    &event,
                    log.block_number.unwrap(),
                    log.transaction_hash.unwrap(),
                )
                .await?;
            }
        }
        self.store.set_last_block(log.block_number.unwrap() as i64).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[async_trait::async_trait]
impl Strategy<u64> for AavePoolStrategy {
    async fn execute(&self, block_number: &u64) -> Result<()> {
        info!("🔍 Block: {:?}", block_number);
        let start_time = std::time::Instant::now();

        let filter = Filter::new()
            .address(self.pool_address)
            .from_block(BlockNumberOrTag::Number(*block_number))
            .to_block(BlockNumberOrTag::Number(*block_number));
        let logs = self.provider.get_logs(&filter).await?;
        if logs.is_empty() {
            info!("🔍 No logs found for block: {:?}", block_number);
        } else {
            for log in logs {
                if self.is_pool_log(&log) {
                    if let Some(event) = decode_event_log(&log) {
                        self.process_aave_event(
                            &event,
                            log.block_number.unwrap(),
                            log.transaction_hash.unwrap(),
                        )
                        .await?;
                    }
                }
            }
        }
        self.store.set_last_block(*block_number as i64).await?;
        let end_time = std::time::Instant::now();
        let duration = end_time.duration_since(start_time);
        info!("🔍 Block processing took {:?}", duration);

        let ops = self.check_for_liquidation_opportunities().await?;
        if !ops.is_empty() {
            info!("🔍 Liquidation opportunities found: {:?}", ops);
            let best_opportunity = self.get_best_opportunity(&ops).await?;
            if let Some(best_opportunity) = best_opportunity {
                info!("🔍 Best opportunity found: {:?}", best_opportunity);
                self.executor.execute_liquidation(best_opportunity).await?;
            }
        }
        let end_time = std::time::Instant::now();
        let duration = end_time.duration_since(start_time);
        info!("🔍 Liquidation opportunities check took {:?}", duration);

        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

fn percent_mul(a: U256, bps: U256) -> U256 {
    (U256::from(5000) + (a * bps)) / U256::from(10000)
}

fn percent_div(a: U256, bps: U256) -> U256 {
    let half_bps = bps / U256::from(2);
    (half_bps + (a * U256::from(10000))) / bps
}
