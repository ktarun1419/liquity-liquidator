use eyre::Result;

/// Trait that defines a strategy that can be executed when a new block is received
#[async_trait::async_trait]
pub trait Strategy<T>: Send + Sync + 'static {
    /// Execute the strategy with the new block data
    async fn execute(&self, tick: &T) -> Result<()>;

    /// Get the name of this strategy for logging purposes
    fn name(&self) -> &str;
}
