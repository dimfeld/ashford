use chrono::{DateTime, Duration, SecondsFormat, Utc};
use libsql::{Row, params};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::db::{Database, DbError};
use crate::gmail::oauth::{
    DEFAULT_REFRESH_BUFFER, OAuthError, OAuthTokens, TOKEN_ENDPOINT,
    refresh_access_token_with_endpoint,
};

const ACCOUNT_COLUMNS: &str =
    "id, provider, email, display_name, config_json, state_json, created_at, updated_at";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PubsubConfig {
    pub topic: Option<String>,
    pub subscription: Option<String>,
    #[serde(default)]
    pub service_account_json: Option<String>,
}

impl Default for PubsubConfig {
    fn default() -> Self {
        Self {
            topic: None,
            subscription: None,
            service_account_json: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountConfig {
    pub client_id: String,
    pub client_secret: String,
    pub oauth: OAuthTokens,
    #[serde(default)]
    pub pubsub: PubsubConfig,
}

/// Tracks the synchronization status for an account.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    /// Normal operation - history sync is working
    #[default]
    Normal,
    /// History ID became stale (404), needs backfill
    NeedsBackfill,
    /// Backfill job is in progress
    Backfilling,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AccountState {
    pub history_id: Option<String>,
    pub last_sync_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub sync_status: SyncStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub id: String,
    pub provider: String,
    pub email: String,
    pub display_name: Option<String>,
    pub config: AccountConfig,
    pub state: AccountState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum AccountError {
    #[error("database error: {0}")]
    Database(#[from] DbError),
    #[error("sql error: {0}")]
    Sql(#[from] libsql::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("datetime parse error: {0}")]
    DateTimeParse(#[from] chrono::ParseError),
    #[error("account not found: {0}")]
    NotFound(String),
    #[error("optimistic locking conflict for account {0}")]
    Conflict(String),
    #[error("oauth error: {0}")]
    OAuth(#[from] OAuthError),
}

#[derive(Clone)]
pub struct AccountRepository {
    db: Database,
}

impl AccountRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn create(
        &self,
        email: impl Into<String>,
        display_name: Option<String>,
        config: AccountConfig,
    ) -> Result<Account, AccountError> {
        let id = Uuid::new_v4().to_string();
        let now = now_rfc3339();
        let config_json = serde_json::to_string(&config)?;
        let state = AccountState::default();
        let state_json = serde_json::to_string(&state)?;
        let provider = "gmail";

        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "INSERT INTO accounts (id, provider, email, display_name, config_json, state_json, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
                     RETURNING {ACCOUNT_COLUMNS}"
                ),
                params![
                    id,
                    provider,
                    email.into(),
                    display_name,
                    config_json,
                    state_json,
                    now
                ],
            )
            .await?;

        let row = rows
            .next()
            .await?
            .ok_or_else(|| AccountError::NotFound("insert failed".into()))?;
        row_to_account(row)
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Account, AccountError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!("SELECT {ACCOUNT_COLUMNS} FROM accounts WHERE id = ?1"),
                params![id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_account(row),
            None => Err(AccountError::NotFound(id.to_string())),
        }
    }

    pub async fn get_by_email(&self, email: &str) -> Result<Account, AccountError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!("SELECT {ACCOUNT_COLUMNS} FROM accounts WHERE email = ?1"),
                params![email],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_account(row),
            None => Err(AccountError::NotFound(email.to_string())),
        }
    }

    pub async fn update_config(
        &self,
        id: &str,
        config: &AccountConfig,
    ) -> Result<Account, AccountError> {
        self.update_config_with_expected(id, config, None).await
    }

    pub async fn update_state(
        &self,
        id: &str,
        state: &AccountState,
    ) -> Result<Account, AccountError> {
        let now = now_rfc3339();
        let state_json = serde_json::to_string(state)?;
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "UPDATE accounts
                     SET state_json = ?1, updated_at = ?2
                     WHERE id = ?3
                     RETURNING {ACCOUNT_COLUMNS}"
                ),
                params![state_json, now, id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_account(row),
            None => Err(AccountError::NotFound(id.to_string())),
        }
    }

    pub async fn list_all(&self) -> Result<Vec<Account>, AccountError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!("SELECT {ACCOUNT_COLUMNS} FROM accounts ORDER BY created_at"),
                (),
            )
            .await?;

        let mut accounts = Vec::new();
        while let Some(row) = rows.next().await? {
            accounts.push(row_to_account(row)?);
        }
        Ok(accounts)
    }

    pub async fn delete(&self, id: &str) -> Result<(), AccountError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                "DELETE FROM accounts WHERE id = ?1 RETURNING id",
                params![id],
            )
            .await?;

        match rows.next().await? {
            Some(_) => Ok(()),
            None => Err(AccountError::NotFound(id.to_string())),
        }
    }

    pub async fn refresh_tokens_if_needed(
        &self,
        account_id: &str,
        http: &reqwest::Client,
    ) -> Result<Account, AccountError> {
        self.refresh_tokens_if_needed_with_endpoint(
            account_id,
            http,
            DEFAULT_REFRESH_BUFFER,
            TOKEN_ENDPOINT,
        )
        .await
    }

    pub async fn refresh_tokens_if_needed_with_endpoint(
        &self,
        account_id: &str,
        http: &reqwest::Client,
        buffer: Duration,
        endpoint: &str,
    ) -> Result<Account, AccountError> {
        let account = self.get_by_id(account_id).await?;
        self.refresh_tokens_for_account_with_endpoint(account, http, buffer, endpoint)
            .await
    }

    pub async fn refresh_tokens_for_account(
        &self,
        account: Account,
        http: &reqwest::Client,
        buffer: Duration,
    ) -> Result<Account, AccountError> {
        self.refresh_tokens_for_account_with_endpoint(account, http, buffer, TOKEN_ENDPOINT)
            .await
    }

    pub async fn refresh_tokens_for_account_with_endpoint(
        &self,
        account: Account,
        http: &reqwest::Client,
        buffer: Duration,
        endpoint: &str,
    ) -> Result<Account, AccountError> {
        if !account.config.oauth.needs_refresh(Utc::now(), buffer) {
            return Ok(account);
        }

        let refreshed = refresh_access_token_with_endpoint(
            http,
            &account.config.client_id,
            &account.config.client_secret,
            &account.config.oauth,
            endpoint,
        )
        .await?;

        let mut new_config = account.config.clone();
        new_config.oauth = refreshed;

        self.update_config_with_expected(&account.id, &new_config, Some(account.updated_at))
            .await
    }

    async fn update_config_with_expected(
        &self,
        id: &str,
        config: &AccountConfig,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> Result<Account, AccountError> {
        let now = now_rfc3339();
        let config_json = serde_json::to_string(config)?;
        let conn = self.db.connection().await?;

        let mut rows = if let Some(expected) = expected_updated_at {
            let expected_str = to_rfc3339(expected);
            conn.query(
                &format!(
                    "UPDATE accounts
                     SET config_json = ?1, updated_at = ?2
                     WHERE id = ?3 AND updated_at = ?4
                     RETURNING {ACCOUNT_COLUMNS}"
                ),
                params![config_json, now, id, expected_str],
            )
            .await?
        } else {
            conn.query(
                &format!(
                    "UPDATE accounts
                     SET config_json = ?1, updated_at = ?2
                     WHERE id = ?3
                     RETURNING {ACCOUNT_COLUMNS}"
                ),
                params![config_json, now, id],
            )
            .await?
        };

        match rows.next().await? {
            Some(row) => row_to_account(row),
            None => match expected_updated_at {
                Some(_) => Err(AccountError::Conflict(id.to_string())),
                None => Err(AccountError::NotFound(id.to_string())),
            },
        }
    }
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn to_rfc3339(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn row_to_account(row: Row) -> Result<Account, AccountError> {
    let config_json: String = row.get(4)?;
    let state_json: String = row.get(5)?;
    let created_at: String = row.get(6)?;
    let updated_at: String = row.get(7)?;

    Ok(Account {
        id: row.get(0)?,
        provider: row.get(1)?,
        email: row.get(2)?,
        display_name: row.get(3)?,
        config: serde_json::from_str(&config_json)?,
        state: serde_json::from_str(&state_json)?,
        created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at)?.with_timezone(&Utc),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::migrations::run_migrations;
    use tempfile::TempDir;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn setup_repo() -> (AccountRepository, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        // Use a unique database filename to avoid any potential conflicts
        let db_name = format!("db_{}.sqlite", uuid::Uuid::new_v4());
        let db_path = dir.path().join(db_name);
        let db = Database::new(&db_path).await.expect("create db");
        run_migrations(&db).await.expect("migrations");
        (AccountRepository::new(db), dir)
    }

    fn sample_config(expires_in: Duration) -> AccountConfig {
        AccountConfig {
            client_id: "client".into(),
            client_secret: "secret".into(),
            oauth: OAuthTokens {
                access_token: "access".into(),
                refresh_token: "refresh".into(),
                expires_at: Utc::now() + expires_in,
            },
            pubsub: PubsubConfig {
                topic: Some("projects/example/topics/gmail".into()),
                subscription: Some("projects/example/subscriptions/gmail".into()),
                service_account_json: None,
            },
        }
    }

    #[tokio::test]
    async fn create_and_lookup_account() {
        let (repo, _dir) = setup_repo().await;
        let config = sample_config(Duration::hours(1));

        let account = repo
            .create("user@example.com", Some("User".into()), config.clone())
            .await
            .expect("create account");

        assert_eq!(account.email, "user@example.com");
        assert_eq!(account.provider, "gmail");
        assert_eq!(account.config, config);
        assert!(account.state.history_id.is_none());

        let by_id = repo.get_by_id(&account.id).await.expect("get by id");
        assert_eq!(by_id, account);

        let by_email = repo
            .get_by_email("user@example.com")
            .await
            .expect("get by email");
        assert_eq!(by_email.id, account.id);

        let listed = repo.list_all().await.expect("list accounts");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, account.id);
    }

    #[tokio::test]
    async fn update_config_and_state() {
        let (repo, _dir) = setup_repo().await;
        let config = sample_config(Duration::hours(1));
        let account = repo
            .create("user@example.com", None, config)
            .await
            .expect("create account");

        let new_config = AccountConfig {
            client_id: "client2".into(),
            client_secret: "secret2".into(),
            oauth: OAuthTokens {
                access_token: "new".into(),
                refresh_token: "refresh2".into(),
                expires_at: Utc::now() + Duration::minutes(30),
            },
            pubsub: PubsubConfig::default(),
        };

        let updated = repo
            .update_config(&account.id, &new_config)
            .await
            .expect("update config");
        assert_eq!(updated.config, new_config);
        assert!(updated.updated_at > account.updated_at);

        let new_state = AccountState {
            history_id: Some("123".into()),
            last_sync_at: Some(Utc::now()),
            sync_status: SyncStatus::Normal,
        };

        let state_updated = repo
            .update_state(&account.id, &new_state)
            .await
            .expect("update state");
        assert_eq!(state_updated.state.history_id.as_deref(), Some("123"));
        assert!(state_updated.updated_at > updated.updated_at);
    }

    #[tokio::test]
    async fn delete_removes_account() {
        let (repo, _dir) = setup_repo().await;
        let config = sample_config(Duration::hours(1));
        let account = repo
            .create("user@example.com", None, config)
            .await
            .expect("create account");

        repo.delete(&account.id).await.expect("delete succeeds");
        let err = repo
            .get_by_id(&account.id)
            .await
            .expect_err("should be gone");
        assert!(matches!(err, AccountError::NotFound(_)));
    }

    #[test]
    fn pubsub_config_defaults_without_service_account_json() {
        let json = serde_json::json!({
            "client_id": "client",
            "client_secret": "secret",
            "oauth": {
                "access_token": "access",
                "refresh_token": "refresh",
                "expires_at": "2025-01-01T00:00:00Z"
            },
            "pubsub": {
                "topic": "projects/example/topics/gmail",
                "subscription": "projects/example/subscriptions/gmail"
            }
        })
        .to_string();

        let config: AccountConfig = serde_json::from_str(&json).expect("deserialize config");
        assert_eq!(
            config.pubsub.topic.as_deref(),
            Some("projects/example/topics/gmail")
        );
        assert_eq!(
            config.pubsub.subscription.as_deref(),
            Some("projects/example/subscriptions/gmail")
        );
        assert_eq!(config.pubsub.service_account_json, None);
    }

    #[tokio::test]
    async fn refresh_tokens_skips_when_fresh() {
        let (repo, _dir) = setup_repo().await;
        let config = sample_config(Duration::hours(2));
        let account = repo
            .create("user@example.com", None, config)
            .await
            .expect("create account");

        let client = reqwest::Client::new();
        let refreshed = repo
            .refresh_tokens_if_needed(&account.id, &client)
            .await
            .expect("refresh check");

        assert_eq!(refreshed.config.oauth, account.config.oauth);
    }

    #[tokio::test]
    async fn refresh_tokens_updates_and_persists() {
        let (repo, _dir) = setup_repo().await;
        let config = sample_config(Duration::minutes(1));
        let account = repo
            .create("user@example.com", None, config)
            .await
            .expect("create account");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "new_access",
                "refresh_token": "new_refresh",
                "expires_in": 3600,
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let refreshed = repo
            .refresh_tokens_if_needed_with_endpoint(
                &account.id,
                &client,
                DEFAULT_REFRESH_BUFFER,
                &format!("{}/token", server.uri()),
            )
            .await
            .expect("refresh succeeds");

        assert_eq!(refreshed.config.oauth.access_token, "new_access");
        assert_eq!(refreshed.config.oauth.refresh_token, "new_refresh");
        assert!(refreshed.updated_at > account.updated_at);

        let stored = repo.get_by_id(&account.id).await.expect("reload account");
        assert_eq!(stored.config.oauth.access_token, "new_access");
    }

    #[tokio::test]
    async fn refresh_tokens_respects_optimistic_locking() {
        let (repo, _dir) = setup_repo().await;
        let config = sample_config(Duration::minutes(1));
        let account = repo
            .create("user@example.com", None, config)
            .await
            .expect("create account");

        // Simulate another updater moving updated_at forward before our refresh write.
        let new_state = AccountState {
            history_id: Some("later".into()),
            last_sync_at: None,
            sync_status: SyncStatus::Normal,
        };
        let updated = repo
            .update_state(&account.id, &new_state)
            .await
            .expect("update state");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "new_access",
                "refresh_token": "new_refresh",
                "expires_in": 3600,
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let conflict = repo
            .refresh_tokens_for_account_with_endpoint(
                account,
                &client,
                DEFAULT_REFRESH_BUFFER,
                &format!("{}/token", server.uri()),
            )
            .await
            .expect_err("should conflict");

        assert!(matches!(conflict, AccountError::Conflict(_)));
        let current = repo.get_by_id(&updated.id).await.expect("load latest");
        assert_eq!(
            current.config.oauth.access_token,
            updated.config.oauth.access_token
        );
    }

    #[tokio::test]
    async fn missing_accounts_report_not_found() {
        let (repo, _dir) = setup_repo().await;

        let missing_email = repo
            .get_by_email("absent@example.com")
            .await
            .expect_err("missing email should fail");
        assert!(matches!(missing_email, AccountError::NotFound(_)));

        let missing_delete = repo
            .delete("nonexistent-id")
            .await
            .expect_err("delete missing should fail");
        assert!(matches!(missing_delete, AccountError::NotFound(_)));

        let config = sample_config(Duration::hours(1));
        let missing_update = repo
            .update_config("missing-id", &config)
            .await
            .expect_err("update missing should fail");
        assert!(matches!(missing_update, AccountError::NotFound(_)));
    }

    #[tokio::test]
    async fn service_account_json_roundtrips_through_storage() {
        let (repo, _dir) = setup_repo().await;
        let mut config = sample_config(Duration::hours(1));
        let credentials = r#"{"type":"service_account","client_email":"svc@example.com"}"#;
        config.pubsub.service_account_json = Some(credentials.into());

        let account = repo
            .create("user@example.com", None, config.clone())
            .await
            .expect("create account");

        let stored = repo.get_by_id(&account.id).await.expect("load stored");
        assert_eq!(
            stored.config.pubsub.service_account_json.as_deref(),
            Some(credentials)
        );
        assert_eq!(stored.config.pubsub, config.pubsub);
    }
}
