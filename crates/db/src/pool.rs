//! Database connection pooling and WAL mode setup.
//!
//! Initializes a SQLite connection pool via `sqlx` with WAL journal mode
//! for concurrent read/write performance. The pool is the primary async
//! database handle used throughout the application.

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;
use std::str::FromStr;
use tracing::info;

use openrustclaw_core::error::{DatabaseError, Error, Result};

/// Initialize the SQLite connection pool with WAL mode.
///
/// # Arguments
///
/// * `database_url` - SQLite connection string (e.g. `"sqlite:data/openrustclaw.db"`)
/// * `max_connections` - Maximum number of connections in the pool
///
/// # Errors
///
/// Returns [`Error::Database`] if the connection string is invalid or the
/// database cannot be opened.
pub async fn init_pool(database_url: &str, max_connections: u32) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(database_url)
        .map_err(|e| Error::Database(DatabaseError::Connection(e.to_string())))?
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .create_if_missing(true)
        .busy_timeout(std::time::Duration::from_secs(30));

    let pool = SqlitePoolOptions::new()
        .max_connections(max_connections)
        .connect_with(options)
        .await
        .map_err(|e| Error::Database(DatabaseError::Connection(e.to_string())))?;

    info!(
        "SQLite pool initialized (WAL mode, max_connections={})",
        max_connections
    );
    Ok(pool)
}
