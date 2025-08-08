
use alloy::sol;
use serde::{Deserialize, Serialize};

sol!(
    #[derive(Debug, Default, Serialize, Deserialize)]
    #[sol(rpc)]
    Multicall,
    "../artifacts/Multicall.sol/Multicall.json"
);

pub const MULTICALL_ADDRESS: &str = "0xcA11bde05977b3631167028862bE2a173976CA11";
