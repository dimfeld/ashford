//! Actions API endpoints.
//!
//! Provides:
//! - GET /api/actions - List actions with filtering and pagination
//! - GET /api/actions/:id - Get action detail
//! - POST /api/actions/:id/undo - Queue an undo action

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::{Duration, Utc};
use serde::Serialize;
use serde_json::json;

use ashford_core::decisions::ActionLinkRepository;
use ashford_core::{
    ActionDetail, ActionLinkRelationType, ActionListFilter, ActionListItem, ActionRepository,
    ActionStatus, DEFAULT_ORG_ID, DEFAULT_USER_ID, JOB_TYPE_ACTION_GMAIL, JobQueue, NewAction,
    NewActionLink, PaginatedResponse, UndoActionResponse,
};

use crate::AppState;

/// Create the actions API router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_actions))
        .route("/{id}", get(get_action))
        .route("/{id}/undo", post(undo_action))
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

    fn not_found(message: impl Into<String>) -> Self {
        Self::new("not_found", message)
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self::new("bad_request", message)
    }

    fn internal(message: impl Into<String>) -> Self {
        Self::new("internal_error", message)
    }
}

/// Parse time window string to a DateTime cutoff.
///
/// Supported formats:
/// - "24h" - 24 hours ago
/// - "7d" - 7 days ago
/// - "30d" - 30 days ago
/// - None or empty - no time filter
fn parse_time_window(time_window: Option<&str>) -> Option<chrono::DateTime<Utc>> {
    let tw = time_window?;
    if tw.is_empty() {
        return None;
    }

    let duration = if tw.ends_with('h') {
        let hours: i64 = tw.trim_end_matches('h').parse().ok()?;
        Duration::hours(hours)
    } else if tw.ends_with('d') {
        let days: i64 = tw.trim_end_matches('d').parse().ok()?;
        Duration::days(days)
    } else {
        return None;
    };

    Some(Utc::now() - duration)
}

/// Parse comma-separated action types.
fn parse_action_types(action_type: Option<&str>) -> Option<Vec<String>> {
    let at = action_type?;
    if at.is_empty() {
        return None;
    }
    Some(at.split(',').map(|s| s.trim().to_string()).collect())
}

/// Parse comma-separated statuses.
fn parse_statuses(status: Option<&str>) -> Option<Vec<ActionStatus>> {
    let s = status?;
    if s.is_empty() {
        return None;
    }
    let statuses: Vec<ActionStatus> = s
        .split(',')
        .filter_map(|s| ActionStatus::from_str(s.trim()))
        .collect();

    if statuses.is_empty() {
        None
    } else {
        Some(statuses)
    }
}

/// GET /api/actions
///
/// List actions with filtering and pagination.
///
/// Query parameters:
/// - time_window: "24h", "7d", "30d", or empty for all
/// - account_id: Filter by account
/// - sender: Filter by sender email or domain
/// - action_type: Comma-separated action types
/// - status: Comma-separated statuses
/// - min_confidence: Minimum confidence (0-100)
/// - max_confidence: Maximum confidence (0-100)
/// - limit: Items per page (default 20, max 100)
/// - offset: Pagination offset
async fn list_actions(
    State(state): State<AppState>,
    Query(filter): Query<ActionListFilter>,
) -> impl IntoResponse {
    let repo = ActionRepository::new(state.db.clone());

    // Parse filters
    let min_created_at = parse_time_window(filter.time_window.as_deref());
    let action_types = parse_action_types(filter.action_type.as_deref());
    let statuses = parse_statuses(filter.status.as_deref());

    // Convert confidence from 0-100 to 0-1 scale
    let min_confidence = filter.min_confidence.map(|c| c / 100.0);
    let max_confidence = filter.max_confidence.map(|c| c / 100.0);

    // Pagination with defaults and limits
    let limit = filter.limit.unwrap_or(20).clamp(1, 100);
    let offset = filter.offset.unwrap_or(0).max(0);

    let result = repo
        .list_filtered(
            DEFAULT_ORG_ID,
            DEFAULT_USER_ID,
            min_created_at,
            filter.account_id.as_deref(),
            filter.sender.as_deref(),
            action_types.as_deref(),
            statuses.as_deref(),
            min_confidence,
            max_confidence,
            limit,
            offset,
        )
        .await;

    match result {
        Ok((rows, total)) => {
            let items: Vec<ActionListItem> = rows
                .into_iter()
                .map(|row| {
                    // Compute can_undo before moving fields
                    let can_undo = row.can_undo();
                    ActionListItem {
                        id: row.id,
                        account_id: row.account_id,
                        action_type: row.action_type,
                        status: row.status,
                        confidence: row.confidence,
                        created_at: row.created_at,
                        executed_at: row.executed_at,
                        message_subject: row.message_subject,
                        message_from_email: row.message_from_email,
                        message_from_name: row.message_from_name,
                        can_undo,
                    }
                })
                .collect();

            let response = PaginatedResponse::new(items, total, limit, offset);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to list actions: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal(format!("Failed to list actions: {}", e))),
            )
                .into_response()
        }
    }
}

/// GET /api/actions/:id
///
/// Get detailed information about a specific action.
async fn get_action(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let repo = ActionRepository::new(state.db.clone());

    match repo.get_detail(DEFAULT_ORG_ID, DEFAULT_USER_ID, &id).await {
        Ok(row) => {
            // Compute derived fields first to avoid partial move issues
            let can_undo = row.can_undo();
            let gmail_link = row.gmail_link();
            let has_been_undone = row.has_been_undone();

            let detail = ActionDetail {
                id: row.action.id,
                org_id: row.action.org_id,
                user_id: row.action.user_id,
                account_id: row.action.account_id,
                message_id: row.action.message_id,
                decision_id: row.action.decision_id,
                action_type: row.action.action_type,
                parameters_json: row.action.parameters_json,
                status: row.action.status,
                error_message: row.action.error_message,
                executed_at: row.action.executed_at,
                undo_hint_json: row.action.undo_hint_json,
                trace_id: row.action.trace_id,
                created_at: row.action.created_at,
                updated_at: row.action.updated_at,
                decision: row.decision,
                message_subject: row.message_subject,
                message_from_email: row.message_from_email,
                message_from_name: row.message_from_name,
                message_snippet: row.message_snippet,
                provider_message_id: row.provider_message_id,
                account_email: row.account_email,
                can_undo,
                gmail_link,
                has_been_undone,
                undo_action_id: row.undo_action_id,
            };
            (StatusCode::OK, Json(detail)).into_response()
        }
        Err(ashford_core::ActionError::NotFound(_)) => (
            StatusCode::NOT_FOUND,
            Json(ApiError::not_found(format!("Action not found: {}", id))),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to get action {}: {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal(format!("Failed to get action: {}", e))),
            )
                .into_response()
        }
    }
}

/// POST /api/actions/:id/undo
///
/// Queue an undo action for a completed action.
///
/// Requirements for undo:
/// 1. Action must be in Completed status
/// 2. Action must have undo_hint_json with inverse_action
/// 3. Action must not have already been undone
async fn undo_action(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let action_repo = ActionRepository::new(state.db.clone());
    let link_repo = ActionLinkRepository::new(state.db.clone());
    let queue = JobQueue::new(state.db.clone());

    // Get the action detail to check eligibility
    let row = match action_repo
        .get_detail(DEFAULT_ORG_ID, DEFAULT_USER_ID, &id)
        .await
    {
        Ok(row) => row,
        Err(ashford_core::ActionError::NotFound(_)) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiError::not_found(format!("Action not found: {}", id))),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Failed to get action {}: {}", id, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal(format!("Failed to get action: {}", e))),
            )
                .into_response();
        }
    };

    // Check undo eligibility
    if row.action.status != ActionStatus::Completed {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError::bad_request(format!(
                "Cannot undo action with status: {:?}",
                row.action.status
            ))),
        )
            .into_response();
    }

    if row.has_been_undone() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError::bad_request("Action has already been undone")),
        )
            .into_response();
    }

    // Extract inverse action info from undo_hint_json
    let inverse_action = match row.action.undo_hint_json.get("inverse_action") {
        Some(action_value) => match action_value.as_str().filter(|s| !s.trim().is_empty()) {
            Some(valid) => valid.to_string(),
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiError::bad_request(
                        "Action does not support undo (invalid inverse_action)",
                    )),
                )
                    .into_response();
            }
        },
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiError::bad_request("Action does not support undo")),
            )
                .into_response();
        }
    };

    let inverse_parameters = row
        .action
        .undo_hint_json
        .get("inverse_parameters")
        .cloned()
        .unwrap_or(json!({}));

    // Create the undo action
    let new_undo_action = NewAction {
        org_id: DEFAULT_ORG_ID,
        user_id: DEFAULT_USER_ID,
        account_id: row.action.account_id.clone(),
        message_id: row.action.message_id.clone(),
        decision_id: None, // Undo actions are not from decisions
        action_type: inverse_action.clone(),
        parameters_json: inverse_parameters,
        status: ActionStatus::Queued,
        error_message: None,
        executed_at: None,
        undo_hint_json: json!({}), // Undo of undo not supported
        trace_id: None,
    };

    let undo_action = match action_repo.create(new_undo_action).await {
        Ok(action) => action,
        Err(e) => {
            tracing::error!("Failed to create undo action: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal(format!(
                    "Failed to create undo action: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    // Create the action link (original is the cause, undo_action is the effect)
    // This means "effect_action_id is the undo of cause_action_id"
    let new_link = NewActionLink {
        cause_action_id: id.clone(),
        effect_action_id: undo_action.id.clone(),
        relation_type: ActionLinkRelationType::UndoOf,
    };

    if let Err(e) = link_repo.create(new_link).await {
        tracing::error!("Failed to create action link: {}", e);

        // Best-effort cleanup so the orphaned undo action won't execute
        let _ = action_repo
            .update_status(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                &undo_action.id,
                ActionStatus::Canceled,
                Some("Failed to create undo link".into()),
                None,
            )
            .await;

        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::internal(format!(
                "Failed to create undo link: {}",
                e
            ))),
        )
            .into_response();
    }

    // Enqueue the job to execute the undo action
    let job_payload = json!({
        "account_id": row.action.account_id,
        "action_id": undo_action.id
    });

    if let Err(e) = queue
        .enqueue(JOB_TYPE_ACTION_GMAIL, job_payload, None, 0)
        .await
    {
        tracing::error!("Failed to enqueue undo job: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::internal(format!(
                "Failed to enqueue undo job: {}",
                e
            ))),
        )
            .into_response();
    }

    let response = UndoActionResponse {
        undo_action_id: undo_action.id,
        status: "queued".to_string(),
        message: format!(
            "Undo action queued: {} -> {}",
            row.action.action_type, inverse_action
        ),
    };

    (StatusCode::OK, Json(response)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ashford_core::{
        ActionRepository, ActionStatus, DEFAULT_ORG_ID, DEFAULT_USER_ID, Database, NewAction,
        migrations::run_migrations,
    };
    use axum::body::to_bytes;
    use libsql::params;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn parse_time_window_hours() {
        let now = Utc::now();
        let result = parse_time_window(Some("24h")).unwrap();
        let expected = now - Duration::hours(24);
        // Allow 1 second tolerance
        assert!((result - expected).num_seconds().abs() < 1);
    }

    #[test]
    fn parse_time_window_days() {
        let now = Utc::now();
        let result = parse_time_window(Some("7d")).unwrap();
        let expected = now - Duration::days(7);
        assert!((result - expected).num_seconds().abs() < 1);
    }

    #[test]
    fn parse_time_window_empty() {
        assert!(parse_time_window(None).is_none());
        assert!(parse_time_window(Some("")).is_none());
    }

    #[test]
    fn parse_time_window_invalid() {
        assert!(parse_time_window(Some("invalid")).is_none());
        assert!(parse_time_window(Some("24")).is_none());
    }

    #[test]
    fn parse_action_types_single() {
        let result = parse_action_types(Some("archive")).unwrap();
        assert_eq!(result, vec!["archive"]);
    }

    #[test]
    fn parse_action_types_multiple() {
        let result = parse_action_types(Some("archive, label, mark_read")).unwrap();
        assert_eq!(result, vec!["archive", "label", "mark_read"]);
    }

    #[test]
    fn parse_action_types_empty() {
        assert!(parse_action_types(None).is_none());
        assert!(parse_action_types(Some("")).is_none());
    }

    #[test]
    fn parse_statuses_single() {
        let result = parse_statuses(Some("completed")).unwrap();
        assert_eq!(result, vec![ActionStatus::Completed]);
    }

    #[test]
    fn parse_statuses_multiple() {
        let result = parse_statuses(Some("queued, completed, failed")).unwrap();
        assert_eq!(
            result,
            vec![
                ActionStatus::Queued,
                ActionStatus::Completed,
                ActionStatus::Failed
            ]
        );
    }

    #[test]
    fn parse_statuses_with_invalid() {
        // Should skip invalid values
        let result = parse_statuses(Some("completed, invalid, queued")).unwrap();
        assert_eq!(result, vec![ActionStatus::Completed, ActionStatus::Queued]);
    }

    async fn setup_db() -> (Database, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        let db_path = dir.path().join("test.sqlite");
        let db = Database::new(&db_path).await.expect("create db");
        run_migrations(&db).await.expect("migrations");
        (db, dir)
    }

    async fn seed_message(db: &Database) -> (String, String) {
        let now = Utc::now().to_rfc3339();
        let account_id = "acc-test".to_string();
        let thread_id = "thr-test".to_string();
        let message_id = "msg-test".to_string();

        let conn = db.connection().await.expect("conn");
        conn.execute(
            "INSERT INTO accounts (id, provider, email, display_name, config_json, state_json, created_at, updated_at, org_id, user_id)
             VALUES (?1, 'gmail', ?2, ?3, '{}', '{}', ?4, ?4, 1, 1)",
            params![account_id.clone(), "user@example.com", "User", now.clone()],
        )
        .await
        .expect("insert account");

        conn.execute(
            "INSERT INTO threads (id, account_id, provider_thread_id, subject, snippet, last_message_at, metadata_json, raw_json, created_at, updated_at, org_id, user_id)
             VALUES (?1, ?2, 'prov-thread', 'Subject', 'Snippet', ?3, '{}', '{}', ?3, ?3, 1, 1)",
            params![thread_id.clone(), account_id.clone(), now.clone()],
        )
        .await
        .expect("insert thread");

        conn.execute(
            "INSERT INTO messages (id, account_id, thread_id, provider_message_id, from_email, from_name, to_json, cc_json, bcc_json, subject, snippet, received_at, internal_date, labels_json, headers_json, body_plain, body_html, raw_json, created_at, updated_at, org_id, user_id)
             VALUES (?1, ?2, ?3, 'prov-msg', 'sender@example.com', 'Sender', '[]', '[]', '[]', 'Subject', 'Snippet', ?4, ?4, '[]', '{}', NULL, NULL, '{}', ?4, ?4, 1, 1)",
            params![message_id.clone(), account_id.clone(), thread_id.clone(), now.clone()],
        )
        .await
        .expect("insert message");

        (account_id, message_id)
    }

    async fn insert_completed_action(
        db: &Database,
        account_id: &str,
        message_id: &str,
        undo_hint_json: serde_json::Value,
    ) -> String {
        let repo = ActionRepository::new(db.clone());
        let action = repo
            .create(NewAction {
                org_id: DEFAULT_ORG_ID,
                user_id: DEFAULT_USER_ID,
                account_id: account_id.to_string(),
                message_id: message_id.to_string(),
                decision_id: None,
                action_type: "archive".to_string(),
                parameters_json: json!({"label": "inbox"}),
                status: ActionStatus::Queued,
                error_message: None,
                executed_at: None,
                undo_hint_json: json!({}),
                trace_id: None,
            })
            .await
            .expect("create action");
        let executing = repo
            .mark_executing(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action.id)
            .await
            .expect("mark executing");
        let completed = repo
            .mark_completed_with_undo_hint(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                &executing.id,
                undo_hint_json,
            )
            .await
            .expect("complete action");
        completed.id
    }

    #[tokio::test]
    async fn undo_action_rejects_non_string_inverse_action() {
        let (db, _dir) = setup_db().await;
        let (account_id, message_id) = seed_message(&db).await;
        let action_id = insert_completed_action(
            &db,
            &account_id,
            &message_id,
            json!({"inverse_action": {"bad": "data"}}),
        )
        .await;

        let state = crate::AppState { db: db.clone() };
        let response = undo_action(State(state), Path(action_id.clone()))
            .await
            .into_response();

        let status = response.status();
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).expect("json body");
        assert_eq!(status, StatusCode::BAD_REQUEST, "body: {}", body);
        assert_eq!(body.get("error"), Some(&json!("bad_request")));
        assert!(
            body["message"]
                .as_str()
                .unwrap_or_default()
                .contains("invalid inverse_action")
        );

        let conn = db.connection().await.expect("conn");
        let mut rows = conn
            .query(
                "SELECT COUNT(*) FROM actions WHERE id != ?1",
                params![action_id],
            )
            .await
            .expect("query actions");
        let count: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
        assert_eq!(count, 0, "no undo action should be created");
    }

    #[tokio::test]
    async fn undo_action_cancels_undo_when_link_insert_fails() {
        let (db, _dir) = setup_db().await;
        let (account_id, message_id) = seed_message(&db).await;
        let action_id = insert_completed_action(
            &db,
            &account_id,
            &message_id,
            json!({"inverse_action": "apply_label"}),
        )
        .await;

        {
            let conn = db.connection().await.expect("conn");
            conn.execute(
                "CREATE TRIGGER fail_action_links_insert
                 BEFORE INSERT ON action_links
                 BEGIN
                   SELECT RAISE(FAIL, 'triggered failure');
                 END;",
                (),
            )
            .await
            .expect("create trigger");
        }

        let state = crate::AppState { db: db.clone() };
        let response = undo_action(State(state), Path(action_id.clone()))
            .await
            .into_response();

        let status = response.status();
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).expect("json body");
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "body: {}", body);
        assert_eq!(body.get("error"), Some(&json!("internal_error")));
        assert!(
            body["message"]
                .as_str()
                .unwrap_or_default()
                .contains("Failed to create undo link")
        );

        let conn = db.connection().await.expect("conn");
        let mut rows = conn
            .query(
                "SELECT id, status FROM actions WHERE id != ?1",
                params![action_id],
            )
            .await
            .expect("query actions");
        let row = rows.next().await.unwrap().expect("undo action row");
        let undo_status: String = row.get(1).expect("status");
        assert_eq!(undo_status, ActionStatus::Canceled.as_str());

        let mut link_rows = conn
            .query("SELECT COUNT(*) FROM action_links", ())
            .await
            .expect("count links");
        let link_count: i64 = link_rows.next().await.unwrap().unwrap().get(0).unwrap();
        assert_eq!(link_count, 0, "link insert should have failed");

        let mut job_rows = conn
            .query("SELECT COUNT(*) FROM jobs", ())
            .await
            .expect("count jobs");
        let job_count: i64 = job_rows.next().await.unwrap().unwrap().get(0).unwrap();
        assert_eq!(job_count, 0, "no jobs should be enqueued when link fails");
    }
}
