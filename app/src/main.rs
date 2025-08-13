mod collector;
mod db;
mod multicall;
mod strategy;
mod config;
mod liquity;

use collector::{BlockCollector, LogCollector};
use db::{DatabaseStore, initialize_database};
use config::{get_info};

use alloy::{
    network::EthereumWallet,
    providers::{
        fillers::{
            BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
            WalletFiller,
        }, Identity, ProviderBuilder, RootProvider
    },
    signers::local::PrivateKeySigner,
    sol
};
use eyre::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    liquity::{liquity_exexcution::LiquityExecutor, liquity_strategy::LiquityStrategy}
};


pub type DefaultProvider = FillProvider<
    JoinFill<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        WalletFiller<EthereumWallet>,
    >,
    RootProvider,
>;

sol!(
    #[derive(Debug, Default, Serialize, Deserialize)]
    #[sol(rpc)]
    AddressRegistry,
    "../artifacts/AddressRegistry.sol/AddressRegistry.json"
);

sol!(
    #[derive(Debug, Default, Serialize, Deserialize)]
    #[sol(rpc)]
    PriceFeed,
    "../artifacts/PriceFeed.sol/PriceFeed.json"
);

 const PRIVATE_KEY: &str = "0x600640501f924642f7c828e91451599b0d66ddcdb73749bb4178c97bf7a77a3d";

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run -- <protocol-name>");
        std::process::exit(1);
    }

    let protocol = &args[1];
    let config = get_info(protocol).unwrap_or_else(|| {
        eprintln!("Unknown protocol: {}", protocol);
        std::process::exit(1);
    });

    // Initialize the database
    
    let pool = initialize_database(config.database_url).await?;

    
    // Create the store interface
    let store = DatabaseStore::new(pool);
    let store = Arc::new(store);
    

    let mut last_block = store.get_last_block().await?;
    println!("last block from db: {}", last_block);
    if last_block == 0 {
        store.set_last_block(config.start_block as i64).await?;
        last_block = config.start_block as i64;
    }

    //intiailize the instances
    let signer: PrivateKeySigner = PRIVATE_KEY.parse().expect("should parse private key");
    let wallet = EthereumWallet::from(signer);

    let provider = ProviderBuilder::new().connect(config.rpc_url).await?;
    let http_provider = ProviderBuilder::new().wallet(wallet).connect_http(config.rpc_url.parse()?);

    let provider = Arc::new(provider);
    let http_provider: Arc<DefaultProvider> = Arc::new(http_provider);


    let address_registry_instance = AddressRegistry::new(config.address_registry , &*provider);
    let mcr = address_registry_instance.MCR().call().await?;
    let trove_manager = address_registry_instance.troveManager().call().await?;

    let liquity_executor= LiquityExecutor::new(config.liquidator_address, trove_manager ,  http_provider.clone() , provider.clone());

    let liquity_strategy = LiquityStrategy::new(trove_manager, store.clone(), provider.clone() , config.oracle_address , mcr, liquity_executor ).await;

    let mut log_collector = LogCollector::new();
    log_collector.set_contract_address(trove_manager);
    log_collector.set_start_block(last_block as u64);
    log_collector.connect_provider(provider.clone()).await;
    log_collector._add_strategy(Box::new(liquity_strategy.clone())).await;

    loop {
        let new_block = log_collector.start_listening_with_history().await?;
        let current_block = log_collector.get_current_block_number().await?;
        if new_block == current_block {
            break;
        }
    }

    let ws_provider = ProviderBuilder::new().connect_http(config.rpc_url.parse().unwrap());
    let ws_provider = Arc::new(ws_provider);



    let mut block_collector = BlockCollector::new();
    block_collector.connect_provider(ws_provider.clone()).await;
    block_collector.add_strategy(Box::new(liquity_strategy)).await;
    block_collector.start_listening().await?;

    Ok(())
}
