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
    Migration {
        version: "003_add_thread_message_unique_indices",
        sql: include_str!("../../../migrations/003_add_thread_message_unique_indices.sql"),
    },
    Migration {
        version: "004_add_org_user_columns",
        sql: include_str!("../../../migrations/004_add_org_user_columns.sql"),
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
    use std::collections::HashMap;
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
        assert_eq!(count, 4, "migrations should only record once each");
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

    fn column_map(
        rows: Vec<(String, bool, Option<String>)>,
    ) -> HashMap<String, (bool, Option<String>)> {
        rows.into_iter()
            .map(|(name, notnull, default)| (name, (notnull, default)))
            .collect()
    }

    async fn load_columns(conn: &Connection, table: &str) -> Vec<(String, bool, Option<String>)> {
        let mut rows = conn
            .query(&format!("PRAGMA table_info({table})"), ())
            .await
            .expect("pragma table_info");
        let mut cols = Vec::new();
        while let Some(row) = rows.next().await.expect("row result") {
            let name: String = row.get(1).expect("name");
            let notnull: i64 = row.get(3).expect("notnull");
            let default: Option<String> = row.get(4).ok();
            cols.push((name, notnull == 1, default));
        }
        cols
    }

    async fn index_exists(conn: &Connection, table: &str, index_name: &str) -> bool {
        let mut rows = conn
            .query(&format!("PRAGMA index_list({table})"), ())
            .await
            .expect("pragma index_list");
        while let Some(row) = rows.next().await.expect("row result") {
            let name: String = row.get(1).expect("name");
            if name == index_name {
                return true;
            }
        }
        false
    }

    #[tokio::test]
    async fn org_user_columns_added_with_expected_constraints_and_indexes() {
        let dir = TempDir::new().expect("temp dir");
        let db_path = dir.path().join("db.sqlite");
        let db = Database::new(&db_path).await.expect("create db");

        run_migrations(&db).await.expect("migrations succeed");

        let conn = db.connection().await.expect("open connection");

        let required = [
            "accounts",
            "threads",
            "messages",
            "decisions",
            "actions",
            "rules_chat_sessions",
            "rules_chat_messages",
        ];

        for table in required {
            let cols = column_map(load_columns(&conn, table).await);
            let (org_notnull, org_default) = cols
                .get("org_id")
                .expect("org_id present on required tables");
            let (user_notnull, user_default) = cols
                .get("user_id")
                .expect("user_id present on required tables");
            assert!(
                *org_notnull && *user_notnull,
                "{table} org_id and user_id should be NOT NULL"
            );
            assert_eq!(
                org_default.as_deref(),
                Some("1"),
                "{table} org_id default 1"
            );
            assert_eq!(
                user_default.as_deref(),
                Some("1"),
                "{table} user_id default 1"
            );
            assert!(
                index_exists(&conn, table, &format!("{table}_org_user_idx")).await,
                "{table} should have org/user composite index"
            );
        }

        let nullable_user = ["deterministic_rules", "llm_rules", "directions"];
        for table in nullable_user {
            let cols = column_map(load_columns(&conn, table).await);
            let (org_notnull, org_default) = cols
                .get("org_id")
                .expect("org_id present on nullable tables");
            let (user_notnull, _user_default) = cols
                .get("user_id")
                .expect("user_id present on nullable tables");
            assert!(*org_notnull, "{table} org_id should be NOT NULL");
            assert_eq!(
                org_default.as_deref(),
                Some("1"),
                "{table} org_id default 1"
            );
            assert!(!*user_notnull, "{table} user_id should be nullable");
            assert!(
                index_exists(&conn, table, &format!("{table}_org_user_idx")).await,
                "{table} should have org/user composite index"
            );
        }
    }

    #[tokio::test]
    async fn org_user_migration_backfills_existing_rows() {
        let dir = TempDir::new().expect("temp dir");
        let db_path = dir.path().join("db.sqlite");
        let db = Database::new(&db_path).await.expect("create db");
        let conn = db.connection().await.expect("open connection");

        apply_migrations(&conn, &MIGRATIONS[..3])
            .await
            .expect("initial migrations");

        conn.execute(
            "INSERT INTO accounts (id, provider, email, display_name, config_json, state_json, created_at, updated_at)
             VALUES (?1, 'gmail', 'one@example.com', 'One', '{}', '{}', '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z')",
            params!["acc1"],
        )
        .await
        .expect("insert account");

        conn.execute(
            "INSERT INTO deterministic_rules (id, name, description, scope, scope_ref, priority, enabled, conditions_json, action_type, action_parameters_json, safe_mode, created_at, updated_at)
             VALUES (?1, 'Rule 1', 'desc', 'global', NULL, 100, 1, '{}', 'move', '{}', 'default', '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z')",
            params!["rule1"],
        )
        .await
        .expect("insert deterministic rule");

        apply_migrations(&conn, &MIGRATIONS[3..])
            .await
            .expect("apply org/user migration");

        let mut rows = conn
            .query(
                "SELECT org_id, user_id FROM accounts WHERE id = ?1",
                params!["acc1"],
            )
            .await
            .expect("query accounts");
        let account_row = rows.next().await.expect("row result").expect("account row");
        let account_org: i64 = account_row.get(0).expect("org_id");
        let account_user: i64 = account_row.get(1).expect("user_id");
        assert_eq!(account_org, 1, "existing accounts should backfill org_id=1");
        assert_eq!(
            account_user, 1,
            "existing accounts should backfill user_id=1"
        );

        let mut rows = conn
            .query(
                "SELECT org_id, user_id FROM deterministic_rules WHERE id = ?1",
                params!["rule1"],
            )
            .await
            .expect("query deterministic_rules");
        let rule_row = rows.next().await.expect("row result").expect("rule row");
        let rule_org: i64 = rule_row.get(0).expect("org_id");
        let rule_user: Option<i64> = rule_row.get(1).expect("user_id");
        assert_eq!(rule_org, 1, "org-wide tables should backfill org_id=1");
        assert!(
            rule_user.is_none(),
            "nullable user_id tables should leave existing rows NULL"
        );
    }
}
