use std::sync::Arc;
use alloy::sol_types::SolCall;
use eyre::Result;

use alloy::primitives::Address;
use alloy::{sol};

use crate::DefaultProvider;

sol!(
    #[sol(rpc)]
    BaseSwapper,
    "../artifacts/Swapper.sol/Swapper.json"
);

#[derive(Debug, Clone)]
pub struct SwapperInstance {
    pub swapper_address: Address,
    pub _http_provider: Arc<DefaultProvider>,
}

impl SwapperInstance {
    pub fn new(swapper_address: Address, _http_provider: Arc<DefaultProvider>) -> Result<Self> {
        return Ok(Self { swapper_address, _http_provider });
    }

    pub fn encode_swapper_data(
        &self,
        router_name: &str,
        token: Address,
        path: &[u8],
    ) -> Result<Vec<u8>> {
        match router_name {
            "kittenRouterSwap" => {
                Ok(BaseSwapper::kittenRouterSwapCall { token, path: path.to_vec().into() }
                    .abi_encode())
            }
            "laminarRouterSwap" => {
                Ok(BaseSwapper::laminarRouterSwapCall { token, path: path.to_vec().into() }
                    .abi_encode())
            }
            "hyperswapRouterSwap" => {
                Ok(BaseSwapper::hyperswapRouterSwapCall { token, path: path.to_vec().into() }
                    .abi_encode())
            }
            _ => Err(eyre::eyre!("Unknown router function: {}", router_name)),
        }
    }


}
