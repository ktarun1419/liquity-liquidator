use std::sync::Arc;

use crate::{
    db::{DatabaseStore, store::Trove},
    liquity::{
        liquity::{TroveManager::TroveManagerEvents, decode_event_log},
        liquity_exexcution::LiquityExecutor,
        trove_memory_cache::TroveMemoryCache,
    },
    strategy::Strategy,
};
use TroveManager::TroveChange;
use alloy::{
    eips::BlockNumberOrTag,
    primitives::{Address, U256, Uint},
    providers::{
        Identity, Provider, RootProvider,
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
    },
    rpc::types::{Filter, Log},
    sol,
};
use eyre::Result;
use log::info;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

sol!(
    #[derive(Debug, Default, Serialize, Deserialize)]
    #[sol(rpc)]
    AggregatePriceFeed,
    "../artifacts/IAggregatePrice.sol/IAggregatePrice.json"
);
// Liquity ABIs (simplified; get full from docs)
sol!(
    #[derive(Debug, Default, Serialize, Deserialize)]
    #[sol(rpc)]
    TroveManager,
    "../artifacts/TroveManager.sol/TroveManager.json"
);

sol!(
    #[derive(Debug, Default, Serialize, Deserialize)]
    #[sol(rpc)]
    AddressRegistry,
    "../artifacts/AddressRegistry.sol/AddressRegistry.json"
);

sol!(
    interface ChainlinkOracle {
        function latestAnswer() external view returns (int256);
    }
);

pub type StrategyProvider = FillProvider<
    JoinFill<
        Identity,
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
    >,
    RootProvider,
>;

static DECIMAL_PRECISION: u128 = 1_000_000_000_000_000_000u128;
const ONE_YEAR: u64 = 31_536_000;

/// Liquity Strategy that monitors and processes TroveUpdated events
#[derive(Clone)]

pub struct LiquityStrategy {
    name: String,
    trove_manager: Address,
    store: Arc<DatabaseStore>,
    provider: Arc<StrategyProvider>,
    oracle: Address,
    mcr: Uint<256, 4>,         // Chainlink ETH/USD
    executor: LiquityExecutor, // Your adapted executor
    memory_cache: TroveMemoryCache,
}

impl LiquityStrategy {
    /// Create a new Liquity strategy
    pub async fn new(
        trove_manager: Address,
        store: Arc<DatabaseStore>,
        provider: Arc<StrategyProvider>,
        oracle_address: Address,
        mcr: Uint<256, 4>,
        executor: LiquityExecutor,
    ) -> Self {

        let memory_cache = TroveMemoryCache::new(2000000);

        Self {
            name: "LiquityStrategy".to_string(),
            trove_manager,
            store,
            provider,
            oracle: oracle_address,
            mcr,
            executor, // executor,
            memory_cache
        }
    }

    /// Process a TroveUpdated event
    async fn process_trove_event(
        &self,
        events: &TroveManagerEvents,
        block_number: u64,
    ) -> Result<()> {
        match events {
            TroveManagerEvents::TroveUpdated(event) => {
                info!(
                    "🛡️ TroveUpdated - Block: {}, trove_id: {}, Debt: {}, Coll: {} , IntersestRate: {}",
                    block_number,
                    event._troveId,
                    event._debt,
                    event._coll,
                    event._annualInterestRate
                );

                let trove_id = event._troveId.to_string();
                let coll = event._coll;
                let debt = event._debt;

                // Handle zero debt/coll as closed/liquidated
                let status = if debt == Uint::ZERO && coll == Uint::ZERO {
                    "closed".to_string()
                } else {
                    "active".to_string()
                };

                // For DB insert, compute full ICR with current price (but we'll re-compute on checks)
                let icr = if debt != Uint::ZERO { (coll) / debt } else { Uint::ZERO };
                let icr_numeric: f64 = icr.try_into().unwrap(); // For sorting

                self.store
                    .upsert_trove(&Trove {
                        trove_id,
                        collateral: coll.to_string(),
                        debt: debt.to_string(),
                        icr: icr.to_string(),
                        icr_numeric,
                        status,
                        interest_rate: event._annualInterestRate.to_string(),
                        last_updated: block_number as i64,
                    })
                    .await?;
            }
            _ => {
                info!("📋 Other Aave Event - Block: {}", block_number);
            }
        }
        Ok(())
    }

    /// Check for liquidation opportunities
    async fn check_for_liquidation_opportunities(
        &self,
        timestamp: u64,
    ) -> Result<Vec<Uint<256, 4>>> {
        let mut liquidatable: Vec<Uint<256, 4>> = Vec::with_capacity(32);

        // Get troves - from memory if available, otherwise from DB
        let sorted_troves = self.memory_cache.get_sorted_troves(&self.store).await?;

        if sorted_troves.is_empty() {
            return Ok(liquidatable);
        }

        info!("Checking {} troves for liquidation", sorted_troves.len());

        // Fetch oracle price once
        let price = self.get_oracle_price().await?;
        let mcr = self.mcr;
        let zero = Uint::<256, 4>::ZERO;
        let timestamp_u64 = timestamp;

        // Process troves with minimal allocations
        for trove in sorted_troves {
            // Early validation - skip invalid troves immediately
            let coll = match Uint::<256, 4>::from_str(&trove.collateral) {
                Ok(val) if val != zero => val,
                _ => continue,
            };

            let debt = match Uint::<256, 4>::from_str(&trove.debt) {
                Ok(val) if val != zero => val,
                _ => continue,
            };

            let interest_rate = match Uint::<256, 4>::from_str(&trove.interest_rate) {
                Ok(val) => val,
                Err(_) => continue,
            };

            // Calculate ICR
            let full_icr = self.calculate_full_icr(
                debt,
                coll,
                interest_rate,
                timestamp_u64,
                trove.last_updated as u64,
                price,
            );

            if full_icr < mcr {
                // Parse trove_id only when needed
                if let Ok(trove_id) = Uint::<256, 4>::from_str(&trove.trove_id) {
                    liquidatable.push(trove_id);
                    {
                        info!("🔍 Trove {} - ICR: {} - LIQUIDATABLE", &trove.trove_id, full_icr);
                        info!("🔍 Collateral {} - Debt {}", coll, debt);
                    }
                }
            } else {
                // Since troves are sorted by risk, we can break early

                {
                    info!(
                        "🔍 Trove {} - ICR: {} - NOT LIQUIDATABLE (stopping)",
                        &trove.trove_id, full_icr
                    );
                    info!("🔍 Collateral {} - Debt {}", coll, debt);
                }
                break;
            }
        }

        // If we found liquidatable troves, clear cache so next call gets fresh data
        if !liquidatable.is_empty() {
            info!(
                "Found {} liquidatable troves - clearing cache for next iteration",
                liquidatable.len()
            );
            self.memory_cache.clear_memory();
        }

        if !liquidatable.is_empty() {
            info!("Found {} liquidatable troves for batching", liquidatable.len());
        }

        Ok(liquidatable)
    }

    /// Get fresh oracle price (ETH/USD)
    async fn get_oracle_price(&self) -> Result<Uint<256, 4>> {
        let price_feed = AggregatePriceFeed::new(self.oracle, &*self.provider);
        let price_i256 = price_feed.latestAnswer().call().await?;

        let price = Uint::from(price_i256) * Uint::from(10u64.pow(10));

        if price <= Uint::ZERO {
            return Err(eyre::eyre!("Invalid oracle price"));
        }

        // Convert from 8 decimals to 18 decimals by multiplying by 10^10

        Ok(price)
    }

    pub fn calculate_full_icr(
        &self,
        debt: Uint<256, 4>,
        coll: Uint<256, 4>,
        interest_rate: Uint<256, 4>,
        block_timestamp: u64,
        last_updated: u64,
        price: Uint<256, 4>,
    ) -> Uint<256, 4> {
        // weightedRecordedDebt = recordedDebt * annualInterestRate
        let weighted_recorded_debt = debt.saturating_mul(interest_rate);

        let period_secs_u64 = block_timestamp.saturating_sub(last_updated);
        let period_u256 = U256::from(period_secs_u64);

        let accrued_interest = Self::calc_interest(weighted_recorded_debt, period_u256);

        let entire_debt = debt + accrued_interest;

        // entireColl = coll + redistCollGain
        let entire_coll = coll;

        (entire_coll * price) / entire_debt
    }

    pub fn calc_interest(weighted_debt: Uint<256, 4>, period: Uint<256, 4>) -> Uint<256, 4> {
        let num = weighted_debt.saturating_mul(period);
        let after_year = num / Uint::from(ONE_YEAR) / Uint::from(DECIMAL_PRECISION);
        after_year
    }
}

#[async_trait::async_trait]
impl Strategy<Log> for LiquityStrategy {
    async fn execute(&self, log: &Log) -> Result<()> {
        if log.address() == self.trove_manager {
            if let Some(event) = decode_event_log(log) {
                // Custom decoder function
                self.process_trove_event(&event, log.block_number.unwrap()).await?;
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
impl Strategy<u64> for LiquityStrategy {
    async fn execute(&self, block_number: &u64) -> Result<()> {
        info!("🔍 Block: {:?}", block_number);
        let start_time = std::time::Instant::now();
       

        let filter = Filter::new()
            .address(self.trove_manager)
            .from_block(BlockNumberOrTag::Number(*block_number))
            .to_block(BlockNumberOrTag::Number(*block_number));
        let logs = self.provider.get_logs(&filter).await?;

        for log in logs {
            if log.address() == self.trove_manager {
                if let Some(event) = decode_event_log(&log) {
                    self.process_trove_event(&event, log.block_number.unwrap()).await?;
                    self.memory_cache.clear_memory();
                }
            }
        }
        self.store.set_last_block(*block_number as i64).await?;

        let timestamp = *block_number;

        let ops = self.check_for_liquidation_opportunities(timestamp).await?;
        if !ops.is_empty() {
            info!("🔍 Liquidation opportunities found: {:?}", ops.len());
            self.executor.execute(ops).await?;
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
