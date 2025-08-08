use eyre::Result;
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::str::FromStr;

/// Initialize the SQLite database with required tables
pub async fn initialize_database(database_url: &str) -> Result<SqlitePool> {
    // Create connection pool with options to create the database file if it doesn't exist
    let pool = SqlitePool::connect_with(
        SqliteConnectOptions::from_str(database_url)?.create_if_missing(true),
    )
    .await?;

    // Create tables if they don't exist
    create_tables(&pool).await?;

    Ok(pool)
}

/// Create all required tables
async fn create_tables(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS last_block (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            block_number INTEGER NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Troves Table - stores Trove states for users
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS troves (
            trove_id TEXT PRIMARY KEY,
            collateral TEXT NOT NULL,
            debt TEXT NOT NULL,
            interest_rate TEXT NOT NULL,
            icr TEXT NOT NULL,   -- Collateral Ratio as string for precision (e.g., "115.23")
            icr_numeric REAL NOT NULL,  -- Numeric version for sorting (e.g., 115.23)
            status TEXT NOT NULL,  -- e.g., 'active', 'liquidated', 'closed'
            last_updated INTEGER NOT NULL
            
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create indices for better query performance
    create_indices(pool).await?;

    Ok(())
}

/// Create database indices for better performance
async fn create_indices(pool: &SqlitePool) -> Result<()> {
    // Index on owner for troves table
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_troves_trove_id ON troves(trove_id)",
    )
    .execute(pool)
    .await?;

    // Index on status for troves table
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_troves_status ON troves(status)")
        .execute(pool)
        .await?;

    // Index on icr_numeric for quick sorting by risk (lowest first)
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_troves_icr_numeric ON troves(icr_numeric)")
        .execute(pool)
        .await?;

    Ok(())
}
