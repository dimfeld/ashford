//! API types for the web UI.
//!
//! These types are designed for API responses and exclude sensitive information
//! like OAuth tokens and internal configuration details.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::accounts::SyncStatus;
use crate::decisions::{ActionStatus, Decision};

/// Summary of an account for API responses.
/// Excludes sensitive OAuth tokens and configuration details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AccountSummary {
    pub id: String,
    pub provider: String,
    pub email: String,
    pub display_name: Option<String>,
    pub sync_status: SyncStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Color information for a label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LabelColors {
    pub background_color: Option<String>,
    pub text_color: Option<String>,
}

/// Summary of a label for API responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LabelSummary {
    pub id: String,
    pub account_id: String,
    pub provider_label_id: String,
    pub name: String,
    pub label_type: String,
    pub description: Option<String>,
    pub colors: LabelColors,
}

/// Summary of a message for list views.
/// Excludes full body content, headers, and raw JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MessageSummary {
    pub id: String,
    pub account_id: String,
    pub subject: Option<String>,
    pub snippet: Option<String>,
    pub from_email: Option<String>,
    pub from_name: Option<String>,
    pub received_at: Option<DateTime<Utc>>,
    pub labels: Vec<String>,
}

/// Action summary for list views.
/// Includes joined data from messages and decisions for display purposes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ActionListItem {
    pub id: String,
    pub account_id: String,
    pub action_type: String,
    pub status: ActionStatus,
    /// Confidence from the associated decision (0.0 - 1.0)
    pub confidence: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub executed_at: Option<DateTime<Utc>>,
    /// Subject from the associated message
    pub message_subject: Option<String>,
    /// Sender email from the associated message
    pub message_from_email: Option<String>,
    /// Sender name from the associated message
    pub message_from_name: Option<String>,
    /// Whether this action can be undone
    pub can_undo: bool,
}

/// Detailed action information including decision and message data.
/// Used for the action detail view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ActionDetail {
    pub id: String,
    #[ts(type = "number")]
    pub org_id: i64,
    #[ts(type = "number")]
    pub user_id: i64,
    pub account_id: String,
    pub message_id: String,
    pub decision_id: Option<String>,
    pub action_type: String,
    #[ts(type = "Record<string, unknown>")]
    pub parameters_json: Value,
    pub status: ActionStatus,
    pub error_message: Option<String>,
    pub executed_at: Option<DateTime<Utc>>,
    #[ts(type = "Record<string, unknown>")]
    pub undo_hint_json: Value,
    pub trace_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// The associated decision, if any
    pub decision: Option<Decision>,
    /// Subject from the associated message
    pub message_subject: Option<String>,
    /// Sender email from the associated message
    pub message_from_email: Option<String>,
    /// Sender name from the associated message
    pub message_from_name: Option<String>,
    /// Snippet from the associated message
    pub message_snippet: Option<String>,
    /// Provider message ID for Gmail link construction
    pub provider_message_id: Option<String>,
    /// Account email for Gmail link construction
    pub account_email: Option<String>,
    /// Whether this action can be undone
    pub can_undo: bool,
    /// Constructed Gmail deep link to the original message
    pub gmail_link: Option<String>,
    /// Whether this action has already been undone
    pub has_been_undone: bool,
    /// If undone, the ID of the undo action
    pub undo_action_id: Option<String>,
}

/// Response for the undo action endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UndoActionResponse {
    pub undo_action_id: String,
    pub status: String,
    pub message: String,
}

/// Filter parameters for listing actions.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ActionListFilter {
    /// Time window: "24h", "7d", "30d", or omit for all
    pub time_window: Option<String>,
    /// Filter by account ID
    pub account_id: Option<String>,
    /// Filter by sender (email or domain)
    pub sender: Option<String>,
    /// Filter by action types (comma-separated)
    pub action_type: Option<String>,
    /// Filter by statuses (comma-separated)
    pub status: Option<String>,
    /// Minimum confidence (0-100)
    pub min_confidence: Option<f64>,
    /// Maximum confidence (0-100)
    pub max_confidence: Option<f64>,
    /// Number of items per page (default 20, max 100)
    pub limit: Option<i64>,
    /// Offset for pagination
    pub offset: Option<i64>,
}

/// Generic pagination wrapper for API list responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PaginatedResponse<T: TS> {
    pub items: Vec<T>,
    #[ts(type = "number")]
    pub total: i64,
    #[ts(type = "number")]
    pub limit: i64,
    #[ts(type = "number")]
    pub offset: i64,
    pub has_more: bool,
}

impl<T: TS> PaginatedResponse<T> {
    /// Create a new paginated response.
    pub fn new(items: Vec<T>, total: i64, limit: i64, offset: i64) -> Self {
        let has_more = offset + (items.len() as i64) < total;
        Self {
            items,
            total,
            limit,
            offset,
            has_more,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paginated_response_has_more_true() {
        let response: PaginatedResponse<String> =
            PaginatedResponse::new(vec!["a".to_string(), "b".to_string()], 10, 2, 0);
        assert!(response.has_more);
        assert_eq!(response.total, 10);
        assert_eq!(response.limit, 2);
        assert_eq!(response.offset, 0);
    }

    #[test]
    fn paginated_response_has_more_false() {
        let response: PaginatedResponse<String> =
            PaginatedResponse::new(vec!["a".to_string(), "b".to_string()], 2, 10, 0);
        assert!(!response.has_more);
    }

    #[test]
    fn paginated_response_last_page() {
        let response: PaginatedResponse<String> =
            PaginatedResponse::new(vec!["c".to_string()], 3, 1, 2);
        assert!(!response.has_more);
    }
}
