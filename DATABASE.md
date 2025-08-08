# Aave Liquidator Database Module

This module provides SQLite database functionality for storing and managing Aave protocol data for liquidation purposes.

## Database Schema

### Tables

#### 1. `user_collateral`
Stores user collateral positions with scaled balances.

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PRIMARY KEY | Auto-incrementing ID |
| user_address | TEXT | User's Ethereum address |
| collateral_address | TEXT | Collateral asset address |
| scaled_balance | TEXT | Scaled balance (string for precision) |
| last_updated | DATETIME | Timestamp of last update |

**Unique constraint**: `(user_address, collateral_address)`

#### 2. `user_debt`
Stores user debt positions with scaled balances.

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PRIMARY KEY | Auto-incrementing ID |
| user_address | TEXT | User's Ethereum address |
| debt_address | TEXT | Debt asset address |
| scaled_balance | TEXT | Scaled balance (string for precision) |
| last_updated | DATETIME | Timestamp of last update |

**Unique constraint**: `(user_address, debt_address)`

#### 3. `asset_config`
Stores Aave market asset configurations.

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PRIMARY KEY | Auto-incrementing ID |
| asset_address | TEXT | Asset contract address |
| symbol | TEXT | Asset symbol (e.g., "ETH", "USDC") |
| decimals | INTEGER | Number of decimals |
| ltv | INTEGER | Loan-to-Value ratio (basis points) |
| liquidation_threshold | INTEGER | Liquidation threshold (basis points) |
| liquidation_bonus | INTEGER | Liquidation bonus (basis points) |
| reserve_factor | INTEGER | Reserve factor (basis points) |
| is_active | BOOLEAN | Whether asset is active |
| is_frozen | BOOLEAN | Whether asset is frozen |
| borrowing_enabled | BOOLEAN | Whether borrowing is enabled |
| stable_borrow_rate_enabled | BOOLEAN | Whether stable borrowing is enabled |
| a_token_address | TEXT | aToken contract address |
| stable_debt_token_address | TEXT | Stable debt token address |
| variable_debt_token_address | TEXT | Variable debt token address |
| interest_rate_strategy_address | TEXT | Interest rate strategy address |
| last_updated | DATETIME | Timestamp of last update |

**Unique constraint**: `asset_address`

## Usage

### Initialization

```rust
use db::{initialize_database, DatabaseStore};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize database
    let pool = initialize_database("sqlite:aave_liquidator.db").await?;
    let store = DatabaseStore::new(pool);
    
    // Use the store...
    Ok(())
}
```

### Working with User Collateral

```rust
// Insert or update user collateral
store.upsert_user_collateral(
    "0x742d35Cc6639C0532c0022C5B1B07bf7E8eBb60e", // user address
    "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", // WETH address
    "1000000000000000000" // scaled balance (1 ETH scaled)
).await?;

// Get user's collateral positions
let collaterals = store.get_user_collateral("0x742d35Cc6639C0532c0022C5B1B07bf7E8eBb60e").await?;

// Get specific collateral position
let collateral = store.get_user_collateral_by_asset(
    "0x742d35Cc6639C0532c0022C5B1B07bf7E8eBb60e",
    "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
).await?;
```

### Working with User Debt

```rust
// Insert or update user debt
store.upsert_user_debt(
    "0x742d35Cc6639C0532c0022C5B1B07bf7E8eBb60e", // user address
    "0xA0b86a33E6417c6ce82a6F32C1A2de9C5c47E7E2", // USDC address
    "500000000" // scaled balance (500 USDC scaled)
).await?;

// Get user's debt positions
let debts = store.get_user_debt("0x742d35Cc6639C0532c0022C5B1B07bf7E8eBb60e").await?;
```

### Working with Asset Configuration

```rust
use db::store::AssetConfigInsert;

// Insert asset configuration
let config = AssetConfigInsert {
    asset_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
    symbol: "WETH".to_string(),
    decimals: 18,
    ltv: 8000, // 80%
    liquidation_threshold: 8250, // 82.5%
    liquidation_bonus: 500, // 5%
    reserve_factor: 1000, // 10%
    is_active: true,
    is_frozen: false,
    borrowing_enabled: true,
    stable_borrow_rate_enabled: true,
    a_token_address: "0x...".to_string(),
    stable_debt_token_address: "0x...".to_string(),
    variable_debt_token_address: "0x...".to_string(),
    interest_rate_strategy_address: "0x...".to_string(),
};

store.upsert_asset_config(&config).await?;

// Get asset configuration
let asset_config = store.get_asset_config_by_address("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").await?;
```

### Utility Methods

```rust
// Get all users with positions
let users = store.get_all_users().await?;

// Get users with both collateral and debt (liquidation candidates)
let candidates = store.get_users_with_positions().await?;
```

## Important Notes

### Scaled Balances

The database stores **scaled balances** instead of exact amounts because:

1. **Precision**: Aave balances change every second due to interest accrual
2. **Efficiency**: Scaled balances remain constant until user actions
3. **Accuracy**: Eliminates rounding errors from frequent updates

To convert scaled balance to actual balance:
```
actual_balance = scaled_balance * liquidity_index / RAY
```

Where `RAY = 10^27` is Aave's precision constant.

### Database File Location

The SQLite database file will be created in the current working directory as `aave_liquidator.db`. You can specify a different path in the connection string.

The database file will be automatically created if it doesn't exist, along with all required tables and indices.

### Error Handling

All database operations return `eyre::Result<T>` for consistent error handling throughout the application.

## Running the Example

To see the database module in action, simply run:

```bash
cargo run
```

This will:
1. Initialize the SQLite database with all required tables
2. Insert sample WETH asset configuration
3. Insert sample user collateral and debt data
4. Query and display the stored data
5. Show summary statistics

The example demonstrates the complete workflow for an Aave liquidator database. 