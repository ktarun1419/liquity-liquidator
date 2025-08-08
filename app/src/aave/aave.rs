use IPool::IPoolEvents;
use alloy::{rpc::types::Log, sol, sol_types::SolEvent};
use serde::{Deserialize, Serialize};

sol!(
    #[derive(Debug, Default, Serialize, Deserialize)]
    #[sol(rpc)]
    IPool,
    "../artifacts/IPool.sol/IPool.json"
);

sol!(
    #[derive(Debug, Default, Serialize, Deserialize)]
    #[sol(rpc)]
    IPriceOracle,
    "../artifacts/IPriceOracle.sol/IPriceOracle.json"
);

sol!(
    #[derive(Debug, Default, Serialize, Deserialize)]
    #[sol(rpc)]
    IPoolAddressesProvider,
    "../artifacts/IPoolAddressesProvider.sol/IPoolAddressesProvider.json"
);

pub fn decode_event_log(log: &Log) -> Option<IPoolEvents> {
    if log.topics().is_empty() {
        return None;
    }

    match log.topics()[0] {
        x if x == IPool::ReserveDataUpdated::SIGNATURE_HASH => {
            let event = log.log_decode::<IPool::ReserveDataUpdated>().unwrap();
            let event = event.data().to_owned();
            return Some(IPoolEvents::ReserveDataUpdated(event));
        }
        x if x == IPool::Supply::SIGNATURE_HASH => {
            let event = log.log_decode::<IPool::Supply>().unwrap();
            let event = event.data().to_owned();
            return Some(IPoolEvents::Supply(event));
        }
        x if x == IPool::Borrow::SIGNATURE_HASH => {
            let event = log.log_decode::<IPool::Borrow>().unwrap();
            let event = event.data().to_owned();
            return Some(IPoolEvents::Borrow(event));
        }
        x if x == IPool::Repay::SIGNATURE_HASH => {
            let event = log.log_decode::<IPool::Repay>().unwrap();
            let event = event.data().to_owned();
            return Some(IPoolEvents::Repay(event));
        }
        x if x == IPool::Withdraw::SIGNATURE_HASH => {
            let event = log.log_decode::<IPool::Withdraw>().unwrap();
            let event = event.data().to_owned();
            return Some(IPoolEvents::Withdraw(event));
        }
        x if x == IPool::LiquidationCall::SIGNATURE_HASH => {
            let event = log.log_decode::<IPool::LiquidationCall>().unwrap();
            let event = event.data().to_owned();
            return Some(IPoolEvents::LiquidationCall(event));
        }
        x if x == IPool::SwapBorrowRateMode::SIGNATURE_HASH => {
            let event = log.log_decode::<IPool::SwapBorrowRateMode>().unwrap();
            let event = event.data().to_owned();
            return Some(IPoolEvents::SwapBorrowRateMode(event));
        }
        x if x == IPool::IsolationModeTotalDebtUpdated::SIGNATURE_HASH => {
            let event = log.log_decode::<IPool::IsolationModeTotalDebtUpdated>().unwrap();
            let event = event.data().to_owned();
            return Some(IPoolEvents::IsolationModeTotalDebtUpdated(event));
        }
        x if x == IPool::UserEModeSet::SIGNATURE_HASH => {
            let event = log.log_decode::<IPool::UserEModeSet>().unwrap();
            let event = event.data().to_owned();
            return Some(IPoolEvents::UserEModeSet(event));
        }
        x if x == IPool::ReserveUsedAsCollateralEnabled::SIGNATURE_HASH => {
            let event = log.log_decode::<IPool::ReserveUsedAsCollateralEnabled>().unwrap();
            let event = event.data().to_owned();
            return Some(IPoolEvents::ReserveUsedAsCollateralEnabled(event));
        }
        x if x == IPool::ReserveUsedAsCollateralDisabled::SIGNATURE_HASH => {
            let event = log.log_decode::<IPool::ReserveUsedAsCollateralDisabled>().unwrap();
            let event = event.data().to_owned();
            return Some(IPoolEvents::ReserveUsedAsCollateralDisabled(event));
        }
        _ => {}
    }

    None
}
