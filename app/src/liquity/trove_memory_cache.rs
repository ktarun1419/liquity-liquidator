use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use eyre::Result;
use log::info;


use crate::db::{DatabaseStore,  store::Trove};


#[derive(Clone, Debug)]
pub struct CachedTrovesData {
    pub troves: Vec<Trove>, // Replace with your actual Trove struct
    pub cached_at: u64,
}

#[derive(Clone)]
pub struct TroveMemoryCache {
    cached_data: Arc<RwLock<Option<CachedTrovesData>>>,
    cache_ttl_seconds: u64,
}

impl TroveMemoryCache {
    pub fn new(cache_ttl_seconds: u64) -> Self {
        Self {
            cached_data: Arc::new(RwLock::new(None)),
            cache_ttl_seconds,
        }
    }

    fn get_current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Get sorted troves - from memory if available and valid, otherwise from DB
    pub async fn get_sorted_troves(&self, store: &Arc<DatabaseStore>) -> Result<Vec<Trove>> {
        let now = Self::get_current_timestamp();
        
        // Check if we have valid cached data
        {
            let cached = self.cached_data.read().unwrap();
            if let Some(ref data) = *cached {
                // Check if cache is still valid
                if now.saturating_sub(data.cached_at) <= self.cache_ttl_seconds {
                    info!("Cache hit - returning {} troves from memory", data.troves.len());
                    return Ok(data.troves.clone());
                }
                info!("Cache expired - fetching fresh data from DB");
            } else {
                info!("Cache empty - fetching initial data from DB");
            }
        }

        // Cache miss or expired - fetch from database
        let troves_from_db = store.get_all_active_troves().await?;
        
        if troves_from_db.is_empty() {
            info!("No active troves found in DB");
            return Ok(Vec::new());
        }

        info!("Fetched {} troves from DB, caching in memory", troves_from_db.len());

        // Store in cache
        {
            let mut cached = self.cached_data.write().unwrap();
            *cached = Some(CachedTrovesData {
                troves: troves_from_db.clone(),
                cached_at: now,
            });
        }

        Ok(troves_from_db)
    }

    /// Clear the memory cache
    pub fn clear_memory(&self) {
        let mut cached = self.cached_data.write().unwrap();
        *cached = None;
        info!("Memory cache cleared");
    }

    /// Get cache info for debugging
    pub fn _get_cache_info(&self) -> _CacheInfo {
        let cached = self.cached_data.read().unwrap();
        let now = Self::get_current_timestamp();
        
        match *cached {
            Some(ref data) => {
                let age_seconds = now.saturating_sub(data.cached_at);
                let is_valid = age_seconds <= self.cache_ttl_seconds;
                
                _CacheInfo {
                    is_cached: true,
                    trove_count: data.troves.len(),
                    age_seconds,
                    is_valid,
                    ttl_seconds: self.cache_ttl_seconds,
                }
            },
            None => _CacheInfo {
                is_cached: false,
                trove_count: 0,
                age_seconds: 0,
                is_valid: false,
                ttl_seconds: self.cache_ttl_seconds,
            }
        }
    }
}

#[derive(Debug)]
pub struct _CacheInfo {
    pub is_cached: bool,
    pub trove_count: usize,
    pub age_seconds: u64,
    pub is_valid: bool,
    pub ttl_seconds: u64,
}
