//! Labels API endpoints.
//!
//! Provides:
//! - GET /api/labels - List all labels across all accounts

use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use serde::Serialize;

use ashford_core::{
    AccountRepository, DEFAULT_ORG_ID, DEFAULT_USER_ID, LabelColors, LabelRepository, LabelSummary,
};

use crate::AppState;

/// Create the labels API router.
pub fn router() -> Router<AppState> {
    Router::new().route("/", get(list_labels))
}

/// Error response for API errors.
#[derive(Debug, Serialize)]
struct ApiError {
    error: String,
    message: String,
}

impl ApiError {
    fn new(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self::new("internal_error", message)
    }
}

/// GET /api/labels
///
/// List all labels across all accounts for the current user.
/// Used by the condition builder to populate the label_present dropdown.
async fn list_labels(State(state): State<AppState>) -> impl IntoResponse {
    let account_repo = AccountRepository::new(state.db.clone());
    let label_repo = LabelRepository::new(state.db.clone());

    // First get all accounts
    let accounts = match account_repo.list_all(DEFAULT_ORG_ID, DEFAULT_USER_ID).await {
        Ok(accounts) => accounts,
        Err(e) => {
            tracing::error!("Failed to list accounts: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal("Failed to list accounts")),
            )
                .into_response();
        }
    };

    // Then get labels for each account
    let mut all_labels: Vec<LabelSummary> = Vec::new();

    for account in accounts {
        match label_repo
            .get_by_account(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account.id)
            .await
        {
            Ok(labels) => {
                for label in labels {
                    all_labels.push(LabelSummary {
                        id: label.id,
                        account_id: label.account_id,
                        provider_label_id: label.provider_label_id,
                        name: label.name,
                        label_type: label.label_type,
                        description: label.description,
                        colors: LabelColors {
                            background_color: label.background_color,
                            text_color: label.text_color,
                        },
                    });
                }
            }
            Err(e) => {
                tracing::error!("Failed to get labels for account {}: {}", account.id, e);
                // Continue with other accounts even if one fails
            }
        }
    }

    (StatusCode::OK, Json(all_labels)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ashford_core::{
        AccountConfig, Database, NewLabel, OAuthTokens, PubsubConfig, migrations::run_migrations,
    };
    use axum::body::to_bytes;
    use chrono::Utc;
    use tempfile::TempDir;

    async fn setup_db() -> (Database, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        let db_path = dir.path().join("test.sqlite");
        let db = Database::new(&db_path).await.expect("create db");
        run_migrations(&db).await.expect("migrations");
        (db, dir)
    }

    async fn create_account(db: &Database) -> String {
        let repo = AccountRepository::new(db.clone());
        let config = AccountConfig {
            client_id: "client_id".into(),
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
    async fn list_labels_returns_empty_when_no_accounts() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        let response = list_labels(State(state)).await.into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let body: Vec<LabelSummary> = serde_json::from_slice(&body_bytes).expect("json body");
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn list_labels_returns_labels_from_account() {
        let (db, _dir) = setup_db().await;
        let account_id = create_account(&db).await;
        let label_repo = LabelRepository::new(db.clone());

        // Create some labels
        label_repo
            .upsert(NewLabel {
                org_id: DEFAULT_ORG_ID,
                user_id: DEFAULT_USER_ID,
                account_id: account_id.clone(),
                provider_label_id: "Label_1".to_string(),
                name: "Work".to_string(),
                label_type: "user".to_string(),
                description: Some("Work emails".to_string()),
                available_to_classifier: true,
                message_list_visibility: Some("show".to_string()),
                label_list_visibility: Some("labelShow".to_string()),
                background_color: Some("#ff0000".to_string()),
                text_color: Some("#ffffff".to_string()),
            })
            .await
            .expect("create label");

        label_repo
            .upsert(NewLabel {
                org_id: DEFAULT_ORG_ID,
                user_id: DEFAULT_USER_ID,
                account_id: account_id.clone(),
                provider_label_id: "Label_2".to_string(),
                name: "Personal".to_string(),
                label_type: "user".to_string(),
                description: None,
                available_to_classifier: false,
                message_list_visibility: None,
                label_list_visibility: None,
                background_color: None,
                text_color: None,
            })
            .await
            .expect("create label");

        let state = crate::AppState { db: db.clone() };
        let response = list_labels(State(state)).await.into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let body: Vec<LabelSummary> = serde_json::from_slice(&body_bytes).expect("json body");

        assert_eq!(body.len(), 2);

        // Labels are sorted by name from the repository
        let personal = body.iter().find(|l| l.name == "Personal").unwrap();
        assert_eq!(personal.provider_label_id, "Label_2");
        assert_eq!(personal.account_id, account_id);
        assert!(personal.colors.background_color.is_none());

        let work = body.iter().find(|l| l.name == "Work").unwrap();
        assert_eq!(work.provider_label_id, "Label_1");
        assert_eq!(work.account_id, account_id);
        assert_eq!(work.colors.background_color.as_deref(), Some("#ff0000"));
        assert_eq!(work.description.as_deref(), Some("Work emails"));
    }

    #[tokio::test]
    async fn list_labels_includes_all_accounts() {
        let (db, _dir) = setup_db().await;
        let account_repo = AccountRepository::new(db.clone());
        let label_repo = LabelRepository::new(db.clone());

        // Create two accounts
        let config = AccountConfig {
            client_id: "client_id".into(),
            client_secret: "secret".into(),
            oauth: OAuthTokens {
                access_token: "access".into(),
                refresh_token: "refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            },
            pubsub: PubsubConfig::default(),
        };

        let account1 = account_repo
            .create(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                "user1@example.com",
                Some("User 1".into()),
                config.clone(),
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
                config,
            )
            .await
            .expect("create account 2")
            .id;

        // Create labels in both accounts
        label_repo
            .upsert(NewLabel {
                org_id: DEFAULT_ORG_ID,
                user_id: DEFAULT_USER_ID,
                account_id: account1.clone(),
                provider_label_id: "Label_A".to_string(),
                name: "Account 1 Label".to_string(),
                label_type: "user".to_string(),
                description: None,
                available_to_classifier: true,
                message_list_visibility: None,
                label_list_visibility: None,
                background_color: None,
                text_color: None,
            })
            .await
            .expect("create label");

        label_repo
            .upsert(NewLabel {
                org_id: DEFAULT_ORG_ID,
                user_id: DEFAULT_USER_ID,
                account_id: account2.clone(),
                provider_label_id: "Label_B".to_string(),
                name: "Account 2 Label".to_string(),
                label_type: "user".to_string(),
                description: None,
                available_to_classifier: true,
                message_list_visibility: None,
                label_list_visibility: None,
                background_color: None,
                text_color: None,
            })
            .await
            .expect("create label");

        let state = crate::AppState { db: db.clone() };
        let response = list_labels(State(state)).await.into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let body: Vec<LabelSummary> = serde_json::from_slice(&body_bytes).expect("json body");

        assert_eq!(body.len(), 2);

        let label_a = body.iter().find(|l| l.account_id == account1).unwrap();
        assert_eq!(label_a.name, "Account 1 Label");

        let label_b = body.iter().find(|l| l.account_id == account2).unwrap();
        assert_eq!(label_b.name, "Account 2 Label");
    }
}
