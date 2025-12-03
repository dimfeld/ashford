//! API types for the web UI.
//!
//! These types are designed for API responses and exclude sensitive information
//! like OAuth tokens and internal configuration details.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::accounts::SyncStatus;

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
