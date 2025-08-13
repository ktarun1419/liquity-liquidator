use std::sync::Arc;

use alloy::{
    primitives::{Address, Bytes, Uint, U256}, 
    providers::{
         ext::TraceApi, Provider
    },
     rpc::types::TransactionRequest, sol, sol_types::SolCall
    
};
use eyre::Result;
use serde::{Deserialize, Serialize};
use TroveManager::TroveChange;

use crate::{liquity::{liquity_exexcution::LiquityLiquidator::LiquityLiquidatorInstance, liquity_strategy::StrategyProvider}, DefaultProvider};

sol!(
    #[derive(Debug, Default, Serialize, Deserialize)]
    #[sol(rpc)]
    TroveManager,
    "../artifacts/TroveManager.sol/TroveManager.json"
);

sol!(
    #[derive(Debug, Default, Serialize, Deserialize)]
    #[sol(rpc)]
    LiquityLiquidator,
    "../artifacts/LiquityLiquidator.sol/LiquidationExecutor.json"
);


#[derive(Clone)]
pub struct LiquityExecutor{
    trove_manager:Address,
    liquidator_instance:LiquityLiquidatorInstance<Arc<DefaultProvider>>,
    http_provider: Arc<DefaultProvider>,
    provider: Arc<StrategyProvider>,
}

impl LiquityExecutor{
    pub fn new(liquidator_address:Address, trove_manager:Address,   http_provider: Arc<DefaultProvider>,  provider: Arc<StrategyProvider>,  )->Self{
        let liquidator_instance = LiquityLiquidator::new(liquidator_address ,http_provider.clone());
        Self{
            trove_manager,
            liquidator_instance,
            provider,
            http_provider
        }
    }

      fn encode_call(&self , trove_ids: Vec<Uint<256,4>>)->Result<Vec<u8>>{
       Ok(TroveManager::batchLiquidateTrovesCall{ _troveArray: trove_ids}.abi_encode())
      
    }

    

    async fn submit_liquidate_txn(
        &self,
        liquidate_txn: TransactionRequest,
    ) -> Result<()> {
        let sendable_tx = self.liquidator_instance.provider().fill(liquidate_txn).await?;
        let send_result =
            self.http_provider.send_tx_envelope(sendable_tx.as_envelope().unwrap().clone()).await;
        let sent_tx = match send_result {
            Ok(tx) => {
                println!("✅ txn sent: {:?}", tx.tx_hash());
               
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


    pub async fn execute(&self , trove_ids: Vec<Uint<256,4>>)->Result<()>{
        let encoded_data = self.encode_call(trove_ids)?;
        let encoded_bytes: Bytes = encoded_data.into();

        let liquidate_txn = self.liquidator_instance.execute(self.trove_manager, U256::ZERO, encoded_bytes).gas(1_500_000)
        .gas_price(1_000_000_000)
        .into_transaction_request();

        self.submit_liquidate_txn(liquidate_txn).await
         
    }
}