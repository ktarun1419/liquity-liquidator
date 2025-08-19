use alloy::providers::{IpcConnect, Provider, ProviderBuilder, WsConnect};
use eyre::Result;
use log::{error, info};
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::RwLock,
    time::{Instant, sleep},
};

use crate::strategy::Strategy;

/// Block collector that listens to new blocks and triggers registered strategies
pub struct BlockCollector {
    strategies: Arc<RwLock<Vec<Box<dyn Strategy<u64>>>>>,
    provider: Option<Arc<dyn Provider>>,
}

impl BlockCollector {
    /// Create a new block collector
    pub fn new() -> Self {
        Self { strategies: Arc::new(RwLock::new(Vec::new())), provider: None }
    }

    /// Add a strategy to the collector
    pub async fn add_strategy(&self, strategy: Box<dyn Strategy<u64>>) {
        let mut strategies = self.strategies.write().await;
        info!("Adding strategy: {}", strategy.name());
        strategies.push(strategy);
    }

    pub async fn connect_provider(&mut self, provider: Arc<dyn Provider>) {
        self.provider = Some(provider);
    }

    /// Connect to Ethereum via WebSocket
    pub async fn _connect_ws(&mut self, ws_url: &str) -> Result<()> {
        info!("Connecting to WebSocket: {}", ws_url);

        let ws = WsConnect::new(ws_url);
        let provider = ProviderBuilder::new().connect_ws(ws).await?;

        self.provider = Some(Arc::new(provider));
        info!("Successfully connected to WebSocket");

        Ok(())
    }

    /// Connect to Ethereum via IPC
    pub async fn _connect_ipc(&mut self, ipc_path: &str) -> Result<()> {
        info!("Connecting to IPC: {}", ipc_path);

        let ipc = IpcConnect::new(ipc_path.to_string());
        let provider = ProviderBuilder::new().connect_ipc(ipc).await?;

        self.provider = Some(Arc::new(provider));
        info!("Successfully connected to IPC");

        Ok(())
    }

    pub async fn _sync_history(&self) -> Result<u64> {
        // let provider = self.provider.as_ref().ok_or_else(|| {
        //     eyre::eyre!("Provider not connected. Call connect_ws() or connect_ipc() first")
        // })?;

        let block_number = self._get_current_block_number().await?;
        info!("Syncing history to block #{}", block_number);

        Ok(block_number)
    }

    /// Start listening to new blocks and trigger strategies
    pub async fn start_listening(&self) -> Result<()> {
        let provider = self.provider.as_ref().ok_or_else(|| {
            eyre::eyre!("Provider not connected. Call connect_ws() or connect_ipc() first")
        })?;

        // let mut block_number = current_block_number;

        info!("Starting block listener...");

        // Subscribe to new block headers
        // let subscription = provider.watch_blocks().await?;
        // let mut stream = subscription.into_stream();

        info!("🚀 Block collector is now listening for new blocks");

        loop {
            let start_time = Instant::now();
            let latest_block = provider.get_block_number().await?;
            self.execute_strategies(&latest_block).await;

            // block_number += 1;

            let elapsed = start_time.elapsed();

            if elapsed < Duration::from_secs(1) {
                sleep(Duration::from_secs(1) - elapsed).await;
            }
        }

        // while let Some(hashes) = stream.next().await {
        //     if hashes.is_empty() {
        //         continue;
        //     }
        //     for hash in hashes {
        //         let block = provider.get_block_by_hash(hash).await?;
        //         if block.is_none() {
        //             info!("🔍 Block not found: {:?}", hash);
        //             continue;
        //         }
        //         let block = block.unwrap();
        //         info!("📦 New block received: #{}", block.header.number);
        //         self.execute_strategies(&block.header.number).await;
        //     }
        // }

        // let subscription = provider.subscribe_blocks().await?;
        // let mut stream = subscription.into_stream();
        // while let Some(block) = stream.next().await {
        //     info!("📦 New block received: #{}", block.number);
        //     self.execute_strategies(&block.number).await;
        // }

        // Ok(())
    }

    /// Execute all registered strategies for a given block
    async fn execute_strategies(&self, block_number: &u64) {
        let strategies = self.strategies.read().await;

        let total_strategies = strategies.len();
        info!("🔄 Executing {} strategies for block #{}", total_strategies, block_number);

        // Execute regular strategies
        for strategy in strategies.iter() {
            let strategy_name = strategy.name();
            match strategy.execute(block_number).await {
                Ok(()) => {
                    info!(
                        "✅ Strategy '{}' executed successfully for block #{}",
                        strategy_name, block_number
                    );
                }
                Err(e) => {
                    error!(
                        "❌ Strategy '{}' failed for block #{}: {}",
                        strategy_name, block_number, e
                    );
                }
            }
        }
    }

    /// Get the number of registered strategies
    pub async fn _strategy_count(&self) -> usize {
        let strategies = self.strategies.read().await;
        strategies.len()
    }

    /// Get the names of all registered strategies
    pub async fn _strategy_names(&self) -> Vec<String> {
        let strategies = self.strategies.read().await;

        let mut names = Vec::new();
        for strategy in strategies.iter() {
            names.push(strategy.name().to_string());
        }
        names
    }

    /// Check if the provider is connected
    pub fn _is_connected(&self) -> bool {
        self.provider.is_some()
    }

    /// Get the current block number from the provider
    pub async fn _get_current_block_number(&self) -> Result<u64> {
        let provider =
            self.provider.as_ref().ok_or_else(|| eyre::eyre!("Provider not connected"))?;

        let block_number = provider.get_block_number().await?;
        Ok(block_number)
    }
}

impl Default for BlockCollector {
    fn default() -> Self {
        Self::new()
    }
}

// Type alias for simpler usage
pub type _SimpleBlockCollector = BlockCollector;
