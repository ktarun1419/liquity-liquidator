use alloy::primitives::{Address, Uint};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, SqlitePool};

use eyre::Result;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserCollateral {
    pub id: i64,
    pub user_address: String,
    pub collateral_address: String,
    pub scaled_balance: String,
    pub enabled: bool,
    pub last_updated: i64,
}

impl UserCollateral {
    pub fn _user_address_typed(&self) -> Result<Address> {
        Address::from_str(&self.user_address).map_err(|e| eyre::eyre!("Invalid address: {}", e))
    }

    pub fn _collateral_address_typed(&self) -> Result<Address> {
        Address::from_str(&self.collateral_address)
            .map_err(|e| eyre::eyre!("Invalid address: {}", e))
    }

}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AssetConfig {
    pub id: i64,
    pub asset_address: String,
    pub symbol: String,
    pub decimals: i32,
    pub ltv: i32,
    pub liquidation_threshold: i32,
    pub liquidation_bonus: i32,
    pub reserve_factor: i32,
    pub is_active: bool,
    pub is_frozen: bool,
    pub borrowing_enabled: bool,
    pub stable_borrow_rate_enabled: bool,
    pub a_token_address: String,
    pub stable_debt_token_address: String,
    pub variable_debt_token_address: String,
    pub interest_rate_strategy_address: String,
    pub last_updated: i64,
}

impl AssetConfig {
    pub fn _asset_address_typed(&self) -> Result<Address> {
        Address::from_str(&self.asset_address).map_err(|e| eyre::eyre!("Invalid address: {}", e))
    }

    pub fn _a_token_address_typed(&self) -> Result<Address> {
        Address::from_str(&self.a_token_address).map_err(|e| eyre::eyre!("Invalid address: {}", e))
    }

    pub fn _stable_debt_token_address_typed(&self) -> Result<Address> {
        Address::from_str(&self.stable_debt_token_address)
            .map_err(|e| eyre::eyre!("Invalid address: {}", e))
    }

    pub fn _variable_debt_token_address_typed(&self) -> Result<Address> {
        Address::from_str(&self.variable_debt_token_address)
            .map_err(|e| eyre::eyre!("Invalid address: {}", e))
    }

    pub fn _interest_rate_strategy_address_typed(&self) -> Result<Address> {
        Address::from_str(&self.interest_rate_strategy_address)
            .map_err(|e| eyre::eyre!("Invalid address: {}", e))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Trove {
    pub trove_id: String,
    pub collateral: String,
    pub debt: String,
    pub icr: String,
    pub icr_numeric: f64,
    pub interest_rate: String,
    pub status: String,
    pub last_updated: i64,
}

pub struct DatabaseStore {
    pool: SqlitePool,
}

impl DatabaseStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get_last_block(&self) -> Result<i64> {
        let last_block = sqlx::query_scalar::<_, i64>("SELECT block_number FROM last_block")
            .fetch_optional(&self.pool)
            .await?;
        Ok(last_block.unwrap_or(0))
    }

    pub async fn set_last_block(&self, block_number: i64) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO last_block (id, block_number) VALUES (1, ?)
            ON CONFLICT(id) DO UPDATE SET block_number = excluded.block_number
            "#,
        )
        .bind(block_number)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ========== Troves Table Methods ==========

    pub async fn get_all_active_troves(&self) -> Result<Vec<Trove>> {
        let troves = sqlx::query_as::<_, Trove>("SELECT * FROM troves WHERE status = 'active'  ORDER BY icr_numeric ASC LIMIT 50")
            .fetch_all(&self.pool)
            .await?;
        Ok(troves)
    }

    pub async fn _get_trove_by_id(&self, trove_id: &str) -> Result<Option<Trove>> {
        let trove = sqlx::query_as::<_, Trove>("SELECT * FROM troves WHERE trove_id = ?")
            .bind(trove_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(trove)
    }


     pub async fn upsert_trove(&self, trove: &Trove) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO troves (trove_id, collateral, debt, icr,interest_rate, icr_numeric, status, last_updated)
            VALUES (?, ?, ?, ?, ?, ? , ? , ?)
            ON CONFLICT(trove_id) DO UPDATE SET
                collateral = excluded.collateral,
                debt = excluded.debt,
                icr = excluded.icr,
                interest_rate = interest_rate,
                icr_numeric = excluded.icr_numeric,
                status = excluded.status,
                last_updated = excluded.last_updated
            "#,
        )
        .bind(&trove.trove_id)
        .bind(&trove.collateral)
        .bind(&trove.debt)
        .bind(&trove.icr)
        .bind(&trove.interest_rate)
        .bind(trove.icr_numeric)
        .bind(&trove.status)
        .bind(trove.last_updated)
        .execute(&self.pool)
        .await?;
        Ok(())
     }

     pub async fn close_troves(&self, trove_ids: &Vec<Uint::<256, 4>>) -> Result<(), sqlx::Error> {
        if trove_ids.is_empty() {
            return Ok(());
        }
    
        // dynamically create placeholders: ?,?,? for SQLite/MySQL
        let placeholders = std::iter::repeat("?")
            .take(trove_ids.len())
            .collect::<Vec<_>>()
            .join(", ");
    
        let query = format!(
            r#"
            UPDATE troves
            SET status = 'closed', last_updated = ?
            WHERE trove_id IN ({})
            AND status = 'active'
            "#,
            placeholders
        );
    
        let mut q = sqlx::query(&query);
    
        // Optional: update last_updated time
        let now = Utc::now();
        q = q.bind(now);
    
        for id in trove_ids {
            // Convert to hex string
            let id_str = format!("0x{:x}", id);
            q = q.bind(id_str);
        }
    
        q.execute(&self.pool).await?;
    
        Ok(())
    }
    

    pub async fn _delete_trove(&self, trove_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM troves WHERE trove_id = ?")
            .bind(trove_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }



    // ========== User Collateral Methods ==========

    /// Fetch all collateral for a specific user
   

    // ========== Utility Methods ==========

    /// Get all users with collateral or debt
    /// Get users with both collateral and debt (potential liquidation candidates)
    pub async fn _get_users_with_positions(&self) -> Result<Vec<String>> {
        let users = sqlx::query(
            r#"
            SELECT DISTINCT c.user_address
            FROM user_collateral c
            INNER JOIN user_debt d ON c.user_address = d.user_address
            ORDER BY c.user_address
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let user_addresses: Vec<String> =
            users.into_iter().map(|row| row.get::<String, _>("user_address")).collect();

        Ok(user_addresses)
    }
}

/// Struct for inserting new asset configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetConfigInsert {
    pub asset_address: String,
    pub symbol: String,
    pub decimals: i32,
    pub ltv: i32,
    pub liquidation_threshold: i32,
    pub liquidation_bonus: i32,
    pub reserve_factor: i32,
    pub is_active: bool,
    pub is_frozen: bool,
    pub borrowing_enabled: bool,
    pub stable_borrow_rate_enabled: bool,
    pub a_token_address: String,
    pub stable_debt_token_address: String,
    pub variable_debt_token_address: String,
    pub interest_rate_strategy_address: String,
}
