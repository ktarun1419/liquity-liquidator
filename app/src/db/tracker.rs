use dashmap::DashMap;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug)]
struct LiquidationRecord {
    timestamp: u64, // For optional expiry logic
}

#[derive(Debug, Clone)]
pub struct LiquidationTracker {
    records: Arc<DashMap<String, LiquidationRecord>>,
}

impl LiquidationTracker {
    pub fn new() -> Self {
        Self {
            records: Arc::new(DashMap::new()),
        }
    }

    pub fn mark_liquidated(&self, position_id: &str) {
        let record = LiquidationRecord {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        self.records.insert(position_id.to_string(), record);
    }

    pub fn is_already_liquidated(&self, position_id: &str) -> bool {
        self.records.contains_key(position_id)
    }
}
