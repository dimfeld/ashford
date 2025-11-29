use chrono::{DateTime, SecondsFormat, Utc};
use libsql::{Row, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use crate::db::{Database, DbError};

const MESSAGE_COLUMNS: &str = "id, account_id, thread_id, provider_message_id, from_email, from_name, to_json, cc_json, bcc_json, subject, snippet, received_at, internal_date, labels_json, headers_json, body_plain, body_html, raw_json, created_at, updated_at";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mailbox {
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub id: String,
    pub account_id: String,
    pub thread_id: String,
    pub provider_message_id: String,
    pub from_email: Option<String>,
    pub from_name: Option<String>,
    pub to: Vec<Mailbox>,
    pub cc: Vec<Mailbox>,
    pub bcc: Vec<Mailbox>,
    pub subject: Option<String>,
    pub snippet: Option<String>,
    pub received_at: Option<DateTime<Utc>>,
    pub internal_date: Option<DateTime<Utc>>,
    pub labels: Vec<String>,
    pub headers: Value,
    pub body_plain: Option<String>,
    pub body_html: Option<String>,
    pub raw_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewMessage {
    pub account_id: String,
    pub thread_id: String,
    pub provider_message_id: String,
    pub from_email: Option<String>,
    pub from_name: Option<String>,
    pub to: Vec<Mailbox>,
    pub cc: Vec<Mailbox>,
    pub bcc: Vec<Mailbox>,
    pub subject: Option<String>,
    pub snippet: Option<String>,
    pub received_at: Option<DateTime<Utc>>,
    pub internal_date: Option<DateTime<Utc>>,
    pub labels: Vec<String>,
    pub headers: Value,
    pub body_plain: Option<String>,
    pub body_html: Option<String>,
    pub raw_json: Value,
}

#[derive(Debug, Error)]
pub enum MessageError {
    #[error("database error: {0}")]
    Database(#[from] DbError),
    #[error("sql error: {0}")]
    Sql(#[from] libsql::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("datetime parse error: {0}")]
    DateTimeParse(#[from] chrono::ParseError),
    #[error("message not found: {0}")]
    NotFound(String),
}

#[derive(Clone)]
pub struct MessageRepository {
    db: Database,
}

impl MessageRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn upsert(&self, new_msg: NewMessage) -> Result<Message, MessageError> {
        let NewMessage {
            account_id,
            thread_id,
            provider_message_id,
            from_email,
            from_name,
            to,
            cc,
            bcc,
            subject,
            snippet,
            received_at,
            internal_date,
            labels,
            headers,
            body_plain,
            body_html,
            raw_json,
        } = new_msg;

        let id = Uuid::new_v4().to_string();
        let now = now_rfc3339();

        let to_json = serde_json::to_string(&to)?;
        let cc_json = serde_json::to_string(&cc)?;
        let bcc_json = serde_json::to_string(&bcc)?;
        let labels_json = serde_json::to_string(&labels)?;
        let headers_json = serde_json::to_string(&headers)?;
        let raw_json = serde_json::to_string(&raw_json)?;
        let received_at_str = received_at.map(to_rfc3339);
        let internal_date_str = internal_date.map(to_rfc3339);
        let provider_message_id_for_error = provider_message_id.clone();

        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "INSERT INTO messages (
                        id, account_id, thread_id, provider_message_id, from_email, from_name, to_json, cc_json, bcc_json, subject, snippet, received_at, internal_date, labels_json, headers_json, body_plain, body_html, raw_json, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?19)
                     ON CONFLICT(account_id, provider_message_id) DO UPDATE SET
                        thread_id = excluded.thread_id,
                        from_email = excluded.from_email,
                        from_name = excluded.from_name,
                        to_json = excluded.to_json,
                        cc_json = excluded.cc_json,
                        bcc_json = excluded.bcc_json,
                        subject = excluded.subject,
                        snippet = excluded.snippet,
                        received_at = excluded.received_at,
                        internal_date = excluded.internal_date,
                        labels_json = excluded.labels_json,
                        headers_json = excluded.headers_json,
                        body_plain = excluded.body_plain,
                        body_html = excluded.body_html,
                        raw_json = excluded.raw_json,
                        updated_at = excluded.updated_at
                     RETURNING {MESSAGE_COLUMNS}"
                ),
                params![
                    id,
                    account_id,
                    thread_id,
                    provider_message_id,
                    from_email,
                    from_name,
                    to_json,
                    cc_json,
                    bcc_json,
                    subject,
                    snippet,
                    received_at_str,
                    internal_date_str,
                    labels_json,
                    headers_json,
                    body_plain,
                    body_html,
                    raw_json,
                    now
                ],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_message(row),
            None => Err(MessageError::NotFound(provider_message_id_for_error)),
        }
    }

    pub async fn get_by_provider_id(
        &self,
        account_id: &str,
        provider_message_id: &str,
    ) -> Result<Message, MessageError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {MESSAGE_COLUMNS} FROM messages WHERE account_id = ?1 AND provider_message_id = ?2"
                ),
                params![account_id, provider_message_id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_message(row),
            None => Err(MessageError::NotFound(provider_message_id.to_string())),
        }
    }

    pub async fn exists(
        &self,
        account_id: &str,
        provider_message_id: &str,
    ) -> Result<bool, MessageError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                "SELECT 1 FROM messages WHERE account_id = ?1 AND provider_message_id = ?2 LIMIT 1",
                params![account_id, provider_message_id],
            )
            .await?;

        Ok(rows.next().await?.is_some())
    }
}

fn row_to_message(row: Row) -> Result<Message, MessageError> {
    let to_json: String = row.get(6)?;
    let cc_json: String = row.get(7)?;
    let bcc_json: String = row.get(8)?;
    let received_at: Option<String> = row.get(11)?;
    let internal_date: Option<String> = row.get(12)?;
    let labels_json: String = row.get(13)?;
    let headers_json: String = row.get(14)?;
    let raw_json: String = row.get(17)?;
    let created_at: String = row.get(18)?;
    let updated_at: String = row.get(19)?;

    Ok(Message {
        id: row.get(0)?,
        account_id: row.get(1)?,
        thread_id: row.get(2)?,
        provider_message_id: row.get(3)?,
        from_email: row.get(4)?,
        from_name: row.get(5)?,
        to: serde_json::from_str(&to_json)?,
        cc: serde_json::from_str(&cc_json)?,
        bcc: serde_json::from_str(&bcc_json)?,
        subject: row.get(9)?,
        snippet: row.get(10)?,
        received_at: match received_at {
            Some(value) => Some(DateTime::parse_from_rfc3339(&value)?.with_timezone(&Utc)),
            None => None,
        },
        internal_date: match internal_date {
            Some(value) => Some(DateTime::parse_from_rfc3339(&value)?.with_timezone(&Utc)),
            None => None,
        },
        labels: serde_json::from_str(&labels_json)?,
        headers: serde_json::from_str(&headers_json)?,
        body_plain: row.get(15)?,
        body_html: row.get(16)?,
        raw_json: serde_json::from_str(&raw_json)?,
        created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at)?.with_timezone(&Utc),
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
    use crate::gmail::OAuthTokens;
    use crate::migrations::run_migrations;
    use crate::threads::ThreadRepository;
    use tempfile::TempDir;

    async fn setup_repo() -> (MessageRepository, Database, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        // Use a unique database filename to avoid any potential conflicts
        let db_name = format!("db_{}.sqlite", uuid::Uuid::new_v4());
        let db_path = dir.path().join(db_name);
        let db = Database::new(&db_path).await.expect("create db");
        run_migrations(&db).await.expect("migrations");
        (MessageRepository::new(db.clone()), db, dir)
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

        repo.create("user@example.com", Some("User".into()), config)
            .await
            .expect("create account")
            .id
    }

    async fn seed_thread(db: &Database, account_id: &str, provider_thread_id: &str) -> String {
        let thread_repo = ThreadRepository::new(db.clone());
        let thread = thread_repo
            .upsert(
                account_id,
                provider_thread_id,
                Some("Subject".into()),
                Some("Snippet".into()),
                Some(Utc::now()),
                serde_json::json!({"raw": true}),
            )
            .await
            .expect("create thread");
        thread.id
    }

    fn sample_new_message(account_id: &str, thread_id: &str) -> NewMessage {
        NewMessage {
            account_id: account_id.to_string(),
            thread_id: thread_id.to_string(),
            provider_message_id: "msg1".into(),
            from_email: Some("alice@example.com".into()),
            from_name: Some("Alice".into()),
            to: vec![Mailbox {
                email: "bob@example.com".into(),
                name: Some("Bob".into()),
            }],
            cc: vec![],
            bcc: vec![],
            subject: Some("Hello".into()),
            snippet: Some("Snippet".into()),
            received_at: Some(Utc::now()),
            internal_date: Some(Utc::now()),
            labels: vec!["INBOX".into()],
            headers: serde_json::json!({"Header": "value"}),
            body_plain: Some("Hi there".into()),
            body_html: Some("<p>Hi there</p>".into()),
            raw_json: serde_json::json!({"raw": true}),
        }
    }

    #[tokio::test]
    async fn upsert_creates_new_message() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "thread1").await;
        let new_msg = sample_new_message(&account_id, &thread_id);
        let stored = repo.upsert(new_msg.clone()).await.expect("upsert");

        assert_eq!(stored.account_id, new_msg.account_id);
        assert_eq!(stored.provider_message_id, new_msg.provider_message_id);
        assert_eq!(stored.from_email.as_deref(), Some("alice@example.com"));
        assert_eq!(stored.to.len(), 1);
        assert_eq!(stored.labels, vec!["INBOX"]);
        assert_eq!(stored.body_plain.as_deref(), Some("Hi there"));
    }

    #[tokio::test]
    async fn upsert_updates_on_conflict() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "thread1").await;
        let mut new_msg = sample_new_message(&account_id, &thread_id);
        let first = repo.upsert(new_msg.clone()).await.expect("first insert");

        new_msg.subject = Some("Updated".into());
        new_msg.labels = vec!["STARRED".into()];
        new_msg.body_plain = Some("Updated body".into());
        new_msg.body_html = None;

        let updated = repo.upsert(new_msg.clone()).await.expect("update");
        assert_eq!(first.id, updated.id, "upsert should keep row");
        assert_eq!(updated.subject.as_deref(), Some("Updated"));
        assert_eq!(updated.labels, vec!["STARRED"]);
        assert_eq!(updated.body_plain.as_deref(), Some("Updated body"));
        assert!(updated.body_html.is_none());
    }

    #[tokio::test]
    async fn get_by_provider_id_fetches_message() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "thread1").await;
        let new_msg = sample_new_message(&account_id, &thread_id);
        repo.upsert(new_msg.clone()).await.expect("insert");

        let fetched = repo
            .get_by_provider_id(&new_msg.account_id, &new_msg.provider_message_id)
            .await
            .expect("fetch");

        assert_eq!(fetched.provider_message_id, new_msg.provider_message_id);
        assert_eq!(fetched.thread_id, new_msg.thread_id);
    }

    #[tokio::test]
    async fn exists_returns_true_when_present() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "thread1").await;
        let new_msg = sample_new_message(&account_id, &thread_id);
        repo.upsert(new_msg.clone()).await.expect("insert");

        let exists = repo
            .exists(&new_msg.account_id, &new_msg.provider_message_id)
            .await
            .expect("exists");
        assert!(exists);

        let missing = repo.exists("account1", "missing").await.expect("exists");
        assert!(!missing);
    }
}
