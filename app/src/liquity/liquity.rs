use TroveManager::{TroveManagerEvents, TroveChange};
use alloy::{rpc::types::Log, sol, sol_types::SolEvent};
use serde::{Deserialize, Serialize};

sol!(
    #[derive(Debug, Default, Serialize, Deserialize)]
    #[sol(rpc)]
    TroveManager,
    "../artifacts/TroveManager.sol/TroveManager.json"
);


pub fn decode_event_log(log: &Log) -> Option<TroveManagerEvents> {
    if log.topics().is_empty() {
        return None;
    }

    match log.topics()[0] {
        x if x == TroveManager::TroveUpdated::SIGNATURE_HASH => {
            let event = log.log_decode::<TroveManager::TroveUpdated>().unwrap();
            let event = event.data().to_owned();
            return Some(TroveManagerEvents::TroveUpdated(event));
        }
        _ => None,
    }
}
