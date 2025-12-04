use chrono::{DateTime, SecondsFormat, Utc};
use libsql::{Row, params};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use crate::db::{Database, DbError};

const THREAD_COLUMNS: &str = "id, account_id, provider_thread_id, subject, snippet, last_message_at, metadata_json, raw_json, created_at, updated_at, org_id, user_id";

#[derive(Debug, Clone, PartialEq)]
pub struct Thread {
    pub id: String,
    pub account_id: String,
    pub provider_thread_id: String,
    pub subject: Option<String>,
    pub snippet: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub metadata_json: Value,
    pub raw_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub org_id: i64,
    pub user_id: i64,
}

#[derive(Debug, Error)]
pub enum ThreadError {
    #[error("database error: {0}")]
    Database(#[from] DbError),
    #[error("sql error: {0}")]
    Sql(#[from] libsql::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("datetime parse error: {0}")]
    DateTimeParse(#[from] chrono::ParseError),
    #[error("thread not found: {0}")]
    NotFound(String),
}

#[derive(Clone)]
pub struct ThreadRepository {
    db: Database,
}

impl ThreadRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn upsert(
        &self,
        org_id: i64,
        user_id: i64,
        account_id: &str,
        provider_thread_id: &str,
        subject: Option<String>,
        snippet: Option<String>,
        last_message_at: Option<DateTime<Utc>>,
        raw_json: Value,
    ) -> Result<Thread, ThreadError> {
        let id = Uuid::new_v4().to_string();
        let metadata_json = Value::Object(Default::default());
        let now = now_rfc3339();
        let last_message_at_str = last_message_at.map(to_rfc3339);
        let raw_json_str = serde_json::to_string(&raw_json)?;
        let metadata_json_str = serde_json::to_string(&metadata_json)?;

        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "INSERT INTO threads (id, account_id, provider_thread_id, subject, snippet, last_message_at, metadata_json, raw_json, created_at, updated_at, org_id, user_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9, ?10, ?11)
                     ON CONFLICT(account_id, provider_thread_id) DO UPDATE SET
                        subject = excluded.subject,
                        snippet = excluded.snippet,
                        last_message_at = CASE
                            WHEN threads.last_message_at IS NULL THEN excluded.last_message_at
                            WHEN excluded.last_message_at IS NULL THEN threads.last_message_at
                            WHEN excluded.last_message_at > threads.last_message_at THEN excluded.last_message_at
                            ELSE threads.last_message_at
                        END,
                        metadata_json = threads.metadata_json,
                        raw_json = excluded.raw_json,
                        updated_at = excluded.updated_at,
                        org_id = excluded.org_id,
                        user_id = excluded.user_id
                     WHERE threads.org_id = excluded.org_id AND threads.user_id = excluded.user_id
                     RETURNING {THREAD_COLUMNS}"
                ),
                params![
                    id,
                    account_id,
                    provider_thread_id,
                    subject,
                    snippet,
                    last_message_at_str,
                    metadata_json_str,
                    raw_json_str,
                    now,
                    org_id,
                    user_id
                ],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_thread(row),
            None => Err(ThreadError::NotFound(provider_thread_id.to_string())),
        }
    }

    pub async fn get_by_provider_id(
        &self,
        org_id: i64,
        user_id: i64,
        account_id: &str,
        provider_thread_id: &str,
    ) -> Result<Thread, ThreadError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {THREAD_COLUMNS} FROM threads WHERE org_id = ?1 AND user_id = ?2 AND account_id = ?3 AND provider_thread_id = ?4"
                ),
                params![org_id, user_id, account_id, provider_thread_id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_thread(row),
            None => Err(ThreadError::NotFound(provider_thread_id.to_string())),
        }
    }

    pub async fn get_by_id(
        &self,
        org_id: i64,
        user_id: i64,
        id: &str,
    ) -> Result<Thread, ThreadError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {THREAD_COLUMNS}
                     FROM threads
                     WHERE org_id = ?1 AND user_id = ?2 AND id = ?3"
                ),
                params![org_id, user_id, id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_thread(row),
            None => Err(ThreadError::NotFound(id.to_string())),
        }
    }

    pub async fn update_last_message_at(
        &self,
        org_id: i64,
        user_id: i64,
        thread_id: &str,
        last_message_at: DateTime<Utc>,
    ) -> Result<Thread, ThreadError> {
        let conn = self.db.connection().await?;
        let now = now_rfc3339();
        let last_message_at_str = to_rfc3339(last_message_at);
        let mut rows = conn
            .query(
                &format!(
                    "UPDATE threads
                     SET last_message_at = CASE
                        WHEN last_message_at IS NULL THEN ?1
                        WHEN ?1 > last_message_at THEN ?1
                        ELSE last_message_at
                     END,
                     updated_at = ?2
                     WHERE id = ?3 AND org_id = ?4 AND user_id = ?5
                     RETURNING {THREAD_COLUMNS}"
                ),
                params![last_message_at_str, now, thread_id, org_id, user_id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_thread(row),
            None => Err(ThreadError::NotFound(thread_id.to_string())),
        }
    }
}

fn row_to_thread(row: Row) -> Result<Thread, ThreadError> {
    let last_message_at: Option<String> = row.get(5)?;
    let metadata_json: String = row.get(6)?;
    let raw_json: String = row.get(7)?;
    let created_at: String = row.get(8)?;
    let updated_at: String = row.get(9)?;
    let org_id: i64 = row.get(10)?;
    let user_id: i64 = row.get(11)?;

    Ok(Thread {
        id: row.get(0)?,
        account_id: row.get(1)?,
        provider_thread_id: row.get(2)?,
        subject: row.get(3)?,
        snippet: row.get(4)?,
        last_message_at: match last_message_at {
            Some(value) => Some(DateTime::parse_from_rfc3339(&value)?.with_timezone(&Utc)),
            None => None,
        },
        metadata_json: serde_json::from_str(&metadata_json)?,
        raw_json: serde_json::from_str(&raw_json)?,
        created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at)?.with_timezone(&Utc),
        org_id,
        user_id,
    })
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn to_rfc3339(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::{AccountConfig, AccountRepository, PubsubConfig};
    use crate::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
    use crate::gmail::OAuthTokens;
    use crate::migrations::run_migrations;
    use tempfile::TempDir;

    async fn setup_repo() -> (ThreadRepository, Database, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        // Use a unique database filename to avoid any potential conflicts
        let db_name = format!("db_{}.sqlite", uuid::Uuid::new_v4());
        let db_path = dir.path().join(db_name);
        let db = Database::new(&db_path).await.expect("create db");
        run_migrations(&db).await.expect("migrations");
        (ThreadRepository::new(db.clone()), db, dir)
    }

    async fn seed_account(db: &Database) -> String {
        let repo = AccountRepository::new(db.clone());
        let config = AccountConfig {
            client_id: "client".into(),
            client_secret: "secret".into(),
            oauth: OAuthTokens {
                access_token: "access".into(),
                refresh_token: "refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            },
            pubsub: PubsubConfig::default(),
        };

        repo.create(
            DEFAULT_ORG_ID,
            DEFAULT_USER_ID,
            "user@example.com",
            Some("User".into()),
            config,
        )
        .await
        .expect("create account")
        .id
    }

    #[tokio::test]
    async fn upsert_creates_new_thread() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;
        let result = repo
            .upsert(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                &account_id,
                "thread1",
                Some("Subject".into()),
                Some("Snippet".into()),
                Some(Utc::now()),
                serde_json::json!({"raw": true}),
            )
            .await
            .expect("upsert");

        assert_eq!(result.account_id, account_id);
        assert_eq!(result.provider_thread_id, "thread1");
        assert_eq!(result.subject.as_deref(), Some("Subject"));
        assert_eq!(result.snippet.as_deref(), Some("Snippet"));
        assert!(result.last_message_at.is_some());
        assert_eq!(result.raw_json, serde_json::json!({"raw": true}));
    }

    #[tokio::test]
    async fn upsert_updates_existing_thread_and_keeps_latest_timestamp() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;
        let earlier = Utc::now() - chrono::Duration::hours(1);
        let later = Utc::now();

        let first = repo
            .upsert(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                &account_id,
                "thread1",
                Some("Subject".into()),
                Some("Snippet".into()),
                Some(earlier),
                serde_json::json!({"seq": 1}),
            )
            .await
            .expect("first insert");

        let updated = repo
            .upsert(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                &account_id,
                "thread1",
                Some("New Subject".into()),
                Some("New Snippet".into()),
                Some(later),
                serde_json::json!({"seq": 2}),
            )
            .await
            .expect("update");

        assert_eq!(first.id, updated.id, "upsert should not create new row");
        assert_eq!(updated.subject.as_deref(), Some("New Subject"));
        assert_eq!(updated.snippet.as_deref(), Some("New Snippet"));
        assert_eq!(
            updated.last_message_at.map(|dt| dt.timestamp_millis()),
            Some(later.timestamp_millis())
        );
        assert_eq!(updated.raw_json, serde_json::json!({"seq": 2}));
    }

    #[tokio::test]
    async fn get_by_provider_id_returns_thread() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;
        repo.upsert(
            DEFAULT_ORG_ID,
            DEFAULT_USER_ID,
            &account_id,
            "thread1",
            None,
            None,
            None,
            serde_json::json!({}),
        )
        .await
        .expect("insert");

        let fetched = repo
            .get_by_provider_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id, "thread1")
            .await
            .expect("fetch");

        assert_eq!(fetched.provider_thread_id, "thread1");
    }

    #[tokio::test]
    async fn update_last_message_at_only_moves_forward() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;
        let earlier = Utc::now() - chrono::Duration::hours(2);
        let later = Utc::now();

        let thread = repo
            .upsert(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                &account_id,
                "thread1",
                None,
                None,
                Some(later),
                serde_json::json!({}),
            )
            .await
            .expect("insert");

        let unchanged = repo
            .update_last_message_at(DEFAULT_ORG_ID, DEFAULT_USER_ID, &thread.id, earlier)
            .await
            .expect("update earlier");
        assert_eq!(
            unchanged.last_message_at.map(|dt| dt.timestamp_millis()),
            Some(later.timestamp_millis())
        );

        let advanced = repo
            .update_last_message_at(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                &thread.id,
                later + chrono::Duration::minutes(5),
            )
            .await
            .expect("update later");
        assert!(advanced.last_message_at.unwrap() > later);
    }

    #[tokio::test]
    async fn thread_queries_are_scoped_to_org_and_user() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;
        let thread = repo
            .upsert(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                &account_id,
                "thread1",
                Some("Subject".into()),
                None,
                None,
                serde_json::json!({}),
            )
            .await
            .expect("create thread");

        assert_eq!(thread.org_id, DEFAULT_ORG_ID);
        assert_eq!(thread.user_id, DEFAULT_USER_ID);

        let wrong_user = repo
            .get_by_provider_id(DEFAULT_ORG_ID, DEFAULT_USER_ID + 1, &account_id, "thread1")
            .await
            .expect_err("wrong user should not fetch thread");
        assert!(matches!(wrong_user, ThreadError::NotFound(_)));

        let wrong_org = repo
            .get_by_provider_id(DEFAULT_ORG_ID + 1, DEFAULT_USER_ID, &account_id, "thread1")
            .await
            .expect_err("wrong org should not fetch thread");
        assert!(matches!(wrong_org, ThreadError::NotFound(_)));

        let update_wrong_user = repo
            .update_last_message_at(DEFAULT_ORG_ID, DEFAULT_USER_ID + 1, &thread.id, Utc::now())
            .await
            .expect_err("update with wrong user should fail");
        assert!(matches!(update_wrong_user, ThreadError::NotFound(_)));
    }
}
