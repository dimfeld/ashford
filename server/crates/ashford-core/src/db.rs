use std::{env, path::Path, sync::Arc};

use libsql::{Builder, Connection, Database as LibSqlDatabase};
use thiserror::Error;

#[derive(Clone)]
pub struct Database {
    inner: Arc<LibSqlDatabase>,
}

#[derive(Error, Debug)]
pub enum DbError {
    #[error("failed to build database: {0}")]
    Build(libsql::Error),
    #[error("failed to open connection: {0}")]
    Connect(libsql::Error),
    #[error("failed to execute statement: {0}")]
    Statement(libsql::Error),
    #[error("missing required LIBSQL_AUTH_TOKEN for remote database")]
    MissingAuthToken,
}

impl Database {
    pub async fn new(database_path: &Path) -> Result<Self, DbError> {
        let path_str = database_path.to_string_lossy();
        let inner = if is_remote(&path_str) {
            let auth_token = env::var("LIBSQL_AUTH_TOKEN")
                .ok()
                .filter(|token| !token.is_empty())
                .ok_or(DbError::MissingAuthToken)?;

            Builder::new_remote(path_str.to_string(), auth_token)
                .build()
                .await
        } else {
            Builder::new_local(path_str.to_string()).build().await
        }
        .map_err(DbError::Build)?;

        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    pub async fn connection(&self) -> Result<Connection, DbError> {
        let conn = self.inner.connect().map_err(DbError::Connect)?;
        conn.execute("PRAGMA foreign_keys = ON", ())
            .await
            .map_err(DbError::Statement)?;
        Ok(conn)
    }

    pub async fn health_check(&self) -> Result<(), DbError> {
        let conn = self.connection().await?;
        let mut rows = conn
            .query("SELECT 1", ())
            .await
            .map_err(DbError::Statement)?;
        let _ = rows.next().await.map_err(DbError::Statement)?;
        Ok(())
    }

    pub fn raw(&self) -> &LibSqlDatabase {
        self.inner.as_ref()
    }
}

fn is_remote(path: &str) -> bool {
    path.starts_with("libsql://") || path.starts_with("http://") || path.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    #[tokio::test]
    async fn connection_enables_foreign_keys() {
        let dir = TempDir::new().expect("temp dir");
        let db_path = dir.path().join("db.sqlite");

        let db = Database::new(&db_path).await.expect("create db");
        let conn = db.connection().await.expect("open connection");
        let mut rows = conn
            .query("PRAGMA foreign_keys", ())
            .await
            .expect("query pragma");
        let value: i64 = rows
            .next()
            .await
            .expect("row present")
            .expect("row")
            .get(0)
            .expect("get value");
        assert_eq!(value, 1, "foreign_keys pragma should be enabled");
    }

    #[tokio::test]
    async fn health_check_runs_simple_query() {
        let dir = TempDir::new().expect("temp dir");
        let db_path = dir.path().join("db.sqlite");
        let db = Database::new(&db_path).await.expect("create db");

        db.health_check().await.expect("health check passes");
    }

    #[tokio::test]
    async fn remote_missing_auth_token_errors() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        unsafe { env::remove_var("LIBSQL_AUTH_TOKEN") };
        let result = Database::new(Path::new("libsql://example.com/db")).await;
        match result {
            Ok(_) => panic!("remote db should require auth token"),
            Err(DbError::MissingAuthToken) => {}
            Err(other) => panic!("unexpected error: {other}"),
        }
    }
}
