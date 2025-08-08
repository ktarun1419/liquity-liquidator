use alloy::{
    primitives::Address,
    providers::{IpcConnect, Provider, ProviderBuilder, WsConnect},
    rpc::types::{BlockNumberOrTag, Filter, Log},
};
use eyre::Result;
use log::{error, info};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;

use crate::strategy::Strategy;

const MAX_BLOCKS_PER_REQUEST: u64 = 10000;

/// Log collector that listens to contract logs and triggers registered strategies
pub struct LogCollector {
    strategies: Arc<RwLock<Vec<Box<dyn Strategy<Log>>>>>,
    provider: Option<Arc<dyn Provider>>,
    contract_address: Option<Address>,
    start_block: Option<u64>,
}

impl LogCollector {
    /// Create a new log collector
    pub fn new() -> Self {
        Self {
            strategies: Arc::new(RwLock::new(Vec::new())),
            provider: None,
            contract_address: None,
            start_block: None,
        }
    }

    /// Create a new log collector with contract address and starting block
    pub fn _new_with_config(contract_address: Address, start_block: u64) -> Self {
        Self {
            strategies: Arc::new(RwLock::new(Vec::new())),
            provider: None,
            contract_address: Some(contract_address),
            start_block: Some(start_block),
        }
    }

    /// Set the contract address to monitor
    pub fn set_contract_address(&mut self, address: Address) {
        self.contract_address = Some(address);
        info!("Set contract address to monitor: {:?}", address);
    }

    /// Set the starting block number for log streaming
    pub fn set_start_block(&mut self, block_number: u64) {
        self.start_block = Some(block_number);
        info!("Set starting block number: {}", block_number);
    }

    /// Add a strategy to the collector
    pub async fn _add_strategy(&self, strategy: Box<dyn Strategy<Log>>) {
        let mut strategies = self.strategies.write().await;
        info!("Adding log strategy: {}", strategy.name());
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

    /// Start listening to new logs and trigger strategies
    pub async fn _start_listening(&self) -> Result<()> {
        let provider = self.provider.as_ref().ok_or_else(|| {
            eyre::eyre!("Provider not connected. Call connect_ws() or connect_ipc() first")
        })?;

        let contract_address = self.contract_address.ok_or_else(|| {
            eyre::eyre!("Contract address not set. Call set_contract_address() first")
        })?;

        let start_block = self
            .start_block
            .ok_or_else(|| eyre::eyre!("Start block not set. Call set_start_block() first"))?;

        info!(
            "Starting log listener for contract {:?} from block {}...",
            contract_address, start_block
        );

        // Create a filter for logs from the specified contract starting from the given block
        let filter = Filter::new()
            .address(contract_address)
            .from_block(BlockNumberOrTag::Number(start_block));

        // Subscribe to logs matching the filter
        let subscription = provider.subscribe_logs(&filter).await?;
        let mut stream = subscription.into_stream();

        info!(
            "🚀 Log collector is now listening for new logs from contract {:?}",
            contract_address
        );

        while let Some(log) = stream.next().await {
            let block_number = log.block_number.unwrap_or(0);
            let tx_hash = log.transaction_hash.unwrap_or_default();
            info!("📋 New log received from block #{}, tx: {:?}", block_number, tx_hash);

            // Execute all strategies for this log
            self.execute_strategies(&log, provider).await;
        }

        Ok(())
    }

    /// Start listening to historical logs from the start block to current, then continue with new
    /// logs
    pub async fn start_listening_with_history(&mut self) -> Result<u64> {
        let provider = self.provider.as_ref().ok_or_else(|| {
            eyre::eyre!("Provider not connected. Call connect_ws() or connect_ipc() first")
        })?;

        let contract_address = self.contract_address.ok_or_else(|| {
            eyre::eyre!("Contract address not set. Call set_contract_address() first")
        })?;

        let start_block = self
            .start_block
            .ok_or_else(|| eyre::eyre!("Start block not set. Call set_start_block() first"))?;

        info!(
            "Starting log listener with history for contract {:?} from block {}...",
            contract_address, start_block
        );

        // First, get historical logs
        let current_block = provider.get_block_number().await?;

        let mut start_block = start_block;
        let mut end_block = current_block;
        if end_block - start_block > MAX_BLOCKS_PER_REQUEST {
            end_block = start_block + MAX_BLOCKS_PER_REQUEST;
        }

        loop {
            info!("🕰️  Fetching historical logs from block {} to {}", start_block, end_block);

            let filter = Filter::new()
                .address(contract_address)
                .from_block(BlockNumberOrTag::Number(start_block))
                .to_block(BlockNumberOrTag::Number(end_block));

            let historical_logs = provider.get_logs(&filter).await;
            if historical_logs.is_err() {
                error!("❌ Error fetching historical logs: {:?}", historical_logs.err());
                continue;
            }
            let historical_logs = historical_logs.unwrap();
            info!("📚 Found {} historical logs", historical_logs.len());

            for log in historical_logs {
                self.execute_strategies(&log, provider).await;
            }

            if end_block >= current_block || start_block >= current_block {
                break;
            }

            start_block = end_block + 1;
            end_block = start_block + MAX_BLOCKS_PER_REQUEST;
            if end_block > current_block {
                end_block = current_block;
            }
        }
        self.start_block = Some(current_block);
        Ok(current_block)
    }

    /// Execute all registered strategies for a given log
    async fn execute_strategies(&self, log: &Log, _provider: &Arc<dyn Provider>) {
        let strategies = self.strategies.read().await;
        let block_number = log.block_number.unwrap_or(0);

        let _total_strategies = strategies.len();

        // Execute regular strategies
        for strategy in strategies.iter() {
            let strategy_name = strategy.name();
            match strategy.execute(log).await {
                Ok(()) => {
                    info!(
                        "✅ Strategy '{}' executed successfully for log from block #{}",
                        strategy_name, block_number
                    );
                }
                Err(e) => {
                    error!(
                        "❌ Strategy '{}' failed for log from block #{}: {}",
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

    /// Check if the collector is properly configured
    pub fn _is_configured(&self) -> bool {
        self.contract_address.is_some() && self.start_block.is_some()
    }

    /// Get the current block number from the provider
    pub async fn get_current_block_number(&self) -> Result<u64> {
        let provider =
            self.provider.as_ref().ok_or_else(|| eyre::eyre!("Provider not connected"))?;

        let block_number = provider.get_block_number().await?;
        Ok(block_number)
    }

    /// Get the contract address being monitored
    pub fn _get_contract_address(&self) -> Option<Address> {
        self.contract_address
    }

    /// Get the starting block number
    pub fn _get_start_block(&self) -> Option<u64> {
        self.start_block
    }
}

impl Default for LogCollector {
    fn default() -> Self {
        Self::new()
    }
}

// Type alias for simpler usage
pub type _SimpleLogCollector = LogCollector;
