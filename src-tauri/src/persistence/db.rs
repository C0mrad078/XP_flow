use std::path::Path;
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("failed to open database at {path}: {source}")]
    Connect {
        path: String,
        #[source]
        source: sqlx::Error,
    },

    #[error("migration failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

/// Opens (creating if necessary) the SQLite database at `path` and applies
/// every pending migration embedded from `../migrations`.
///
/// Startup order (section 12 of the Phase 1 brief):
/// `open database -> check migrations -> apply pending migrations -> start
/// application`. A migration failure is returned as an error rather than
/// swallowed — the caller must refuse to start the app on a broken schema
/// rather than risk silent data corruption.
pub async fn init_pool(path: &Path) -> Result<SqlitePool, DatabaseError> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true)
        // Section 113 quality review ("SQLite contention"): the default
        // rollback-journal mode blocks every reader while a writer holds
        // its transaction (and vice versa), which Phase 3's bulk
        // scheduling transactions make far more likely to actually bite.
        // WAL lets readers and a writer proceed concurrently; a
        // non-zero busy_timeout makes a genuine writer-vs-writer
        // collision retry for a few seconds instead of failing the
        // command immediately with SQLITE_BUSY.
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await
        .map_err(|source| DatabaseError::Connect {
            path: path.display().to_string(),
            source,
        })?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn init_pool_creates_schema() {
        let dir = tempfile_dir();
        let db_path = dir.join("test.db");

        let pool = init_pool(&db_path).await.expect("pool should initialize");

        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'workspaces'",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(tables.len(), 1);
        std::fs::remove_dir_all(dir).ok();
    }

    #[tokio::test]
    async fn running_migrations_twice_is_a_no_op() {
        let dir = tempfile_dir();
        let db_path = dir.join("test.db");

        init_pool(&db_path)
            .await
            .expect("first init should succeed");
        let pool = init_pool(&db_path)
            .await
            .expect("second init should succeed");

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM workspaces")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 0);
        std::fs::remove_dir_all(dir).ok();
    }

    fn tempfile_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("xpflow-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
