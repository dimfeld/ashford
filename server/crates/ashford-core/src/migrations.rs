use std::collections::HashSet;

use libsql::params;
use thiserror::Error;

use crate::db::{Database, DbError};

struct Migration {
    version: &'static str,
    sql: &'static str,
}

static MIGRATIONS: &[Migration] = &[
    Migration {
        version: "001_initial",
        sql: include_str!("../../../migrations/001_initial.sql"),
    },
    Migration {
        version: "002_add_job_completion_fields",
        sql: include_str!("../../../migrations/002_add_job_completion_fields.sql"),
    },
];

#[derive(Error, Debug)]
pub enum MigrationError {
    #[error("database error: {0}")]
    Database(#[from] DbError),
    #[error("migration failed: {0}")]
    LibSql(#[from] libsql::Error),
}

async fn apply_migrations(
    conn: &libsql::Connection,
    migrations: &[Migration],
) -> Result<(), MigrationError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (version TEXT PRIMARY KEY, applied_at TEXT NOT NULL)",
        (),
    )
    .await?;

    let mut applied = HashSet::new();
    let mut rows = conn
        .query("SELECT version FROM schema_migrations", ())
        .await?;
    while let Some(row) = rows.next().await? {
        let version: String = row.get(0)?;
        applied.insert(version);
    }

    for migration in migrations {
        if applied.contains(migration.version) {
            continue;
        }

        let tx = conn.transaction().await?;
        tx.execute_batch(migration.sql).await?;
        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
            params![migration.version],
        )
        .await?;
        tx.commit().await?;
    }

    Ok(())
}

pub async fn run_migrations(db: &Database) -> Result<(), MigrationError> {
    let conn = db.connection().await?;
    apply_migrations(&conn, MIGRATIONS).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use libsql::{Connection, params};
    use tempfile::TempDir;

    async fn table_exists(conn: &Connection, name: &str) -> bool {
        let mut rows = conn
            .query(
                "SELECT name FROM sqlite_master WHERE type='table' AND name = ?1",
                params![name],
            )
            .await
            .expect("query sqlite_master");
        rows.next().await.expect("row result").is_some()
    }

    #[tokio::test]
    async fn applies_initial_migration_and_records_version() {
        let dir = TempDir::new().expect("temp dir");
        let db_path = dir.path().join("db.sqlite");
        let db = Database::new(&db_path).await.expect("create db");

        run_migrations(&db).await.expect("migrations succeed");

        let conn = db.connection().await.expect("open connection");
        assert!(table_exists(&conn, "accounts").await);
        assert!(table_exists(&conn, "jobs").await);
        assert!(table_exists(&conn, "rules_chat_messages").await);

        let mut rows = conn
            .query(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = '001_initial'",
                (),
            )
            .await
            .expect("query schema_migrations");
        let count: i64 = rows
            .next()
            .await
            .expect("row")
            .expect("row value")
            .get(0)
            .expect("count");
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn rerunning_migrations_is_idempotent() {
        let dir = TempDir::new().expect("temp dir");
        let db_path = dir.path().join("db.sqlite");
        let db = Database::new(&db_path).await.expect("create db");

        run_migrations(&db).await.expect("initial migration");
        run_migrations(&db).await.expect("second migration");

        let conn = db.connection().await.expect("open connection");
        let mut rows = conn
            .query("SELECT COUNT(*) FROM schema_migrations", ())
            .await
            .expect("query count");
        let count: i64 = rows
            .next()
            .await
            .expect("row")
            .expect("row value")
            .get(0)
            .expect("count");
        assert_eq!(count, 2, "migrations should only record once each");
    }

    #[tokio::test]
    async fn applied_at_is_iso_8601_utc() {
        let dir = TempDir::new().expect("temp dir");
        let db_path = dir.path().join("db.sqlite");
        let db = Database::new(&db_path).await.expect("create db");

        run_migrations(&db).await.expect("migrations succeed");

        let conn = db.connection().await.expect("open connection");
        let mut rows = conn
            .query(
                "SELECT applied_at FROM schema_migrations WHERE version = '001_initial'",
                (),
            )
            .await
            .expect("query applied_at");
        let applied_at: String = rows
            .next()
            .await
            .expect("row")
            .expect("row value")
            .get(0)
            .expect("value");
        assert!(
            applied_at.len() >= 20 && applied_at.contains('T') && applied_at.ends_with('Z'),
            "applied_at should be ISO 8601 UTC, got {applied_at}"
        );
    }

    #[tokio::test]
    async fn migration_failure_rolls_back() {
        let dir = TempDir::new().expect("temp dir");
        let db_path = dir.path().join("db.sqlite");
        let db = Database::new(&db_path).await.expect("create db");
        let conn = db.connection().await.expect("open connection");

        let failing_migrations = [Migration {
            version: "002_failure",
            sql: "CREATE TABLE should_not_persist(id INTEGER);\nINVALID SQL STATEMENT;",
        }];

        let err = apply_migrations(&conn, &failing_migrations)
            .await
            .expect_err("migration should fail");
        match err {
            MigrationError::LibSql(_) => {}
            other => panic!("unexpected error: {other}"),
        }

        assert!(
            !table_exists(&conn, "should_not_persist").await,
            "failed migration should roll back schema changes"
        );

        let mut rows = conn
            .query(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = '002_failure'",
                (),
            )
            .await
            .expect("query migrations");
        let count: i64 = rows
            .next()
            .await
            .expect("row")
            .expect("row value")
            .get(0)
            .expect("count");
        assert_eq!(
            count, 0,
            "failed migrations should not be recorded in schema_migrations"
        );
    }
}
