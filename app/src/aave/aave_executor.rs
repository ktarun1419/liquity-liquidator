use alloy::primitives::{Address, Bytes, U256};
use alloy::providers::Provider;
use alloy::providers::ext::TraceApi;

use alloy::{rpc::types::TransactionRequest, sol};
use eyre::Result;
use log::warn;
use serde::Deserialize;
use serde_with::{hex::Hex, serde_as};
use std::{collections::HashMap, fs, sync::Arc};

use crate::DefaultProvider;
use crate::contracts::SwapperInstance;
use crate::db::LiquidationTracker;
use crate::{aave::aave_strategy::StrategyProvider};

sol!(
    #[sol(rpc)]
    Liquidator,
    "../artifacts/Liquidator.sol/Liquidator.json"
);

sol!(
    #[sol(rpc)]
    IPool,
    "../artifacts/IPool.sol/IPool.json"
);

sol!(
    struct SwapData {
        address swapper;
        bytes swapperData;
    }
);

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct PoolInfo {
    #[serde_as(as = "Hex")]
    path: Vec<u8>,
    router: String,
}

#[derive(Debug, Clone)]
pub struct LiquidationOpportunity {
    pub user: Address,
    pub collateral: Address,
    pub debt: Address,
    pub _collateral_amount: U256,
    pub debt_amount: U256,
    pub _profit_usd: U256,
}

pub fn get_pools_from_json(path: &str) -> Result<HashMap<String, PoolInfo>> {
    let data = fs::read_to_string(path);
    match data {
        Ok(data) => {
            let pools = serde_json::from_str(&data)?;
            Ok(pools)
        }
        Err(error) => {
            warn!("error while fetching the file {}", error);
            Err(error.into())
        }
    }
}

#[derive(Debug, Clone)]
pub struct LiquidationExecutor {
    _liquidator_address: Address,
    provider: Arc<StrategyProvider>,
    pools: HashMap<String, PoolInfo>,
    http_provider: Arc<DefaultProvider>,
    liquidator_instance: Liquidator::LiquidatorInstance<Arc<DefaultProvider>>,
    swapper_instance: SwapperInstance,
    liquidation_tracker: LiquidationTracker,
}

impl LiquidationExecutor {
    pub fn new(
        _liquidator_address: Address,
        provider: Arc<StrategyProvider>,
        http_provider: Arc<DefaultProvider>,
        liquidator_instance: Liquidator::LiquidatorInstance<Arc<DefaultProvider>>,
        swapper_instance: SwapperInstance,
        liquidation_tracker: LiquidationTracker,
    ) -> Result<Self> {
        let pools = get_pools_from_json("./app/src/pairs.json")?;

        Ok(Self {
            _liquidator_address,
            provider,
            pools,
            http_provider,
            liquidator_instance,
            swapper_instance,
            liquidation_tracker,
        })
    }

    fn get_fee_for_pair(&self, path: &str) -> Option<&PoolInfo> {
        self.pools.get(path)
    }

    fn encode_liquidation_params(
        &self,
        opportunity: &LiquidationOpportunity,
        router_name: &str,
        path: &[u8],
    ) -> Result<TransactionRequest> {
        let swapper_data =
            self.swapper_instance.encode_swapper_data(router_name, opportunity.collateral, path)?;
        let swapper_bytes: Bytes = swapper_data.into();

        let liquidate_txn = self
            .liquidator_instance
            .liquidate(
                opportunity.collateral,
                opportunity.debt,
                opportunity.user,
                opportunity.debt_amount,
                swapper_bytes,
                self.swapper_instance.swapper_address,
            )
            .gas(1_500_000)
            .gas_price(1_000_000_000)
            .into_transaction_request();

        Ok(liquidate_txn)
    }

    pub async fn execute_liquidation(&self, opportunity: LiquidationOpportunity) -> Result<()> {
        let position_id = format!("{}:{}", opportunity.user, opportunity.collateral);
        if self.liquidation_tracker.is_already_liquidated(&position_id) {
            return Ok(());
        };
        let pair_path = format!("{}_{}", opportunity.collateral, opportunity.debt);
        let pool_info = self
            .get_fee_for_pair(&pair_path)
            .ok_or_else(|| eyre::eyre!("Missing pool info for pair: {}", pair_path))?;

        let liquidate_txn =
            self.encode_liquidation_params(&opportunity, &pool_info.router, &pool_info.path)?;

        self.submit_liquidate_txn(liquidate_txn, &position_id).await
    }

    async fn submit_liquidate_txn(
        &self,
        liquidate_txn: TransactionRequest,
        position_id: &String,
    ) -> Result<()> {
        let sendable_tx = self.liquidator_instance.provider().fill(liquidate_txn).await?;
        let send_result =
            self.http_provider.send_tx_envelope(sendable_tx.as_envelope().unwrap().clone()).await;
        let sent_tx = match send_result {
            Ok(tx) => {
                println!("✅ txn sent: {:?}", tx.tx_hash());
                self.liquidation_tracker.mark_liquidated(position_id);
                tx
            }
            Err(e) => {
                eprintln!("❌ error sending tx: {:#?}", e);
                return Err(e.into());
            }
        };

        // 2) wait for receipt
        let tx_hash = *sent_tx.tx_hash();
        let receipt_result = sent_tx.get_receipt().await;
        let traces = self.provider.trace_transaction(tx_hash).await?;
        for trace in traces.iter() {
            println!("traces are {:?}", trace.trace)
        }
        match receipt_result {
            Ok(receipt_result) => {
                println!("📑 receipt: {:?}", receipt_result);
                receipt_result
            }
            Err(e) => {
                eprintln!("❌ error awaiting receipt: {:#?}", e);
                return Err(e.into());
            }
        };

        Ok(())
    }
}
