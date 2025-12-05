//! Accounts API endpoints.
//!
//! Provides:
//! - GET /api/accounts - List accounts

use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use serde::Serialize;

use ashford_core::{Account, AccountRepository, AccountSummary, DEFAULT_ORG_ID, DEFAULT_USER_ID};

use crate::AppState;

/// Create the accounts API router.
pub fn router() -> Router<AppState> {
    Router::new().route("/", get(list_accounts))
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

/// Convert an Account to an AccountSummary (strips sensitive data).
fn account_to_summary(account: Account) -> AccountSummary {
    AccountSummary {
        id: account.id,
        provider: account.provider,
        email: account.email,
        display_name: account.display_name,
        sync_status: account.state.sync_status,
        created_at: account.created_at,
        updated_at: account.updated_at,
    }
}

/// GET /api/accounts
///
/// List all accounts for the current user.
async fn list_accounts(State(state): State<AppState>) -> impl IntoResponse {
    let repo = AccountRepository::new(state.db.clone());

    match repo.list_all(DEFAULT_ORG_ID, DEFAULT_USER_ID).await {
        Ok(accounts) => {
            let summaries: Vec<AccountSummary> =
                accounts.into_iter().map(account_to_summary).collect();
            (StatusCode::OK, Json(summaries)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to list accounts: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal("Failed to list accounts")),
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ashford_core::SyncStatus;

    #[test]
    fn account_to_summary_converts_correctly() {
        let account = Account {
            id: "acc-123".to_string(),
            provider: "gmail".to_string(),
            email: "test@example.com".to_string(),
            display_name: Some("Test User".to_string()),
            config: ashford_core::AccountConfig {
                client_id: "client_id".into(),
                client_secret: "secret".into(),
                oauth: ashford_core::OAuthTokens {
                    access_token: "access".into(),
                    refresh_token: "refresh".into(),
                    expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
                },
                pubsub: ashford_core::PubsubConfig::default(),
            },
            state: ashford_core::AccountState {
                history_id: Some("12345".to_string()),
                last_sync_at: Some(chrono::Utc::now()),
                sync_status: SyncStatus::Normal,
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            org_id: 1,
            user_id: 1,
        };

        let summary = account_to_summary(account.clone());

        assert_eq!(summary.id, "acc-123");
        assert_eq!(summary.provider, "gmail");
        assert_eq!(summary.email, "test@example.com");
        assert_eq!(summary.display_name, Some("Test User".to_string()));
        assert_eq!(summary.sync_status, SyncStatus::Normal);
        // Config should not be present in summary (sensitive data stripped)
    }

    #[test]
    fn account_to_summary_handles_none_display_name() {
        let account = Account {
            id: "acc-456".to_string(),
            provider: "gmail".to_string(),
            email: "another@example.com".to_string(),
            display_name: None,
            config: ashford_core::AccountConfig {
                client_id: "client_id".into(),
                client_secret: "secret".into(),
                oauth: ashford_core::OAuthTokens {
                    access_token: "access".into(),
                    refresh_token: "refresh".into(),
                    expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
                },
                pubsub: ashford_core::PubsubConfig::default(),
            },
            state: ashford_core::AccountState::default(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            org_id: 1,
            user_id: 1,
        };

        let summary = account_to_summary(account);

        assert_eq!(summary.id, "acc-456");
        assert_eq!(summary.email, "another@example.com");
        assert_eq!(summary.display_name, None);
    }
}
