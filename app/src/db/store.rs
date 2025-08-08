use alloy::primitives::{Address, Uint};
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

    pub fn scaled_balance_typed(&self) -> Result<Uint<256, 4>> {
        Uint::from_str(&self.scaled_balance).map_err(|e| eyre::eyre!("Invalid uint: {}", e))
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserDebt {
    pub id: i64,
    pub user_address: String,
    pub debt_address: String,
    pub scaled_balance: String,
    pub last_updated: i64,
}

impl UserDebt {
    pub fn _user_address_typed(&self) -> Result<Address> {
        Address::from_str(&self.user_address).map_err(|e| eyre::eyre!("Invalid address: {}", e))
    }

    pub fn _debt_address_typed(&self) -> Result<Address> {
        Address::from_str(&self.debt_address).map_err(|e| eyre::eyre!("Invalid address: {}", e))
    }

    pub fn scaled_balance_typed(&self) -> Result<Uint<256, 4>> {
        Uint::from_str(&self.scaled_balance).map_err(|e| eyre::eyre!("Invalid uint: {}", e))
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
        let troves = sqlx::query_as::<_, Trove>("SELECT * FROM troves WHERE status = 'active'  ORDER BY icr_numeric ASC")
            .fetch_all(&self.pool)
            .await?;
        Ok(troves)
    }

    pub async fn get_trove_by_id(&self, trove_id: &str) -> Result<Option<Trove>> {
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

    pub async fn delete_trove(&self, trove_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM troves WHERE trove_id = ?")
            .bind(trove_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }



    // ========== User Collateral Methods ==========

    /// Fetch all collateral for a specific user
    pub async fn get_user_collateral(&self, user_address: &str) -> Result<Vec<UserCollateral>> {
        let collaterals = sqlx::query_as::<_, UserCollateral>(
            "SELECT * FROM user_collateral WHERE user_address = ? ORDER BY last_updated DESC",
        )
        .bind(user_address)
        .fetch_all(&self.pool)
        .await?;

        Ok(collaterals)
    }

    /// Fetch specific collateral for a user and asset
    pub async fn get_user_collateral_by_asset(
        &self,
        user_address: &str,
        collateral_address: &str,
    ) -> Result<Option<UserCollateral>> {
        let collateral = sqlx::query_as::<_, UserCollateral>(
            "SELECT * FROM user_collateral WHERE user_address = ? AND collateral_address = ?",
        )
        .bind(user_address)
        .bind(collateral_address)
        .fetch_optional(&self.pool)
        .await?;

        Ok(collateral)
    }



    /// Update or insert user collateral (upsert) - preserves existing enabled state
    pub async fn upsert_user_collateral(
        &self,
        user_address: Address,
        collateral_address: Address,
        scaled_balance: Uint<256, 4>,
        current_block: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO user_collateral (user_address, collateral_address, scaled_balance, enabled, last_updated)
            VALUES (?, ?, ?, FALSE, ?)
            ON CONFLICT(user_address, collateral_address) 
            DO UPDATE SET 
                scaled_balance = excluded.scaled_balance,
                last_updated = excluded.last_updated
            "#
        )
        .bind(user_address.to_string())
        .bind(collateral_address.to_string())
        .bind(scaled_balance.to_string())
        .bind(current_block)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update or insert user collateral with specific enabled state (upsert)
    pub async fn upsert_user_collateral_with_enabled(
        &self,
        user_address: Address,
        collateral_address: Address,
        scaled_balance: Uint<256, 4>,
        enabled: bool,
        current_block: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO user_collateral (user_address, collateral_address, scaled_balance, enabled, last_updated)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(user_address, collateral_address) 
            DO UPDATE SET 
                scaled_balance = excluded.scaled_balance,
                enabled = excluded.enabled,
                last_updated = excluded.last_updated
            "#
        )
        .bind(user_address.to_string())
        .bind(collateral_address.to_string())
        .bind(scaled_balance.to_string())
        .bind(enabled)
        .bind(current_block)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete user collateral
    pub async fn delete_user_collateral(
        &self,
        user_address: &str,
        collateral_address: &str,
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM user_collateral WHERE user_address = ? AND collateral_address = ?",
        )
        .bind(user_address)
        .bind(collateral_address)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ========== User Debt Methods ==========

    /// Fetch all debt for a specific user
    pub async fn get_user_debt(&self, user_address: &str) -> Result<Vec<UserDebt>> {
        let debts = sqlx::query_as::<_, UserDebt>(
            "SELECT * FROM user_debt WHERE user_address = ? ORDER BY last_updated DESC",
        )
        .bind(user_address)
        .fetch_all(&self.pool)
        .await?;

        Ok(debts)
    }

    /// Fetch specific debt for a user and asset
    pub async fn get_user_debt_by_asset(
        &self,
        user_address: &str,
        debt_address: &str,
    ) -> Result<Option<UserDebt>> {
        let debt = sqlx::query_as::<_, UserDebt>(
            "SELECT * FROM user_debt WHERE user_address = ? AND debt_address = ?",
        )
        .bind(user_address)
        .bind(debt_address)
        .fetch_optional(&self.pool)
        .await?;

        Ok(debt)
    }

    /// Update or insert user debt (upsert)
    pub async fn upsert_user_debt(
        &self,
        user_address: Address,
        debt_address: Address,
        scaled_balance: Uint<256, 4>,
        current_block: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO user_debt (user_address, debt_address, scaled_balance, last_updated)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(user_address, debt_address) 
            DO UPDATE SET 
                scaled_balance = excluded.scaled_balance,
                last_updated = ?
            "#,
        )
        .bind(user_address.to_string())
        .bind(debt_address.to_string())
        .bind(scaled_balance.to_string())
        .bind(current_block)
        .bind(current_block)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete user debt
    pub async fn delete_user_debt(&self, user_address: &str, debt_address: &str) -> Result<()> {
        sqlx::query("DELETE FROM user_debt WHERE user_address = ? AND debt_address = ?")
            .bind(user_address)
            .bind(debt_address)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ========== Asset Configuration Methods ==========

    /// Fetch all asset configurations
    pub async fn _get_all_asset_configs(&self) -> Result<Vec<AssetConfig>> {
        let configs =
            sqlx::query_as::<_, AssetConfig>("SELECT * FROM asset_config ORDER BY symbol")
                .fetch_all(&self.pool)
                .await?;

        Ok(configs)
    }

    /// Fetch asset configuration by address
    pub async fn _get_asset_config_by_address(
        &self,
        asset_address: &str,
    ) -> Result<Option<AssetConfig>> {
        let config =
            sqlx::query_as::<_, AssetConfig>("SELECT * FROM asset_config WHERE asset_address = ?")
                .bind(asset_address)
                .fetch_optional(&self.pool)
                .await?;

        Ok(config)
    }

    /// Fetch asset configuration by symbol
    pub async fn _get_asset_config_by_symbol(&self, symbol: &str) -> Result<Option<AssetConfig>> {
        let config =
            sqlx::query_as::<_, AssetConfig>("SELECT * FROM asset_config WHERE symbol = ?")
                .bind(symbol)
                .fetch_optional(&self.pool)
                .await?;

        Ok(config)
    }

    /// Insert or update asset configuration
    pub async fn _upsert_asset_config(
        &self,
        config: &AssetConfigInsert,
        current_block: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO asset_config (
                asset_address, symbol, decimals, ltv, liquidation_threshold, 
                liquidation_bonus, reserve_factor, is_active, is_frozen, 
                borrowing_enabled, stable_borrow_rate_enabled, a_token_address, 
                stable_debt_token_address, variable_debt_token_address, 
                interest_rate_strategy_address, last_updated
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(asset_address) 
            DO UPDATE SET 
                symbol = excluded.symbol,
                decimals = excluded.decimals,
                ltv = excluded.ltv,
                liquidation_threshold = excluded.liquidation_threshold,
                liquidation_bonus = excluded.liquidation_bonus,
                reserve_factor = excluded.reserve_factor,
                is_active = excluded.is_active,
                is_frozen = excluded.is_frozen,
                borrowing_enabled = excluded.borrowing_enabled,
                stable_borrow_rate_enabled = excluded.stable_borrow_rate_enabled,
                a_token_address = excluded.a_token_address,
                stable_debt_token_address = excluded.stable_debt_token_address,
                variable_debt_token_address = excluded.variable_debt_token_address,
                interest_rate_strategy_address = excluded.interest_rate_strategy_address,
                last_updated = ?
            "#,
        )
        .bind(&config.asset_address)
        .bind(&config.symbol)
        .bind(config.decimals)
        .bind(config.ltv)
        .bind(config.liquidation_threshold)
        .bind(config.liquidation_bonus)
        .bind(config.reserve_factor)
        .bind(config.is_active)
        .bind(config.is_frozen)
        .bind(config.borrowing_enabled)
        .bind(config.stable_borrow_rate_enabled)
        .bind(&config.a_token_address)
        .bind(&config.stable_debt_token_address)
        .bind(&config.variable_debt_token_address)
        .bind(&config.interest_rate_strategy_address)
        .bind(current_block)
        .bind(current_block)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete asset configuration
    pub async fn _delete_asset_config(&self, asset_address: &str) -> Result<()> {
        sqlx::query("DELETE FROM asset_config WHERE asset_address = ?")
            .bind(asset_address)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ========== Utility Methods ==========

    /// Get all users with collateral or debt
    pub async fn get_all_users(&self) -> Result<Vec<String>> {
        let users = sqlx::query(
            r#"
            SELECT DISTINCT user_address FROM (
                SELECT user_address FROM user_debt where scaled_balance != ?
            ) ORDER BY user_address
            "#,
        )
        .bind(Uint::<256, 4>::ZERO.to_string())
        .fetch_all(&self.pool)
        .await?;

        let user_addresses: Vec<String> =
            users.into_iter().map(|row| row.get::<String, _>("user_address")).collect();

        Ok(user_addresses)
    }

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
