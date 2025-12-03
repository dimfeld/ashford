use chrono::{DateTime, SecondsFormat, Utc};
use libsql::{Row, params};
use std::collections::HashSet;
use thiserror::Error;
use uuid::Uuid;

use crate::db::{Database, DbError};

const LABEL_COLUMNS: &str = "id, account_id, provider_label_id, name, label_type, description, available_to_classifier, message_list_visibility, label_list_visibility, background_color, text_color, created_at, updated_at, org_id, user_id";

/// A Gmail label stored in the local database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub id: String,
    pub account_id: String,
    pub provider_label_id: String,
    pub name: String,
    pub label_type: String,
    pub description: Option<String>,
    pub available_to_classifier: bool,
    pub message_list_visibility: Option<String>,
    pub label_list_visibility: Option<String>,
    pub background_color: Option<String>,
    pub text_color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub org_id: i64,
    pub user_id: i64,
}

/// Data for creating or updating a label.
#[derive(Debug, Clone)]
pub struct NewLabel {
    pub org_id: i64,
    pub user_id: i64,
    pub account_id: String,
    pub provider_label_id: String,
    pub name: String,
    pub label_type: String,
    pub description: Option<String>,
    pub available_to_classifier: bool,
    pub message_list_visibility: Option<String>,
    pub label_list_visibility: Option<String>,
    pub background_color: Option<String>,
    pub text_color: Option<String>,
}

#[derive(Debug, Error)]
pub enum LabelError {
    #[error("database error: {0}")]
    Database(#[from] DbError),
    #[error("sql error: {0}")]
    Sql(#[from] libsql::Error),
    #[error("datetime parse error: {0}")]
    DateTimeParse(#[from] chrono::ParseError),
    #[error("label not found: {0}")]
    NotFound(String),
}

#[derive(Clone)]
pub struct LabelRepository {
    db: Database,
}

impl LabelRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Insert or update a label by (account_id, provider_label_id).
    /// On conflict, updates all fields except id, created_at.
    pub async fn upsert(&self, new_label: NewLabel) -> Result<Label, LabelError> {
        let NewLabel {
            org_id,
            user_id,
            account_id,
            provider_label_id,
            name,
            label_type,
            description,
            available_to_classifier,
            message_list_visibility,
            label_list_visibility,
            background_color,
            text_color,
        } = new_label;

        let id = Uuid::new_v4().to_string();
        let now = now_rfc3339();
        let available_to_classifier_int = if available_to_classifier { 1 } else { 0 };

        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "INSERT INTO labels (
                        id, account_id, provider_label_id, name, label_type, description,
                        available_to_classifier, message_list_visibility, label_list_visibility,
                        background_color, text_color, created_at, updated_at, org_id, user_id
                    )
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12, ?13, ?14)
                    ON CONFLICT(account_id, provider_label_id) DO UPDATE SET
                        name = excluded.name,
                        label_type = excluded.label_type,
                        message_list_visibility = excluded.message_list_visibility,
                        label_list_visibility = excluded.label_list_visibility,
                        background_color = excluded.background_color,
                        text_color = excluded.text_color,
                        updated_at = excluded.updated_at,
                        org_id = excluded.org_id,
                        user_id = excluded.user_id
                    WHERE labels.org_id = excluded.org_id AND labels.user_id = excluded.user_id
                    RETURNING {LABEL_COLUMNS}"
                ),
                params![
                    id,
                    account_id,
                    provider_label_id,
                    name,
                    label_type,
                    description,
                    available_to_classifier_int,
                    message_list_visibility,
                    label_list_visibility,
                    background_color,
                    text_color,
                    now,
                    org_id,
                    user_id
                ],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_label(row),
            None => Err(LabelError::NotFound(
                "upsert failed for provider_label_id".to_string(),
            )),
        }
    }

    /// Get all labels for an account.
    pub async fn get_by_account(
        &self,
        org_id: i64,
        user_id: i64,
        account_id: &str,
    ) -> Result<Vec<Label>, LabelError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {LABEL_COLUMNS} FROM labels
                     WHERE org_id = ?1 AND user_id = ?2 AND account_id = ?3
                     ORDER BY name"
                ),
                params![org_id, user_id, account_id],
            )
            .await?;

        let mut labels = Vec::new();
        while let Some(row) = rows.next().await? {
            labels.push(row_to_label(row)?);
        }
        Ok(labels)
    }

    /// Lookup a label by account_id + provider_label_id.
    pub async fn get_by_provider_id(
        &self,
        org_id: i64,
        user_id: i64,
        account_id: &str,
        provider_label_id: &str,
    ) -> Result<Label, LabelError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {LABEL_COLUMNS} FROM labels
                     WHERE org_id = ?1 AND user_id = ?2 AND account_id = ?3 AND provider_label_id = ?4"
                ),
                params![org_id, user_id, account_id, provider_label_id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_label(row),
            None => Err(LabelError::NotFound(provider_label_id.to_string())),
        }
    }

    /// Get labels where available_to_classifier = true for an account.
    pub async fn get_available_for_classifier(
        &self,
        org_id: i64,
        user_id: i64,
        account_id: &str,
    ) -> Result<Vec<Label>, LabelError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {LABEL_COLUMNS} FROM labels
                     WHERE org_id = ?1 AND user_id = ?2 AND account_id = ?3 AND available_to_classifier = 1
                     ORDER BY name"
                ),
                params![org_id, user_id, account_id],
            )
            .await?;

        let mut labels = Vec::new();
        while let Some(row) = rows.next().await? {
            labels.push(row_to_label(row)?);
        }
        Ok(labels)
    }

    /// Delete all labels for an account that are NOT in the provided provider_label_ids set.
    /// Returns the number of labels deleted.
    pub async fn delete_not_in_provider_ids(
        &self,
        org_id: i64,
        user_id: i64,
        account_id: &str,
        keep_provider_ids: &[&str],
    ) -> Result<u64, LabelError> {
        let conn = self.db.connection().await?;

        if keep_provider_ids.is_empty() {
            // Delete all labels for this account
            let result = conn
                .execute(
                    "DELETE FROM labels WHERE org_id = ?1 AND user_id = ?2 AND account_id = ?3",
                    params![org_id, user_id, account_id],
                )
                .await?;
            return Ok(result);
        }

        // Build a parameterized query with placeholders for the IDs to keep
        let placeholders: Vec<String> = (0..keep_provider_ids.len())
            .map(|i| format!("?{}", i + 4))
            .collect();
        let placeholders_str = placeholders.join(", ");

        let query = format!(
            "DELETE FROM labels
             WHERE org_id = ?1 AND user_id = ?2 AND account_id = ?3
             AND provider_label_id NOT IN ({placeholders_str})"
        );

        // Build params dynamically
        let mut params_vec: Vec<libsql::Value> =
            vec![org_id.into(), user_id.into(), account_id.into()];
        for id in keep_provider_ids {
            params_vec.push((*id).into());
        }

        let result = conn.execute(&query, params_vec).await?;
        Ok(result)
    }

    /// Find provider_label_ids that exist locally but not in the provided API labels.
    /// Returns the local label IDs that have been deleted on the server.
    pub async fn find_deleted_label_ids(
        &self,
        org_id: i64,
        user_id: i64,
        account_id: &str,
        api_provider_ids: &[&str],
    ) -> Result<Vec<String>, LabelError> {
        let local_labels = self.get_by_account(org_id, user_id, account_id).await?;
        let api_ids: HashSet<&str> = api_provider_ids.iter().copied().collect();

        let deleted: Vec<String> = local_labels
            .into_iter()
            .filter(|label| !api_ids.contains(label.provider_label_id.as_str()))
            .map(|label| label.provider_label_id)
            .collect();

        Ok(deleted)
    }

    /// Lookup a label by name (case-insensitive) for an account.
    /// Returns the first match if multiple labels have names that match case-insensitively.
    pub async fn get_by_name(
        &self,
        org_id: i64,
        user_id: i64,
        account_id: &str,
        name: &str,
    ) -> Result<Label, LabelError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {LABEL_COLUMNS} FROM labels
                     WHERE org_id = ?1 AND user_id = ?2 AND account_id = ?3 AND LOWER(name) = LOWER(?4)
                     LIMIT 1"
                ),
                params![org_id, user_id, account_id, name],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_label(row),
            None => Err(LabelError::NotFound(name.to_string())),
        }
    }
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn row_to_label(row: Row) -> Result<Label, LabelError> {
    let available_to_classifier: i64 = row.get(6)?;
    let created_at: String = row.get(11)?;
    let updated_at: String = row.get(12)?;

    Ok(Label {
        id: row.get(0)?,
        account_id: row.get(1)?,
        provider_label_id: row.get(2)?,
        name: row.get(3)?,
        label_type: row.get(4)?,
        description: row.get(5)?,
        available_to_classifier: available_to_classifier == 1,
        message_list_visibility: row.get(7)?,
        label_list_visibility: row.get(8)?,
        background_color: row.get(9)?,
        text_color: row.get(10)?,
        created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at)?.with_timezone(&Utc),
        org_id: row.get(13)?,
        user_id: row.get(14)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::{AccountConfig, AccountRepository, PubsubConfig};
    use crate::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
    use crate::gmail::OAuthTokens;
    use crate::migrations::run_migrations;
    use tempfile::TempDir;

    async fn setup_repo() -> (LabelRepository, Database, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        let db_name = format!("db_{}.sqlite", uuid::Uuid::new_v4());
        let db_path = dir.path().join(db_name);
        let db = Database::new(&db_path).await.expect("create db");
        run_migrations(&db).await.expect("migrations");
        (LabelRepository::new(db.clone()), db, dir)
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

    fn sample_new_label(account_id: &str, provider_label_id: &str, name: &str) -> NewLabel {
        NewLabel {
            org_id: DEFAULT_ORG_ID,
            user_id: DEFAULT_USER_ID,
            account_id: account_id.to_string(),
            provider_label_id: provider_label_id.to_string(),
            name: name.to_string(),
            label_type: "user".to_string(),
            description: None,
            available_to_classifier: true,
            message_list_visibility: Some("show".to_string()),
            label_list_visibility: Some("labelShow".to_string()),
            background_color: None,
            text_color: None,
        }
    }

    #[tokio::test]
    async fn upsert_creates_new_label() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;
        let new_label = sample_new_label(&account_id, "Label_123", "Work");

        let stored = repo.upsert(new_label.clone()).await.expect("upsert");

        assert_eq!(stored.account_id, account_id);
        assert_eq!(stored.provider_label_id, "Label_123");
        assert_eq!(stored.name, "Work");
        assert_eq!(stored.label_type, "user");
        assert!(stored.available_to_classifier);
        assert_eq!(stored.message_list_visibility.as_deref(), Some("show"));
    }

    #[tokio::test]
    async fn upsert_updates_on_conflict() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;
        let new_label = sample_new_label(&account_id, "Label_123", "Work");

        let first = repo.upsert(new_label.clone()).await.expect("first insert");

        // Update the label with new values
        let updated_label = NewLabel {
            name: "Work Updated".to_string(),
            background_color: Some("#ff0000".to_string()),
            ..new_label.clone()
        };

        let updated = repo.upsert(updated_label).await.expect("update");

        assert_eq!(first.id, updated.id, "upsert should keep row id");
        assert_eq!(updated.name, "Work Updated");
        assert_eq!(updated.background_color.as_deref(), Some("#ff0000"));
        assert!(updated.updated_at >= first.updated_at);
    }

    #[tokio::test]
    async fn get_by_account_returns_all_labels() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;

        repo.upsert(sample_new_label(&account_id, "Label_1", "Alpha"))
            .await
            .expect("insert 1");
        repo.upsert(sample_new_label(&account_id, "Label_2", "Beta"))
            .await
            .expect("insert 2");
        repo.upsert(sample_new_label(&account_id, "INBOX", "INBOX"))
            .await
            .expect("insert 3");

        let labels = repo
            .get_by_account(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id)
            .await
            .expect("get_by_account");

        assert_eq!(labels.len(), 3);
        // Should be sorted by name
        assert_eq!(labels[0].name, "Alpha");
        assert_eq!(labels[1].name, "Beta");
        assert_eq!(labels[2].name, "INBOX");
    }

    #[tokio::test]
    async fn get_by_provider_id_fetches_label() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;

        repo.upsert(sample_new_label(&account_id, "Label_123", "Work"))
            .await
            .expect("insert");

        let fetched = repo
            .get_by_provider_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id, "Label_123")
            .await
            .expect("fetch");

        assert_eq!(fetched.provider_label_id, "Label_123");
        assert_eq!(fetched.name, "Work");
    }

    #[tokio::test]
    async fn get_by_provider_id_returns_not_found() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;

        let result = repo
            .get_by_provider_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id, "nonexistent")
            .await;

        assert!(matches!(result, Err(LabelError::NotFound(_))));
    }

    #[tokio::test]
    async fn get_available_for_classifier_filters_correctly() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;

        let available = sample_new_label(&account_id, "Label_1", "Available");
        let not_available = NewLabel {
            available_to_classifier: false,
            ..sample_new_label(&account_id, "Label_2", "Not Available")
        };

        repo.upsert(available).await.expect("insert available");
        repo.upsert(not_available)
            .await
            .expect("insert not available");

        let labels = repo
            .get_available_for_classifier(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id)
            .await
            .expect("get_available_for_classifier");

        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].name, "Available");
    }

    #[tokio::test]
    async fn delete_not_in_provider_ids_removes_deleted_labels() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;

        repo.upsert(sample_new_label(&account_id, "Label_1", "Keep"))
            .await
            .expect("insert 1");
        repo.upsert(sample_new_label(&account_id, "Label_2", "Delete"))
            .await
            .expect("insert 2");
        repo.upsert(sample_new_label(&account_id, "Label_3", "Also Keep"))
            .await
            .expect("insert 3");

        let deleted_count = repo
            .delete_not_in_provider_ids(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                &account_id,
                &["Label_1", "Label_3"],
            )
            .await
            .expect("delete");

        assert_eq!(deleted_count, 1);

        let remaining = repo
            .get_by_account(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id)
            .await
            .expect("get remaining");

        assert_eq!(remaining.len(), 2);
        let names: Vec<&str> = remaining.iter().map(|l| l.name.as_str()).collect();
        assert!(names.contains(&"Keep"));
        assert!(names.contains(&"Also Keep"));
        assert!(!names.contains(&"Delete"));
    }

    #[tokio::test]
    async fn delete_not_in_provider_ids_with_empty_list_deletes_all() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;

        repo.upsert(sample_new_label(&account_id, "Label_1", "One"))
            .await
            .expect("insert 1");
        repo.upsert(sample_new_label(&account_id, "Label_2", "Two"))
            .await
            .expect("insert 2");

        let deleted_count = repo
            .delete_not_in_provider_ids(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id, &[])
            .await
            .expect("delete all");

        assert_eq!(deleted_count, 2);

        let remaining = repo
            .get_by_account(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id)
            .await
            .expect("get remaining");

        assert!(remaining.is_empty());
    }

    #[tokio::test]
    async fn find_deleted_label_ids_identifies_missing_labels() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;

        repo.upsert(sample_new_label(&account_id, "Label_1", "One"))
            .await
            .expect("insert 1");
        repo.upsert(sample_new_label(&account_id, "Label_2", "Two"))
            .await
            .expect("insert 2");
        repo.upsert(sample_new_label(&account_id, "Label_3", "Three"))
            .await
            .expect("insert 3");

        // Simulate API returning only Label_1 and Label_3
        let deleted = repo
            .find_deleted_label_ids(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                &account_id,
                &["Label_1", "Label_3"],
            )
            .await
            .expect("find deleted");

        assert_eq!(deleted.len(), 1);
        assert!(deleted.contains(&"Label_2".to_string()));
    }

    #[tokio::test]
    async fn get_by_name_is_case_insensitive() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;

        repo.upsert(sample_new_label(&account_id, "Label_123", "Work"))
            .await
            .expect("insert");

        // Try different cases
        let fetched_lower = repo
            .get_by_name(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id, "work")
            .await
            .expect("fetch lowercase");
        assert_eq!(fetched_lower.name, "Work");

        let fetched_upper = repo
            .get_by_name(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id, "WORK")
            .await
            .expect("fetch uppercase");
        assert_eq!(fetched_upper.name, "Work");

        let fetched_mixed = repo
            .get_by_name(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id, "WoRk")
            .await
            .expect("fetch mixed case");
        assert_eq!(fetched_mixed.name, "Work");
    }

    #[tokio::test]
    async fn get_by_name_returns_not_found() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;

        let result = repo
            .get_by_name(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id, "nonexistent")
            .await;

        assert!(matches!(result, Err(LabelError::NotFound(_))));
    }

    #[tokio::test]
    async fn label_queries_scope_to_org_and_user() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;

        let label = sample_new_label(&account_id, "Label_123", "Work");
        let stored = repo.upsert(label).await.expect("insert");

        assert_eq!(stored.org_id, DEFAULT_ORG_ID);
        assert_eq!(stored.user_id, DEFAULT_USER_ID);

        // Wrong user should not find the label
        let wrong_user = repo
            .get_by_provider_id(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID + 1,
                &account_id,
                "Label_123",
            )
            .await
            .expect_err("wrong user should not fetch");
        assert!(matches!(wrong_user, LabelError::NotFound(_)));

        // Wrong org should not find the label
        let wrong_org = repo
            .get_by_provider_id(
                DEFAULT_ORG_ID + 1,
                DEFAULT_USER_ID,
                &account_id,
                "Label_123",
            )
            .await
            .expect_err("wrong org should not fetch");
        assert!(matches!(wrong_org, LabelError::NotFound(_)));
    }

    #[tokio::test]
    async fn upsert_preserves_description_and_available_to_classifier() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;

        // Create label with description and available_to_classifier = false
        let new_label = NewLabel {
            description: Some("Important work emails".to_string()),
            available_to_classifier: false,
            ..sample_new_label(&account_id, "Label_123", "Work")
        };

        let first = repo.upsert(new_label).await.expect("first insert");
        assert_eq!(first.description.as_deref(), Some("Important work emails"));
        assert!(!first.available_to_classifier);

        // Upsert with a name change - description and available_to_classifier should be preserved
        // because the ON CONFLICT clause doesn't update those fields
        let updated_label = NewLabel {
            description: None, // New value (but should NOT be applied because of our ON CONFLICT clause)
            available_to_classifier: true, // New value (but should NOT be applied)
            ..sample_new_label(&account_id, "Label_123", "Work Renamed")
        };

        let updated = repo.upsert(updated_label).await.expect("update");

        // Name should be updated
        assert_eq!(updated.name, "Work Renamed");
        // Description and available_to_classifier should be preserved
        assert_eq!(
            updated.description.as_deref(),
            Some("Important work emails")
        );
        assert!(!updated.available_to_classifier);
    }

    #[tokio::test]
    async fn label_with_colors() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;

        let new_label = NewLabel {
            background_color: Some("#ff0000".to_string()),
            text_color: Some("#ffffff".to_string()),
            ..sample_new_label(&account_id, "Label_123", "Colorful")
        };

        let stored = repo.upsert(new_label).await.expect("insert");

        assert_eq!(stored.background_color.as_deref(), Some("#ff0000"));
        assert_eq!(stored.text_color.as_deref(), Some("#ffffff"));
    }

    #[tokio::test]
    async fn system_vs_user_label_types() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;

        let system_label = NewLabel {
            label_type: "system".to_string(),
            ..sample_new_label(&account_id, "INBOX", "INBOX")
        };

        let user_label = sample_new_label(&account_id, "Label_123", "Custom");

        let stored_system = repo.upsert(system_label).await.expect("insert system");
        let stored_user = repo.upsert(user_label).await.expect("insert user");

        assert_eq!(stored_system.label_type, "system");
        assert_eq!(stored_user.label_type, "user");
    }

    #[tokio::test]
    async fn get_by_account_returns_empty_for_no_labels() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;

        let labels = repo
            .get_by_account(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id)
            .await
            .expect("get_by_account");

        assert!(labels.is_empty());
    }

    #[tokio::test]
    async fn get_available_for_classifier_returns_empty_when_all_excluded() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;

        // Create labels that are all excluded from classifier
        let excluded1 = NewLabel {
            available_to_classifier: false,
            ..sample_new_label(&account_id, "Label_1", "Hidden")
        };
        let excluded2 = NewLabel {
            available_to_classifier: false,
            ..sample_new_label(&account_id, "Label_2", "Also Hidden")
        };

        repo.upsert(excluded1).await.expect("insert 1");
        repo.upsert(excluded2).await.expect("insert 2");

        let available = repo
            .get_available_for_classifier(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id)
            .await
            .expect("get_available");

        assert!(available.is_empty());
    }

    #[tokio::test]
    async fn label_with_special_characters_in_name() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;

        // Test various special characters that might appear in label names
        let special_names = [
            ("Label_1", "Work/Projects"),
            ("Label_2", "Family & Friends"),
            ("Label_3", "Priority!!!"),
            ("Label_4", "TODO: Urgent"),
            ("Label_5", "Label with \"quotes\""),
            ("Label_6", "Unicode: cafe"),
        ];

        for (provider_id, name) in special_names {
            let label = sample_new_label(&account_id, provider_id, name);
            let stored = repo.upsert(label).await.expect("insert special");
            assert_eq!(stored.name, name);

            // Verify we can retrieve by name
            let fetched = repo
                .get_by_name(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id, name)
                .await
                .expect("fetch by name");
            assert_eq!(fetched.name, name);
        }

        let all = repo
            .get_by_account(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id)
            .await
            .expect("get all");

        assert_eq!(all.len(), 6);
    }

    #[tokio::test]
    async fn find_deleted_label_ids_with_all_deleted() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;

        repo.upsert(sample_new_label(&account_id, "Label_1", "One"))
            .await
            .expect("insert 1");
        repo.upsert(sample_new_label(&account_id, "Label_2", "Two"))
            .await
            .expect("insert 2");

        // API returns empty list - all labels were deleted
        let deleted = repo
            .find_deleted_label_ids(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id, &[])
            .await
            .expect("find deleted");

        assert_eq!(deleted.len(), 2);
        assert!(deleted.contains(&"Label_1".to_string()));
        assert!(deleted.contains(&"Label_2".to_string()));
    }

    #[tokio::test]
    async fn find_deleted_label_ids_with_none_deleted() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;

        repo.upsert(sample_new_label(&account_id, "Label_1", "One"))
            .await
            .expect("insert 1");
        repo.upsert(sample_new_label(&account_id, "Label_2", "Two"))
            .await
            .expect("insert 2");

        // API returns all existing labels - nothing deleted
        let deleted = repo
            .find_deleted_label_ids(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                &account_id,
                &["Label_1", "Label_2"],
            )
            .await
            .expect("find deleted");

        assert!(deleted.is_empty());
    }

    #[tokio::test]
    async fn find_deleted_label_ids_with_empty_database() {
        let (repo, db, _dir) = setup_repo().await;
        let account_id = seed_account(&db).await;

        // No labels in database, API has some labels
        let deleted = repo
            .find_deleted_label_ids(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                &account_id,
                &["Label_1", "Label_2"],
            )
            .await
            .expect("find deleted");

        // Nothing to delete since database is empty
        assert!(deleted.is_empty());
    }

    #[tokio::test]
    async fn upsert_different_account_same_provider_id() {
        let (repo, db, _dir) = setup_repo().await;

        // Create two accounts
        let account_repo = AccountRepository::new(db.clone());
        let config1 = AccountConfig {
            client_id: "client".into(),
            client_secret: "secret".into(),
            oauth: OAuthTokens {
                access_token: "access".into(),
                refresh_token: "refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            },
            pubsub: PubsubConfig::default(),
        };
        let config2 = config1.clone();

        let account1 = account_repo
            .create(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                "user1@example.com",
                Some("User 1".into()),
                config1,
            )
            .await
            .expect("create account 1")
            .id;

        let account2 = account_repo
            .create(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                "user2@example.com",
                Some("User 2".into()),
                config2,
            )
            .await
            .expect("create account 2")
            .id;

        // Insert same provider_label_id in both accounts
        let label1 = sample_new_label(&account1, "INBOX", "INBOX Account 1");
        let label2 = sample_new_label(&account2, "INBOX", "INBOX Account 2");

        let stored1 = repo.upsert(label1).await.expect("insert account1");
        let stored2 = repo.upsert(label2).await.expect("insert account2");

        // Should be different rows with different IDs
        assert_ne!(stored1.id, stored2.id);
        assert_eq!(stored1.provider_label_id, "INBOX");
        assert_eq!(stored2.provider_label_id, "INBOX");
        assert_eq!(stored1.name, "INBOX Account 1");
        assert_eq!(stored2.name, "INBOX Account 2");
    }

    #[tokio::test]
    async fn delete_not_in_provider_ids_scopes_to_account() {
        let (repo, db, _dir) = setup_repo().await;

        // Create two accounts
        let account_repo = AccountRepository::new(db.clone());
        let config1 = AccountConfig {
            client_id: "client".into(),
            client_secret: "secret".into(),
            oauth: OAuthTokens {
                access_token: "access".into(),
                refresh_token: "refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            },
            pubsub: PubsubConfig::default(),
        };
        let config2 = config1.clone();

        let account1 = account_repo
            .create(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                "user1@example.com",
                Some("User 1".into()),
                config1,
            )
            .await
            .expect("create account 1")
            .id;

        let account2 = account_repo
            .create(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                "user2@example.com",
                Some("User 2".into()),
                config2,
            )
            .await
            .expect("create account 2")
            .id;

        // Add labels to both accounts
        repo.upsert(sample_new_label(&account1, "Label_1", "Account1 Label1"))
            .await
            .expect("insert");
        repo.upsert(sample_new_label(&account1, "Label_2", "Account1 Label2"))
            .await
            .expect("insert");
        repo.upsert(sample_new_label(&account2, "Label_1", "Account2 Label1"))
            .await
            .expect("insert");

        // Delete labels from account1 that aren't in the keep list
        let deleted = repo
            .delete_not_in_provider_ids(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account1, &["Label_1"])
            .await
            .expect("delete");

        assert_eq!(deleted, 1); // Only Label_2 from account1 should be deleted

        // Account2's Label_1 should still exist
        let account2_labels = repo
            .get_by_account(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account2)
            .await
            .expect("get account2 labels");

        assert_eq!(account2_labels.len(), 1);
        assert_eq!(account2_labels[0].name, "Account2 Label1");
    }
}
