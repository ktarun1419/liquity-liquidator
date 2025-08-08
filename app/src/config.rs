use alloy::primitives::{Address, address};
pub struct ProtocolConfig {
  
    pub address_registry: Address,
    pub oracle_address: Address, 
    pub rpc_url: &'static str,
    pub liquidator_address: Address,
   
    pub start_block: u64,
    pub database_url: &'static str,
}

pub fn get_info(protocol: &str) -> Option<ProtocolConfig> {
    match protocol {
        "felix" => Some(ProtocolConfig {
            address_registry:address!("0x7201fb5c3ba06f10a858819f62221ae2f473815d"),
            oracle_address: address!("0xa8a94Da411425634e3Ed6C331a32ab4fd774aa43"),
            liquidator_address: address!("0x0a032a540febbf4755ab4cd1f4e98c4a51c074c0"),
            rpc_url: "https://rpc.hyperlend.finance/archive",
            start_block: 1093281,
            database_url: "sqlite:felix_main.db",
        }),

        "liquity" => Some(ProtocolConfig {
            address_registry:address!("0x20f7c9ad66983f6523a0881d0f82406541417526"),
            oracle_address: address!("0xa8a94Da411425634e3Ed6C331a32ab4fd774aa43"),
            liquidator_address: address!("0x0a032a540febbf4755ab4cd1f4e98c4a51c074c0"),
            rpc_url: "https://eth.llamarpc.com",
            start_block:  21686212,
            database_url: "sqlite:liquity_main.db",
        }),


        // "hyperyield" => Some(ProtocolConfig {
        //     aave_pool_address: address!("0xC0Fd3F8e8b0334077c9f342671be6f1a53001F12"),
        //     liquidator_address: address!("0x67fc30ecb49a859847f2bdad37c1efda210e8918"),
        //     swapper_address: address!("0xC275a3dc6Ed864BA9FAC3937cdb0C6fA3C553f96"),
        //     gateway_address: address!("0x859A76DFB86b57D249B48B03E35638ffe106a06b"),
        //     rpc_url: "https://rpc.hyperlend.finance/archive",
        //     rpc_mainnet: "https://rpc.hyperliquid.xyz/evm",
        //     start_block: 9325736,
        //     database_url: "sqlite:hyperyield_liquidation.db",
        //     data_provider:address!("0x022F164dDBa35a994ad0f001705e9c187156E244")
        // }),
        _ => None,
    }
}
