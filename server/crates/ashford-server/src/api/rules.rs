//! Rules API endpoints.
//!
//! Provides:
//! - GET /api/rules/deterministic - List deterministic rules
//! - GET /api/rules/deterministic/:id - Get a deterministic rule by ID
//! - POST /api/rules/deterministic - Create a deterministic rule
//! - PATCH /api/rules/deterministic/:id - Update a deterministic rule
//! - DELETE /api/rules/deterministic/:id - Delete a deterministic rule
//! - GET /api/rules/llm - List LLM rules
//! - GET /api/rules/llm/:id - Get an LLM rule by ID
//! - POST /api/rules/llm - Create an LLM rule
//! - PATCH /api/rules/llm/:id - Update an LLM rule
//! - DELETE /api/rules/llm/:id - Delete an LLM rule

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use ashford_core::{
    DEFAULT_ORG_ID, DEFAULT_USER_ID, DeterministicRuleError, DeterministicRuleRepository,
    LlmRuleError, LlmRuleRepository, NewDeterministicRule, NewLlmRule, RuleScope, SafeMode,
};

use crate::AppState;

/// Create the rules API router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Deterministic rules
        .route("/deterministic", get(list_deterministic_rules))
        .route("/deterministic", post(create_deterministic_rule))
        .route(
            "/deterministic/swap-priorities",
            post(swap_deterministic_rule_priorities),
        )
        .route("/deterministic/{id}", get(get_deterministic_rule))
        .route("/deterministic/{id}", patch(update_deterministic_rule))
        .route("/deterministic/{id}", delete(delete_deterministic_rule))
        // LLM rules
        .route("/llm", get(list_llm_rules))
        .route("/llm", post(create_llm_rule))
        .route("/llm/{id}", get(get_llm_rule))
        .route("/llm/{id}", patch(update_llm_rule))
        .route("/llm/{id}", delete(delete_llm_rule))
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

// ============================================================================
// Deterministic Rules Endpoints
// ============================================================================

/// GET /api/rules/deterministic
///
/// List all deterministic rules, sorted by priority ASC (lower number = earlier execution).
async fn list_deterministic_rules(State(state): State<AppState>) -> impl IntoResponse {
    let repo = DeterministicRuleRepository::new(state.db.clone());

    match repo.list_all(DEFAULT_ORG_ID, DEFAULT_USER_ID).await {
        Ok(rules) => (StatusCode::OK, Json(rules)).into_response(),
        Err(e) => {
            tracing::error!("Failed to list deterministic rules: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal("Failed to list deterministic rules")),
            )
                .into_response()
        }
    }
}

/// GET /api/rules/deterministic/:id
///
/// Get a single deterministic rule by ID.
async fn get_deterministic_rule(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let repo = DeterministicRuleRepository::new(state.db.clone());

    match repo.get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &id).await {
        Ok(rule) => (StatusCode::OK, Json(rule)).into_response(),
        Err(DeterministicRuleError::NotFound(_)) => (
            StatusCode::NOT_FOUND,
            Json(ApiError::not_found(format!(
                "Deterministic rule not found: {}",
                id
            ))),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to get deterministic rule {}: {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal(format!(
                    "Failed to get deterministic rule: {}",
                    e
                ))),
            )
                .into_response()
        }
    }
}

/// Request body for creating a deterministic rule.
#[derive(Debug, Deserialize)]
pub struct CreateDeterministicRuleRequest {
    pub name: String,
    pub description: Option<String>,
    /// Defaults to Global if not specified.
    pub scope: Option<RuleScope>,
    pub scope_ref: Option<String>,
    pub priority: Option<i64>,
    pub enabled: Option<bool>,
    pub conditions_json: Value,
    pub action_type: String,
    pub action_parameters_json: Option<Value>,
    pub safe_mode: Option<SafeMode>,
}

/// POST /api/rules/deterministic
///
/// Create a new deterministic rule.
async fn create_deterministic_rule(
    State(state): State<AppState>,
    Json(body): Json<CreateDeterministicRuleRequest>,
) -> impl IntoResponse {
    // Validate required fields
    if body.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError::bad_request("Name is required")),
        )
            .into_response();
    }

    if body.action_type.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError::bad_request("Action type is required")),
        )
            .into_response();
    }

    // Validate conditions_json is not null
    if body.conditions_json.is_null() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError::bad_request("Conditions are required")),
        )
            .into_response();
    }

    let scope = body.scope.unwrap_or(RuleScope::Global);
    // Clear scope_ref if scope is Global (it would be meaningless)
    let scope_ref = if scope == RuleScope::Global {
        None
    } else {
        body.scope_ref
    };

    let new_rule = NewDeterministicRule {
        org_id: DEFAULT_ORG_ID,
        user_id: Some(DEFAULT_USER_ID),
        name: body.name,
        description: body.description,
        scope,
        scope_ref,
        priority: body.priority.unwrap_or(100),
        enabled: body.enabled.unwrap_or(true),
        disabled_reason: None,
        conditions_json: body.conditions_json,
        action_type: body.action_type,
        action_parameters_json: body
            .action_parameters_json
            .unwrap_or(Value::Object(Default::default())),
        safe_mode: body.safe_mode.unwrap_or(SafeMode::Default),
    };

    let repo = DeterministicRuleRepository::new(state.db.clone());

    match repo.create(new_rule).await {
        Ok(rule) => (StatusCode::CREATED, Json(rule)).into_response(),
        Err(e) => {
            tracing::error!("Failed to create deterministic rule: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal(format!(
                    "Failed to create deterministic rule: {}",
                    e
                ))),
            )
                .into_response()
        }
    }
}

/// Helper module for deserializing fields that distinguish between null and absent.
/// `None` = field not present (keep existing value)
/// `Some(None)` = field explicitly set to null (clear the value)
/// `Some(Some(T))` = field explicitly set to a value
mod nullable {
    use serde::{Deserialize, Deserializer};

    /// Deserialize an optional nullable field.
    /// Returns `Some(None)` for explicit null, `Some(Some(value))` for a value,
    /// and uses serde's default (None) when the field is absent.
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        // This will deserialize the outer Option - if field is present, we get Some
        // Then the inner Option handles null vs value
        Ok(Some(Option::deserialize(deserializer)?))
    }
}

/// Request body for updating a deterministic rule.
/// All fields are optional for partial updates.
///
/// For fields that can be cleared (set to null), we use `Option<Option<T>>`:
/// - Field absent: `None` - keep existing value
/// - Field set to null: `Some(None)` - clear the value
/// - Field set to a value: `Some(Some(value))` - update to new value
#[derive(Debug, Deserialize)]
pub struct UpdateDeterministicRuleRequest {
    pub name: Option<String>,
    /// Can be cleared by sending null.
    #[serde(default, deserialize_with = "nullable::deserialize")]
    pub description: Option<Option<String>>,
    pub scope: Option<RuleScope>,
    /// Can be cleared by sending null. Automatically cleared when scope is Global.
    #[serde(default, deserialize_with = "nullable::deserialize")]
    pub scope_ref: Option<Option<String>>,
    pub priority: Option<i64>,
    pub enabled: Option<bool>,
    /// Can be cleared by sending null.
    #[serde(default, deserialize_with = "nullable::deserialize")]
    pub disabled_reason: Option<Option<String>>,
    pub conditions_json: Option<Value>,
    pub action_type: Option<String>,
    pub action_parameters_json: Option<Value>,
    pub safe_mode: Option<SafeMode>,
}

/// PATCH /api/rules/deterministic/:id
///
/// Update an existing deterministic rule with partial data.
async fn update_deterministic_rule(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateDeterministicRuleRequest>,
) -> impl IntoResponse {
    let repo = DeterministicRuleRepository::new(state.db.clone());

    // First, fetch the existing rule
    let existing = match repo.get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &id).await {
        Ok(rule) => rule,
        Err(DeterministicRuleError::NotFound(_)) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiError::not_found(format!(
                    "Deterministic rule not found: {}",
                    id
                ))),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Failed to fetch deterministic rule {}: {}", id, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal(format!(
                    "Failed to fetch deterministic rule: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    // Merge the update with existing values
    // For nullable fields (Option<Option<T>>):
    // - None = field absent, keep existing
    // - Some(None) = explicit null, clear the value
    // - Some(Some(v)) = new value provided
    let description = match body.description {
        None => existing.description, // Field absent: keep existing
        Some(None) => None,           // Explicit null: clear
        Some(Some(v)) => Some(v),     // New value: use it
    };

    let disabled_reason = match body.disabled_reason {
        None => existing.disabled_reason,
        Some(None) => None,
        Some(Some(v)) => Some(v),
    };

    let scope = body.scope.unwrap_or(existing.scope);

    // Handle scope_ref: clear if scope is Global, otherwise use the update logic
    let scope_ref = if scope == RuleScope::Global {
        // Global scope never has a scope_ref
        None
    } else {
        match body.scope_ref {
            None => existing.scope_ref, // Field absent: keep existing
            Some(None) => None,         // Explicit null: clear
            Some(Some(v)) => Some(v),   // New value: use it
        }
    };

    let updated_rule = NewDeterministicRule {
        org_id: existing.org_id,
        user_id: existing.user_id,
        name: body.name.unwrap_or(existing.name),
        description,
        scope,
        scope_ref,
        priority: body.priority.unwrap_or(existing.priority),
        enabled: body.enabled.unwrap_or(existing.enabled),
        disabled_reason,
        conditions_json: body.conditions_json.unwrap_or(existing.conditions_json),
        action_type: body.action_type.unwrap_or(existing.action_type),
        action_parameters_json: body
            .action_parameters_json
            .unwrap_or(existing.action_parameters_json),
        safe_mode: body.safe_mode.unwrap_or(existing.safe_mode),
    };

    match repo
        .update(DEFAULT_ORG_ID, DEFAULT_USER_ID, &id, updated_rule)
        .await
    {
        Ok(rule) => (StatusCode::OK, Json(rule)).into_response(),
        Err(DeterministicRuleError::NotFound(_)) => (
            StatusCode::NOT_FOUND,
            Json(ApiError::not_found(format!(
                "Deterministic rule not found: {}",
                id
            ))),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to update deterministic rule {}: {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal(format!(
                    "Failed to update deterministic rule: {}",
                    e
                ))),
            )
                .into_response()
        }
    }
}

/// DELETE /api/rules/deterministic/:id
///
/// Delete a deterministic rule.
async fn delete_deterministic_rule(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let repo = DeterministicRuleRepository::new(state.db.clone());

    match repo.delete(DEFAULT_ORG_ID, DEFAULT_USER_ID, &id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(DeterministicRuleError::NotFound(_)) => (
            StatusCode::NOT_FOUND,
            Json(ApiError::not_found(format!(
                "Deterministic rule not found: {}",
                id
            ))),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to delete deterministic rule {}: {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal(format!(
                    "Failed to delete deterministic rule: {}",
                    e
                ))),
            )
                .into_response()
        }
    }
}

/// Request body for swapping priorities between two deterministic rules.
#[derive(Debug, Deserialize)]
pub struct SwapPrioritiesRequest {
    pub rule_a_id: String,
    pub rule_b_id: String,
}

/// Response body for swap priorities operation.
#[derive(Debug, Serialize, Deserialize)]
pub struct SwapPrioritiesResponse {
    pub success: bool,
}

/// POST /api/rules/deterministic/swap-priorities
///
/// Atomically swap priorities between two deterministic rules.
/// This ensures both updates succeed or neither does, preventing inconsistent state.
///
/// The implementation reads the current priorities INSIDE the transaction to prevent
/// TOCTOU (time-of-check to time-of-use) race conditions. It also verifies that each
/// UPDATE affected exactly one row to detect concurrent deletions.
async fn swap_deterministic_rule_priorities(
    State(state): State<AppState>,
    Json(body): Json<SwapPrioritiesRequest>,
) -> impl IntoResponse {
    use libsql::params;

    // Validate IDs are not empty
    if body.rule_a_id.trim().is_empty() || body.rule_b_id.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError::bad_request("Both rule IDs are required")),
        )
            .into_response();
    }

    // Validate IDs are different
    if body.rule_a_id == body.rule_b_id {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError::bad_request(
                "Cannot swap a rule's priority with itself",
            )),
        )
            .into_response();
    }

    // Get a connection for the transaction
    let conn = match state.db.connection().await {
        Ok(conn) => conn,
        Err(e) => {
            tracing::error!("Failed to get database connection: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal("Failed to connect to database")),
            )
                .into_response();
        }
    };

    // Start a transaction FIRST to ensure all reads and writes are atomic
    let tx = match conn.transaction().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Failed to start transaction: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal("Failed to start transaction")),
            )
                .into_response();
        }
    };

    // Fetch both rules INSIDE the transaction to prevent TOCTOU race conditions
    let rule_a_priority: i64 = match tx
        .query(
            "SELECT priority FROM deterministic_rules WHERE id = ?1 AND org_id = ?2 AND (user_id = ?3 OR user_id IS NULL)",
            params![body.rule_a_id.clone(), DEFAULT_ORG_ID, DEFAULT_USER_ID],
        )
        .await
    {
        Ok(mut rows) => match rows.next().await {
            Ok(Some(row)) => row.get(0).unwrap_or(0),
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ApiError::not_found(format!(
                        "Deterministic rule not found: {}",
                        body.rule_a_id
                    ))),
                )
                    .into_response();
            }
            Err(e) => {
                tracing::error!("Failed to fetch rule {}: {}", body.rule_a_id, e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiError::internal("Failed to fetch rule")),
                )
                    .into_response();
            }
        },
        Err(e) => {
            tracing::error!("Failed to query rule {}: {}", body.rule_a_id, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal("Failed to fetch rule")),
            )
                .into_response();
        }
    };

    let rule_b_priority: i64 = match tx
        .query(
            "SELECT priority FROM deterministic_rules WHERE id = ?1 AND org_id = ?2 AND (user_id = ?3 OR user_id IS NULL)",
            params![body.rule_b_id.clone(), DEFAULT_ORG_ID, DEFAULT_USER_ID],
        )
        .await
    {
        Ok(mut rows) => match rows.next().await {
            Ok(Some(row)) => row.get(0).unwrap_or(0),
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ApiError::not_found(format!(
                        "Deterministic rule not found: {}",
                        body.rule_b_id
                    ))),
                )
                    .into_response();
            }
            Err(e) => {
                tracing::error!("Failed to fetch rule {}: {}", body.rule_b_id, e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiError::internal("Failed to fetch rule")),
                )
                    .into_response();
            }
        },
        Err(e) => {
            tracing::error!("Failed to query rule {}: {}", body.rule_b_id, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal("Failed to fetch rule")),
            )
                .into_response();
        }
    };

    // Update rule A with rule B's priority and verify exactly one row was affected
    let rows_affected_a = match tx
        .execute(
            "UPDATE deterministic_rules SET priority = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?2 AND org_id = ?3 AND (user_id = ?4 OR user_id IS NULL)",
            params![rule_b_priority, body.rule_a_id.clone(), DEFAULT_ORG_ID, DEFAULT_USER_ID],
        )
        .await
    {
        Ok(count) => count,
        Err(e) => {
            tracing::error!("Failed to update rule {}: {}", body.rule_a_id, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal("Failed to update rule priority")),
            )
                .into_response();
        }
    };

    if rows_affected_a != 1 {
        tracing::error!(
            "Expected 1 row affected when updating rule {}, but got {}",
            body.rule_a_id,
            rows_affected_a
        );
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::internal(
                "Rule was modified or deleted during swap",
            )),
        )
            .into_response();
    }

    // Update rule B with rule A's priority and verify exactly one row was affected
    let rows_affected_b = match tx
        .execute(
            "UPDATE deterministic_rules SET priority = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?2 AND org_id = ?3 AND (user_id = ?4 OR user_id IS NULL)",
            params![rule_a_priority, body.rule_b_id.clone(), DEFAULT_ORG_ID, DEFAULT_USER_ID],
        )
        .await
    {
        Ok(count) => count,
        Err(e) => {
            tracing::error!("Failed to update rule {}: {}", body.rule_b_id, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal("Failed to update rule priority")),
            )
                .into_response();
        }
    };

    if rows_affected_b != 1 {
        tracing::error!(
            "Expected 1 row affected when updating rule {}, but got {}",
            body.rule_b_id,
            rows_affected_b
        );
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::internal(
                "Rule was modified or deleted during swap",
            )),
        )
            .into_response();
    }

    // Commit the transaction
    if let Err(e) = tx.commit().await {
        tracing::error!("Failed to commit transaction: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::internal("Failed to commit priority swap")),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(SwapPrioritiesResponse { success: true }),
    )
        .into_response()
}

// ============================================================================
// LLM Rules Endpoints
// ============================================================================

/// GET /api/rules/llm
///
/// List all LLM rules.
async fn list_llm_rules(State(state): State<AppState>) -> impl IntoResponse {
    let repo = LlmRuleRepository::new(state.db.clone());

    match repo.list_all(DEFAULT_ORG_ID, DEFAULT_USER_ID).await {
        Ok(rules) => (StatusCode::OK, Json(rules)).into_response(),
        Err(e) => {
            tracing::error!("Failed to list LLM rules: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal("Failed to list LLM rules")),
            )
                .into_response()
        }
    }
}

/// GET /api/rules/llm/:id
///
/// Get a single LLM rule by ID.
async fn get_llm_rule(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let repo = LlmRuleRepository::new(state.db.clone());

    match repo.get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &id).await {
        Ok(rule) => (StatusCode::OK, Json(rule)).into_response(),
        Err(LlmRuleError::NotFound(_)) => (
            StatusCode::NOT_FOUND,
            Json(ApiError::not_found(format!("LLM rule not found: {}", id))),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to get LLM rule {}: {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal(format!("Failed to get LLM rule: {}", e))),
            )
                .into_response()
        }
    }
}

/// Request body for creating an LLM rule.
#[derive(Debug, Deserialize)]
pub struct CreateLlmRuleRequest {
    pub name: String,
    pub description: Option<String>,
    pub scope: Option<RuleScope>,
    pub scope_ref: Option<String>,
    pub rule_text: String,
    pub enabled: Option<bool>,
    pub metadata_json: Option<Value>,
}

/// POST /api/rules/llm
///
/// Create a new LLM rule.
async fn create_llm_rule(
    State(state): State<AppState>,
    Json(body): Json<CreateLlmRuleRequest>,
) -> impl IntoResponse {
    // Validate required fields
    if body.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError::bad_request("Name is required")),
        )
            .into_response();
    }

    if body.rule_text.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError::bad_request("Rule text is required")),
        )
            .into_response();
    }

    let scope = body.scope.unwrap_or(RuleScope::Global);
    // Clear scope_ref if scope is Global (it would be meaningless)
    let scope_ref = if scope == RuleScope::Global {
        None
    } else {
        body.scope_ref
    };

    let new_rule = NewLlmRule {
        org_id: DEFAULT_ORG_ID,
        user_id: Some(DEFAULT_USER_ID),
        name: body.name,
        description: body.description,
        scope,
        scope_ref,
        rule_text: body.rule_text,
        enabled: body.enabled.unwrap_or(true),
        metadata_json: body
            .metadata_json
            .unwrap_or(Value::Object(Default::default())),
    };

    let repo = LlmRuleRepository::new(state.db.clone());

    match repo.create(new_rule).await {
        Ok(rule) => (StatusCode::CREATED, Json(rule)).into_response(),
        Err(e) => {
            tracing::error!("Failed to create LLM rule: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal(format!(
                    "Failed to create LLM rule: {}",
                    e
                ))),
            )
                .into_response()
        }
    }
}

/// Request body for updating an LLM rule.
/// All fields are optional for partial updates.
///
/// For fields that can be cleared (set to null), we use `Option<Option<T>>`:
/// - Field absent: `None` - keep existing value
/// - Field set to null: `Some(None)` - clear the value
/// - Field set to a value: `Some(Some(value))` - update to new value
#[derive(Debug, Deserialize)]
pub struct UpdateLlmRuleRequest {
    pub name: Option<String>,
    /// Can be cleared by sending null.
    #[serde(default, deserialize_with = "nullable::deserialize")]
    pub description: Option<Option<String>>,
    pub scope: Option<RuleScope>,
    /// Can be cleared by sending null. Automatically cleared when scope is Global.
    #[serde(default, deserialize_with = "nullable::deserialize")]
    pub scope_ref: Option<Option<String>>,
    pub rule_text: Option<String>,
    pub enabled: Option<bool>,
    pub metadata_json: Option<Value>,
}

/// PATCH /api/rules/llm/:id
///
/// Update an existing LLM rule with partial data.
async fn update_llm_rule(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateLlmRuleRequest>,
) -> impl IntoResponse {
    let repo = LlmRuleRepository::new(state.db.clone());

    // First, fetch the existing rule
    let existing = match repo.get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &id).await {
        Ok(rule) => rule,
        Err(LlmRuleError::NotFound(_)) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiError::not_found(format!("LLM rule not found: {}", id))),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Failed to fetch LLM rule {}: {}", id, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal(format!(
                    "Failed to fetch LLM rule: {}",
                    e
                ))),
            )
                .into_response();
        }
    };

    // Merge the update with existing values
    // For nullable fields (Option<Option<T>>):
    // - None = field absent, keep existing
    // - Some(None) = explicit null, clear the value
    // - Some(Some(v)) = new value provided
    let description = match body.description {
        None => existing.description,
        Some(None) => None,
        Some(Some(v)) => Some(v),
    };

    let scope = body.scope.unwrap_or(existing.scope);

    // Handle scope_ref: clear if scope is Global, otherwise use the update logic
    let scope_ref = if scope == RuleScope::Global {
        None
    } else {
        match body.scope_ref {
            None => existing.scope_ref,
            Some(None) => None,
            Some(Some(v)) => Some(v),
        }
    };

    let updated_rule = NewLlmRule {
        org_id: existing.org_id,
        user_id: existing.user_id,
        name: body.name.unwrap_or(existing.name),
        description,
        scope,
        scope_ref,
        rule_text: body.rule_text.unwrap_or(existing.rule_text),
        enabled: body.enabled.unwrap_or(existing.enabled),
        metadata_json: body.metadata_json.unwrap_or(existing.metadata_json),
    };

    match repo
        .update(DEFAULT_ORG_ID, DEFAULT_USER_ID, &id, updated_rule)
        .await
    {
        Ok(rule) => (StatusCode::OK, Json(rule)).into_response(),
        Err(LlmRuleError::NotFound(_)) => (
            StatusCode::NOT_FOUND,
            Json(ApiError::not_found(format!("LLM rule not found: {}", id))),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to update LLM rule {}: {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal(format!(
                    "Failed to update LLM rule: {}",
                    e
                ))),
            )
                .into_response()
        }
    }
}

/// DELETE /api/rules/llm/:id
///
/// Delete an LLM rule.
async fn delete_llm_rule(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let repo = LlmRuleRepository::new(state.db.clone());

    match repo.delete(DEFAULT_ORG_ID, DEFAULT_USER_ID, &id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(LlmRuleError::NotFound(_)) => (
            StatusCode::NOT_FOUND,
            Json(ApiError::not_found(format!("LLM rule not found: {}", id))),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to delete LLM rule {}: {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal(format!(
                    "Failed to delete LLM rule: {}",
                    e
                ))),
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ashford_core::{Database, DeterministicRule, LlmRule, migrations::run_migrations};
    use axum::body::to_bytes;
    use serde_json::json;
    use tempfile::TempDir;

    async fn setup_db() -> (Database, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        let db_path = dir.path().join("test.sqlite");
        let db = Database::new(&db_path).await.expect("create db");
        run_migrations(&db).await.expect("migrations");
        (db, dir)
    }

    #[tokio::test]
    async fn list_deterministic_rules_returns_empty_list() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        let response = list_deterministic_rules(State(state)).await.into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let body: Vec<DeterministicRule> = serde_json::from_slice(&body_bytes).expect("json body");
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn create_deterministic_rule_success() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        let request = CreateDeterministicRuleRequest {
            name: "Test Rule".to_string(),
            description: Some("A test rule".to_string()),
            scope: Some(RuleScope::Global),
            scope_ref: None,
            priority: Some(50),
            enabled: Some(true),
            conditions_json: json!({"type": "sender_domain", "value": "example.com"}),
            action_type: "archive".to_string(),
            action_parameters_json: Some(json!({})),
            safe_mode: Some(SafeMode::Default),
        };

        let response = create_deterministic_rule(State(state), Json(request))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let body: DeterministicRule = serde_json::from_slice(&body_bytes).expect("json body");
        assert_eq!(body.name, "Test Rule");
        assert_eq!(body.priority, 50);
        assert!(body.enabled);
    }

    #[tokio::test]
    async fn create_deterministic_rule_missing_name() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        let request = CreateDeterministicRuleRequest {
            name: "".to_string(),
            description: None,
            scope: Some(RuleScope::Global),
            scope_ref: None,
            priority: None,
            enabled: None,
            conditions_json: json!({"type": "sender_domain", "value": "example.com"}),
            action_type: "archive".to_string(),
            action_parameters_json: None,
            safe_mode: None,
        };

        let response = create_deterministic_rule(State(state), Json(request))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn update_deterministic_rule_partial() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // First create a rule
        let create_request = CreateDeterministicRuleRequest {
            name: "Original Name".to_string(),
            description: Some("Original description".to_string()),
            scope: Some(RuleScope::Global),
            scope_ref: None,
            priority: Some(100),
            enabled: Some(true),
            conditions_json: json!({"type": "sender_domain", "value": "example.com"}),
            action_type: "archive".to_string(),
            action_parameters_json: None,
            safe_mode: None,
        };

        let create_response = create_deterministic_rule(State(state.clone()), Json(create_request))
            .await
            .into_response();
        let body_bytes = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let created: DeterministicRule = serde_json::from_slice(&body_bytes).expect("json body");

        // Now update only the name
        let update_request = UpdateDeterministicRuleRequest {
            name: Some("Updated Name".to_string()),
            description: None,
            scope: None,
            scope_ref: None,
            priority: None,
            enabled: None,
            disabled_reason: None,
            conditions_json: None,
            action_type: None,
            action_parameters_json: None,
            safe_mode: None,
        };

        let update_response =
            update_deterministic_rule(State(state), Path(created.id.clone()), Json(update_request))
                .await
                .into_response();

        assert_eq!(update_response.status(), StatusCode::OK);
        let body_bytes = to_bytes(update_response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let updated: DeterministicRule = serde_json::from_slice(&body_bytes).expect("json body");
        assert_eq!(updated.name, "Updated Name");
        assert_eq!(
            updated.description,
            Some("Original description".to_string())
        ); // Unchanged
        assert_eq!(updated.priority, 100); // Unchanged
    }

    #[tokio::test]
    async fn delete_deterministic_rule_success() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // First create a rule
        let create_request = CreateDeterministicRuleRequest {
            name: "To Delete".to_string(),
            description: None,
            scope: Some(RuleScope::Global),
            scope_ref: None,
            priority: None,
            enabled: None,
            conditions_json: json!({"type": "sender_domain", "value": "example.com"}),
            action_type: "archive".to_string(),
            action_parameters_json: None,
            safe_mode: None,
        };

        let create_response = create_deterministic_rule(State(state.clone()), Json(create_request))
            .await
            .into_response();
        let body_bytes = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let created: DeterministicRule = serde_json::from_slice(&body_bytes).expect("json body");

        // Delete the rule
        let delete_response =
            delete_deterministic_rule(State(state.clone()), Path(created.id.clone()))
                .await
                .into_response();

        assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

        // Verify it's gone
        let get_response = get_deterministic_rule(State(state), Path(created.id))
            .await
            .into_response();
        assert_eq!(get_response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn delete_deterministic_rule_not_found() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        let response = delete_deterministic_rule(State(state), Path("nonexistent".to_string()))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn list_llm_rules_returns_empty_list() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        let response = list_llm_rules(State(state)).await.into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let body: Vec<LlmRule> = serde_json::from_slice(&body_bytes).expect("json body");
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn create_llm_rule_success() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        let request = CreateLlmRuleRequest {
            name: "Test LLM Rule".to_string(),
            description: Some("A test LLM rule".to_string()),
            scope: Some(RuleScope::Global),
            scope_ref: None,
            rule_text: "Archive all newsletters".to_string(),
            enabled: Some(true),
            metadata_json: None,
        };

        let response = create_llm_rule(State(state), Json(request))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let body: LlmRule = serde_json::from_slice(&body_bytes).expect("json body");
        assert_eq!(body.name, "Test LLM Rule");
        assert_eq!(body.rule_text, "Archive all newsletters");
        assert!(body.enabled);
    }

    #[tokio::test]
    async fn create_llm_rule_missing_rule_text() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        let request = CreateLlmRuleRequest {
            name: "Test Rule".to_string(),
            description: None,
            scope: None,
            scope_ref: None,
            rule_text: "".to_string(),
            enabled: None,
            metadata_json: None,
        };

        let response = create_llm_rule(State(state), Json(request))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn update_llm_rule_partial() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // First create a rule
        let create_request = CreateLlmRuleRequest {
            name: "Original Name".to_string(),
            description: Some("Original description".to_string()),
            scope: Some(RuleScope::Global),
            scope_ref: None,
            rule_text: "Original rule text".to_string(),
            enabled: Some(true),
            metadata_json: None,
        };

        let create_response = create_llm_rule(State(state.clone()), Json(create_request))
            .await
            .into_response();
        let body_bytes = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let created: LlmRule = serde_json::from_slice(&body_bytes).expect("json body");

        // Now update only the rule_text
        let update_request = UpdateLlmRuleRequest {
            name: None,
            description: None,
            scope: None,
            scope_ref: None,
            rule_text: Some("Updated rule text".to_string()),
            enabled: None,
            metadata_json: None,
        };

        let update_response =
            update_llm_rule(State(state), Path(created.id.clone()), Json(update_request))
                .await
                .into_response();

        assert_eq!(update_response.status(), StatusCode::OK);
        let body_bytes = to_bytes(update_response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let updated: LlmRule = serde_json::from_slice(&body_bytes).expect("json body");
        assert_eq!(updated.name, "Original Name"); // Unchanged
        assert_eq!(updated.rule_text, "Updated rule text");
    }

    #[tokio::test]
    async fn delete_llm_rule_success() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // First create a rule
        let create_request = CreateLlmRuleRequest {
            name: "To Delete".to_string(),
            description: None,
            scope: None,
            scope_ref: None,
            rule_text: "Some rule text".to_string(),
            enabled: None,
            metadata_json: None,
        };

        let create_response = create_llm_rule(State(state.clone()), Json(create_request))
            .await
            .into_response();
        let body_bytes = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let created: LlmRule = serde_json::from_slice(&body_bytes).expect("json body");

        // Delete the rule
        let delete_response = delete_llm_rule(State(state.clone()), Path(created.id.clone()))
            .await
            .into_response();

        assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

        // Verify it's gone
        let get_response = get_llm_rule(State(state), Path(created.id))
            .await
            .into_response();
        assert_eq!(get_response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn deterministic_rules_sorted_by_priority() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // Create rules with different priorities (out of order)
        for (name, priority) in [
            ("Low Priority", 100),
            ("High Priority", 10),
            ("Mid Priority", 50),
        ] {
            let request = CreateDeterministicRuleRequest {
                name: name.to_string(),
                description: None,
                scope: Some(RuleScope::Global),
                scope_ref: None,
                priority: Some(priority),
                enabled: None,
                conditions_json: json!({"type": "sender_domain", "value": "example.com"}),
                action_type: "archive".to_string(),
                action_parameters_json: None,
                safe_mode: None,
            };
            create_deterministic_rule(State(state.clone()), Json(request)).await;
        }

        // List rules
        let response = list_deterministic_rules(State(state)).await.into_response();
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let rules: Vec<DeterministicRule> = serde_json::from_slice(&body_bytes).expect("json body");

        assert_eq!(rules.len(), 3);
        assert_eq!(rules[0].name, "High Priority");
        assert_eq!(rules[0].priority, 10);
        assert_eq!(rules[1].name, "Mid Priority");
        assert_eq!(rules[1].priority, 50);
        assert_eq!(rules[2].name, "Low Priority");
        assert_eq!(rules[2].priority, 100);
    }

    #[tokio::test]
    async fn get_deterministic_rule_not_found() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        let response = get_deterministic_rule(State(state), Path("nonexistent".to_string()))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn update_deterministic_rule_not_found() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        let update_request = UpdateDeterministicRuleRequest {
            name: Some("New Name".to_string()),
            description: None,
            scope: None,
            scope_ref: None,
            priority: None,
            enabled: None,
            disabled_reason: None,
            conditions_json: None,
            action_type: None,
            action_parameters_json: None,
            safe_mode: None,
        };

        let response = update_deterministic_rule(
            State(state),
            Path("nonexistent".to_string()),
            Json(update_request),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn create_deterministic_rule_missing_action_type() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        let request = CreateDeterministicRuleRequest {
            name: "Test Rule".to_string(),
            description: None,
            scope: Some(RuleScope::Global),
            scope_ref: None,
            priority: None,
            enabled: None,
            conditions_json: json!({"type": "sender_domain", "value": "example.com"}),
            action_type: "".to_string(), // Empty action_type
            action_parameters_json: None,
            safe_mode: None,
        };

        let response = create_deterministic_rule(State(state), Json(request))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_deterministic_rule_null_conditions() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        let request = CreateDeterministicRuleRequest {
            name: "Test Rule".to_string(),
            description: None,
            scope: Some(RuleScope::Global),
            scope_ref: None,
            priority: None,
            enabled: None,
            conditions_json: Value::Null, // Null conditions
            action_type: "archive".to_string(),
            action_parameters_json: None,
            safe_mode: None,
        };

        let response = create_deterministic_rule(State(state), Json(request))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_llm_rule_missing_name() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        let request = CreateLlmRuleRequest {
            name: "".to_string(), // Empty name
            description: None,
            scope: None,
            scope_ref: None,
            rule_text: "Some rule text".to_string(),
            enabled: None,
            metadata_json: None,
        };

        let response = create_llm_rule(State(state), Json(request))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_llm_rule_not_found() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        let response = get_llm_rule(State(state), Path("nonexistent".to_string()))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn update_llm_rule_not_found() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        let update_request = UpdateLlmRuleRequest {
            name: Some("New Name".to_string()),
            description: None,
            scope: None,
            scope_ref: None,
            rule_text: None,
            enabled: None,
            metadata_json: None,
        };

        let response = update_llm_rule(
            State(state),
            Path("nonexistent".to_string()),
            Json(update_request),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn delete_llm_rule_not_found() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        let response = delete_llm_rule(State(state), Path("nonexistent".to_string()))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_deterministic_rule_success() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // First create a rule
        let create_request = CreateDeterministicRuleRequest {
            name: "Test Rule".to_string(),
            description: Some("A description".to_string()),
            scope: Some(RuleScope::Global),
            scope_ref: None,
            priority: Some(50),
            enabled: Some(true),
            conditions_json: json!({"type": "sender_domain", "value": "example.com"}),
            action_type: "archive".to_string(),
            action_parameters_json: None,
            safe_mode: None,
        };

        let create_response = create_deterministic_rule(State(state.clone()), Json(create_request))
            .await
            .into_response();
        let body_bytes = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let created: DeterministicRule = serde_json::from_slice(&body_bytes).expect("json body");

        // Now get the rule
        let get_response = get_deterministic_rule(State(state), Path(created.id.clone()))
            .await
            .into_response();

        assert_eq!(get_response.status(), StatusCode::OK);
        let body_bytes = to_bytes(get_response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let fetched: DeterministicRule = serde_json::from_slice(&body_bytes).expect("json body");
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.name, "Test Rule");
        assert_eq!(fetched.priority, 50);
    }

    #[tokio::test]
    async fn get_llm_rule_success() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // First create a rule
        let create_request = CreateLlmRuleRequest {
            name: "Test LLM Rule".to_string(),
            description: Some("A description".to_string()),
            scope: Some(RuleScope::Global),
            scope_ref: None,
            rule_text: "Archive newsletters".to_string(),
            enabled: Some(true),
            metadata_json: None,
        };

        let create_response = create_llm_rule(State(state.clone()), Json(create_request))
            .await
            .into_response();
        let body_bytes = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let created: LlmRule = serde_json::from_slice(&body_bytes).expect("json body");

        // Now get the rule
        let get_response = get_llm_rule(State(state), Path(created.id.clone()))
            .await
            .into_response();

        assert_eq!(get_response.status(), StatusCode::OK);
        let body_bytes = to_bytes(get_response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let fetched: LlmRule = serde_json::from_slice(&body_bytes).expect("json body");
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.name, "Test LLM Rule");
        assert_eq!(fetched.rule_text, "Archive newsletters");
    }

    #[tokio::test]
    async fn update_deterministic_rule_enabled_toggle() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // Create a rule
        let create_request = CreateDeterministicRuleRequest {
            name: "Test Rule".to_string(),
            description: None,
            scope: Some(RuleScope::Global),
            scope_ref: None,
            priority: None,
            enabled: Some(true),
            conditions_json: json!({"type": "sender_domain", "value": "example.com"}),
            action_type: "archive".to_string(),
            action_parameters_json: None,
            safe_mode: None,
        };

        let create_response = create_deterministic_rule(State(state.clone()), Json(create_request))
            .await
            .into_response();
        let body_bytes = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let created: DeterministicRule = serde_json::from_slice(&body_bytes).expect("json body");
        assert!(created.enabled);

        // Disable the rule (using None for nullable fields = keep existing)
        let update_request = UpdateDeterministicRuleRequest {
            name: None,
            description: None, // None = keep existing
            scope: None,
            scope_ref: None, // None = keep existing
            priority: None,
            enabled: Some(false),
            disabled_reason: None, // None = keep existing
            conditions_json: None,
            action_type: None,
            action_parameters_json: None,
            safe_mode: None,
        };

        let update_response =
            update_deterministic_rule(State(state), Path(created.id.clone()), Json(update_request))
                .await
                .into_response();

        assert_eq!(update_response.status(), StatusCode::OK);
        let body_bytes = to_bytes(update_response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let updated: DeterministicRule = serde_json::from_slice(&body_bytes).expect("json body");
        assert!(!updated.enabled);
    }

    #[tokio::test]
    async fn update_llm_rule_enabled_toggle() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // Create a rule
        let create_request = CreateLlmRuleRequest {
            name: "Test Rule".to_string(),
            description: None,
            scope: None,
            scope_ref: None,
            rule_text: "Some rule text".to_string(),
            enabled: Some(true),
            metadata_json: None,
        };

        let create_response = create_llm_rule(State(state.clone()), Json(create_request))
            .await
            .into_response();
        let body_bytes = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let created: LlmRule = serde_json::from_slice(&body_bytes).expect("json body");
        assert!(created.enabled);

        // Disable the rule (using None for nullable fields = keep existing)
        let update_request = UpdateLlmRuleRequest {
            name: None,
            description: None, // None = keep existing
            scope: None,
            scope_ref: None, // None = keep existing
            rule_text: None,
            enabled: Some(false),
            metadata_json: None,
        };

        let update_response =
            update_llm_rule(State(state), Path(created.id.clone()), Json(update_request))
                .await
                .into_response();

        assert_eq!(update_response.status(), StatusCode::OK);
        let body_bytes = to_bytes(update_response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let updated: LlmRule = serde_json::from_slice(&body_bytes).expect("json body");
        assert!(!updated.enabled);
    }

    // ========================================================================
    // Task 20: Tests for nullable fields - clearing optional fields with null
    // ========================================================================

    /// Test deserialization of UpdateDeterministicRuleRequest with null values.
    /// Verifies the three-state logic:
    /// - Field absent: None
    /// - Field set to null: Some(None)
    /// - Field set to a value: Some(Some(value))
    #[test]
    fn deserialize_update_deterministic_rule_nullable_fields() {
        // Field absent - description not included in JSON
        let json_absent = r#"{"name": "test"}"#;
        let parsed: UpdateDeterministicRuleRequest = serde_json::from_str(json_absent).unwrap();
        assert!(parsed.description.is_none(), "Absent field should be None");
        assert!(
            parsed.scope_ref.is_none(),
            "Absent scope_ref should be None"
        );
        assert!(
            parsed.disabled_reason.is_none(),
            "Absent disabled_reason should be None"
        );

        // Field explicitly set to null - should be Some(None)
        let json_null = r#"{"description": null, "scope_ref": null, "disabled_reason": null}"#;
        let parsed: UpdateDeterministicRuleRequest = serde_json::from_str(json_null).unwrap();
        assert_eq!(
            parsed.description,
            Some(None),
            "Explicit null should be Some(None)"
        );
        assert_eq!(
            parsed.scope_ref,
            Some(None),
            "Explicit null scope_ref should be Some(None)"
        );
        assert_eq!(
            parsed.disabled_reason,
            Some(None),
            "Explicit null disabled_reason should be Some(None)"
        );

        // Field set to a value - should be Some(Some(value))
        let json_value =
            r#"{"description": "hello", "scope_ref": "ref123", "disabled_reason": "some reason"}"#;
        let parsed: UpdateDeterministicRuleRequest = serde_json::from_str(json_value).unwrap();
        assert_eq!(
            parsed.description,
            Some(Some("hello".to_string())),
            "Value should be Some(Some(value))"
        );
        assert_eq!(
            parsed.scope_ref,
            Some(Some("ref123".to_string())),
            "Value scope_ref should be Some(Some(value))"
        );
        assert_eq!(
            parsed.disabled_reason,
            Some(Some("some reason".to_string())),
            "Value disabled_reason should be Some(Some(value))"
        );
    }

    /// Test deserialization of UpdateLlmRuleRequest with null values.
    #[test]
    fn deserialize_update_llm_rule_nullable_fields() {
        // Field absent
        let json_absent = r#"{"name": "test"}"#;
        let parsed: UpdateLlmRuleRequest = serde_json::from_str(json_absent).unwrap();
        assert!(parsed.description.is_none(), "Absent field should be None");
        assert!(
            parsed.scope_ref.is_none(),
            "Absent scope_ref should be None"
        );

        // Field explicitly set to null
        let json_null = r#"{"description": null, "scope_ref": null}"#;
        let parsed: UpdateLlmRuleRequest = serde_json::from_str(json_null).unwrap();
        assert_eq!(
            parsed.description,
            Some(None),
            "Explicit null should be Some(None)"
        );
        assert_eq!(
            parsed.scope_ref,
            Some(None),
            "Explicit null scope_ref should be Some(None)"
        );

        // Field set to a value
        let json_value = r#"{"description": "world", "scope_ref": "abc"}"#;
        let parsed: UpdateLlmRuleRequest = serde_json::from_str(json_value).unwrap();
        assert_eq!(
            parsed.description,
            Some(Some("world".to_string())),
            "Value should be Some(Some(value))"
        );
        assert_eq!(
            parsed.scope_ref,
            Some(Some("abc".to_string())),
            "Value scope_ref should be Some(Some(value))"
        );
    }

    /// Test that PATCH with explicit null for description clears the description.
    #[tokio::test]
    async fn update_deterministic_rule_clear_description_with_null() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // Create a rule with a description
        let create_request = CreateDeterministicRuleRequest {
            name: "Rule With Description".to_string(),
            description: Some("Initial description".to_string()),
            scope: Some(RuleScope::Global),
            scope_ref: None,
            priority: None,
            enabled: None,
            conditions_json: json!({"type": "sender_domain", "value": "example.com"}),
            action_type: "archive".to_string(),
            action_parameters_json: None,
            safe_mode: None,
        };

        let create_response = create_deterministic_rule(State(state.clone()), Json(create_request))
            .await
            .into_response();
        let body_bytes = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: DeterministicRule = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(created.description, Some("Initial description".to_string()));

        // Update with explicit null - parse from JSON to get Some(None)
        let update_json = r#"{"description": null}"#;
        let update_request: UpdateDeterministicRuleRequest =
            serde_json::from_str(update_json).unwrap();

        let update_response = update_deterministic_rule(
            State(state.clone()),
            Path(created.id.clone()),
            Json(update_request),
        )
        .await
        .into_response();

        assert_eq!(update_response.status(), StatusCode::OK);
        let body_bytes = to_bytes(update_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let updated: DeterministicRule = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(updated.description, None, "Description should be cleared");
    }

    /// Test that PATCH with description omitted keeps the existing description.
    #[tokio::test]
    async fn update_deterministic_rule_keep_description_when_absent() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // Create a rule with a description
        let create_request = CreateDeterministicRuleRequest {
            name: "Rule With Description".to_string(),
            description: Some("Keep this description".to_string()),
            scope: Some(RuleScope::Global),
            scope_ref: None,
            priority: None,
            enabled: None,
            conditions_json: json!({"type": "sender_domain", "value": "example.com"}),
            action_type: "archive".to_string(),
            action_parameters_json: None,
            safe_mode: None,
        };

        let create_response = create_deterministic_rule(State(state.clone()), Json(create_request))
            .await
            .into_response();
        let body_bytes = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: DeterministicRule = serde_json::from_slice(&body_bytes).unwrap();

        // Update with description field absent (only update name)
        let update_json = r#"{"name": "Updated Name"}"#;
        let update_request: UpdateDeterministicRuleRequest =
            serde_json::from_str(update_json).unwrap();

        let update_response = update_deterministic_rule(
            State(state.clone()),
            Path(created.id.clone()),
            Json(update_request),
        )
        .await
        .into_response();

        assert_eq!(update_response.status(), StatusCode::OK);
        let body_bytes = to_bytes(update_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let updated: DeterministicRule = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(updated.name, "Updated Name");
        assert_eq!(
            updated.description,
            Some("Keep this description".to_string()),
            "Description should be preserved"
        );
    }

    /// Test that PATCH with explicit null for scope_ref clears the scope_ref.
    #[tokio::test]
    async fn update_deterministic_rule_clear_scope_ref_with_null() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // Create a rule with domain scope and scope_ref
        let create_request = CreateDeterministicRuleRequest {
            name: "Rule With Scope Ref".to_string(),
            description: None,
            scope: Some(RuleScope::Domain),
            scope_ref: Some("example.com".to_string()),
            priority: None,
            enabled: None,
            conditions_json: json!({"type": "sender_domain", "value": "example.com"}),
            action_type: "archive".to_string(),
            action_parameters_json: None,
            safe_mode: None,
        };

        let create_response = create_deterministic_rule(State(state.clone()), Json(create_request))
            .await
            .into_response();
        let body_bytes = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: DeterministicRule = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(created.scope_ref, Some("example.com".to_string()));

        // Update with explicit null for scope_ref (keeping scope as Domain)
        let update_json = r#"{"scope_ref": null}"#;
        let update_request: UpdateDeterministicRuleRequest =
            serde_json::from_str(update_json).unwrap();

        let update_response = update_deterministic_rule(
            State(state.clone()),
            Path(created.id.clone()),
            Json(update_request),
        )
        .await
        .into_response();

        assert_eq!(update_response.status(), StatusCode::OK);
        let body_bytes = to_bytes(update_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let updated: DeterministicRule = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(updated.scope_ref, None, "scope_ref should be cleared");
        assert_eq!(
            updated.scope,
            RuleScope::Domain,
            "scope should remain Domain"
        );
    }

    /// Test that PATCH changing scope to Global automatically clears scope_ref.
    #[tokio::test]
    async fn update_deterministic_rule_scope_to_global_clears_scope_ref() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // Create a rule with domain scope and scope_ref
        let create_request = CreateDeterministicRuleRequest {
            name: "Rule With Scope Ref".to_string(),
            description: None,
            scope: Some(RuleScope::Domain),
            scope_ref: Some("example.com".to_string()),
            priority: None,
            enabled: None,
            conditions_json: json!({"type": "sender_domain", "value": "example.com"}),
            action_type: "archive".to_string(),
            action_parameters_json: None,
            safe_mode: None,
        };

        let create_response = create_deterministic_rule(State(state.clone()), Json(create_request))
            .await
            .into_response();
        let body_bytes = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: DeterministicRule = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(created.scope, RuleScope::Domain);
        assert_eq!(created.scope_ref, Some("example.com".to_string()));

        // Update scope to Global - scope_ref should be automatically cleared
        // Note: we're NOT explicitly setting scope_ref to null; it should be cleared
        // because Global scope never has a scope_ref
        let update_json = r#"{"scope": "global"}"#;
        let update_request: UpdateDeterministicRuleRequest =
            serde_json::from_str(update_json).unwrap();

        let update_response = update_deterministic_rule(
            State(state.clone()),
            Path(created.id.clone()),
            Json(update_request),
        )
        .await
        .into_response();

        assert_eq!(update_response.status(), StatusCode::OK);
        let body_bytes = to_bytes(update_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let updated: DeterministicRule = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(updated.scope, RuleScope::Global);
        assert_eq!(
            updated.scope_ref, None,
            "scope_ref should be cleared when scope is Global"
        );
    }

    /// Test LLM rule - clearing description with explicit null.
    #[tokio::test]
    async fn update_llm_rule_clear_description_with_null() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // Create a rule with a description
        let create_request = CreateLlmRuleRequest {
            name: "LLM Rule".to_string(),
            description: Some("Initial LLM description".to_string()),
            scope: None,
            scope_ref: None,
            rule_text: "Archive newsletters".to_string(),
            enabled: None,
            metadata_json: None,
        };

        let create_response = create_llm_rule(State(state.clone()), Json(create_request))
            .await
            .into_response();
        let body_bytes = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: LlmRule = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(
            created.description,
            Some("Initial LLM description".to_string())
        );

        // Clear description with explicit null
        let update_json = r#"{"description": null}"#;
        let update_request: UpdateLlmRuleRequest = serde_json::from_str(update_json).unwrap();

        let update_response = update_llm_rule(
            State(state.clone()),
            Path(created.id.clone()),
            Json(update_request),
        )
        .await
        .into_response();

        assert_eq!(update_response.status(), StatusCode::OK);
        let body_bytes = to_bytes(update_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let updated: LlmRule = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(updated.description, None, "Description should be cleared");
    }

    /// Test LLM rule - changing scope to Global clears scope_ref.
    #[tokio::test]
    async fn update_llm_rule_scope_to_global_clears_scope_ref() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // Create a rule with sender scope and scope_ref
        let create_request = CreateLlmRuleRequest {
            name: "LLM Rule".to_string(),
            description: None,
            scope: Some(RuleScope::Sender),
            scope_ref: Some("user@example.com".to_string()),
            rule_text: "Process emails".to_string(),
            enabled: None,
            metadata_json: None,
        };

        let create_response = create_llm_rule(State(state.clone()), Json(create_request))
            .await
            .into_response();
        let body_bytes = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: LlmRule = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(created.scope, RuleScope::Sender);
        // Note: Repository normalizes scope_ref to lowercase for sender scope
        assert!(created.scope_ref.is_some());

        // Change scope to Global
        let update_json = r#"{"scope": "global"}"#;
        let update_request: UpdateLlmRuleRequest = serde_json::from_str(update_json).unwrap();

        let update_response = update_llm_rule(
            State(state.clone()),
            Path(created.id.clone()),
            Json(update_request),
        )
        .await
        .into_response();

        assert_eq!(update_response.status(), StatusCode::OK);
        let body_bytes = to_bytes(update_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let updated: LlmRule = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(updated.scope, RuleScope::Global);
        assert_eq!(
            updated.scope_ref, None,
            "scope_ref should be cleared when scope is Global"
        );
    }

    // ========================================================================
    // Task 24: Tests for scope defaults in create requests
    // ========================================================================

    /// Test that POST creating a deterministic rule without scope field defaults to Global.
    #[tokio::test]
    async fn create_deterministic_rule_defaults_scope_to_global() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // Create a rule without specifying scope (parse from JSON to ensure it's absent)
        let create_json = r#"{
            "name": "No Scope Rule",
            "conditions_json": {"type": "sender_domain", "value": "example.com"},
            "action_type": "archive"
        }"#;
        let create_request: CreateDeterministicRuleRequest =
            serde_json::from_str(create_json).unwrap();
        assert!(
            create_request.scope.is_none(),
            "Scope should be None in request"
        );

        let response = create_deterministic_rule(State(state), Json(create_request))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let created: DeterministicRule = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(
            created.scope,
            RuleScope::Global,
            "Scope should default to Global"
        );
        assert_eq!(
            created.scope_ref, None,
            "scope_ref should be None for Global scope"
        );
    }

    /// Test that POST creating a deterministic rule with explicit scope uses that scope.
    #[tokio::test]
    async fn create_deterministic_rule_with_explicit_scope() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // Create a rule with explicit domain scope
        let create_json = r#"{
            "name": "Domain Scoped Rule",
            "scope": "domain",
            "scope_ref": "example.com",
            "conditions_json": {"type": "sender_domain", "value": "example.com"},
            "action_type": "archive"
        }"#;
        let create_request: CreateDeterministicRuleRequest =
            serde_json::from_str(create_json).unwrap();
        assert_eq!(
            create_request.scope,
            Some(RuleScope::Domain),
            "Scope should be Domain in request"
        );

        let response = create_deterministic_rule(State(state), Json(create_request))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let created: DeterministicRule = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(created.scope, RuleScope::Domain);
        assert_eq!(created.scope_ref, Some("example.com".to_string()));
    }

    /// Test that creating a rule with Global scope ignores scope_ref.
    #[tokio::test]
    async fn create_deterministic_rule_global_scope_ignores_scope_ref() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // Create a rule with Global scope but also provide scope_ref (should be ignored)
        let create_json = r#"{
            "name": "Global Rule With Ref",
            "scope": "global",
            "scope_ref": "should-be-ignored",
            "conditions_json": {"type": "sender_domain", "value": "example.com"},
            "action_type": "archive"
        }"#;
        let create_request: CreateDeterministicRuleRequest =
            serde_json::from_str(create_json).unwrap();

        let response = create_deterministic_rule(State(state), Json(create_request))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let created: DeterministicRule = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(created.scope, RuleScope::Global);
        assert_eq!(
            created.scope_ref, None,
            "scope_ref should be cleared for Global scope"
        );
    }

    /// Test that POST creating an LLM rule without scope defaults to Global.
    #[tokio::test]
    async fn create_llm_rule_defaults_scope_to_global() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // Create a rule without specifying scope
        let create_json = r#"{
            "name": "No Scope LLM Rule",
            "rule_text": "Archive newsletters"
        }"#;
        let create_request: CreateLlmRuleRequest = serde_json::from_str(create_json).unwrap();
        assert!(
            create_request.scope.is_none(),
            "Scope should be None in request"
        );

        let response = create_llm_rule(State(state), Json(create_request))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let created: LlmRule = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(
            created.scope,
            RuleScope::Global,
            "Scope should default to Global"
        );
        assert_eq!(created.scope_ref, None);
    }

    /// Test that POST creating an LLM rule with explicit scope uses that scope.
    #[tokio::test]
    async fn create_llm_rule_with_explicit_scope() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // Create a rule with explicit account scope
        let create_json = r#"{
            "name": "Account Scoped LLM Rule",
            "scope": "account",
            "scope_ref": "account-123",
            "rule_text": "Process emails for this account"
        }"#;
        let create_request: CreateLlmRuleRequest = serde_json::from_str(create_json).unwrap();
        assert_eq!(create_request.scope, Some(RuleScope::Account));

        let response = create_llm_rule(State(state), Json(create_request))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let created: LlmRule = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(created.scope, RuleScope::Account);
        assert_eq!(created.scope_ref, Some("account-123".to_string()));
    }

    // ========================================================================
    // Task 23: Tests for swap-priorities endpoint
    // ========================================================================

    /// Test successful swap of priorities between two deterministic rules.
    #[tokio::test]
    async fn swap_deterministic_rule_priorities_success() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // Create two rules with different priorities
        let create_request_a = CreateDeterministicRuleRequest {
            name: "Rule A".to_string(),
            description: None,
            scope: Some(RuleScope::Global),
            scope_ref: None,
            priority: Some(10),
            enabled: Some(true),
            conditions_json: json!({"type": "sender_domain", "value": "example.com"}),
            action_type: "archive".to_string(),
            action_parameters_json: None,
            safe_mode: None,
        };
        let response_a = create_deterministic_rule(State(state.clone()), Json(create_request_a))
            .await
            .into_response();
        let body_bytes = to_bytes(response_a.into_body(), usize::MAX).await.unwrap();
        let rule_a: DeterministicRule = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(rule_a.priority, 10);

        let create_request_b = CreateDeterministicRuleRequest {
            name: "Rule B".to_string(),
            description: None,
            scope: Some(RuleScope::Global),
            scope_ref: None,
            priority: Some(20),
            enabled: Some(true),
            conditions_json: json!({"type": "sender_domain", "value": "other.com"}),
            action_type: "archive".to_string(),
            action_parameters_json: None,
            safe_mode: None,
        };
        let response_b = create_deterministic_rule(State(state.clone()), Json(create_request_b))
            .await
            .into_response();
        let body_bytes = to_bytes(response_b.into_body(), usize::MAX).await.unwrap();
        let rule_b: DeterministicRule = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(rule_b.priority, 20);

        // Swap priorities
        let swap_request = SwapPrioritiesRequest {
            rule_a_id: rule_a.id.clone(),
            rule_b_id: rule_b.id.clone(),
        };
        let swap_response =
            swap_deterministic_rule_priorities(State(state.clone()), Json(swap_request))
                .await
                .into_response();

        assert_eq!(swap_response.status(), StatusCode::OK);
        let body_bytes = to_bytes(swap_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let swap_result: SwapPrioritiesResponse = serde_json::from_slice(&body_bytes).unwrap();
        assert!(swap_result.success);

        // Verify priorities were swapped
        let get_response_a = get_deterministic_rule(State(state.clone()), Path(rule_a.id.clone()))
            .await
            .into_response();
        let body_bytes = to_bytes(get_response_a.into_body(), usize::MAX)
            .await
            .unwrap();
        let updated_rule_a: DeterministicRule = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(
            updated_rule_a.priority, 20,
            "Rule A should have Rule B's original priority"
        );

        let get_response_b = get_deterministic_rule(State(state.clone()), Path(rule_b.id.clone()))
            .await
            .into_response();
        let body_bytes = to_bytes(get_response_b.into_body(), usize::MAX)
            .await
            .unwrap();
        let updated_rule_b: DeterministicRule = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(
            updated_rule_b.priority, 10,
            "Rule B should have Rule A's original priority"
        );
    }

    /// Test swap with non-existent rule A returns 404.
    #[tokio::test]
    async fn swap_deterministic_rule_priorities_rule_a_not_found() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // Create only rule B
        let create_request_b = CreateDeterministicRuleRequest {
            name: "Rule B".to_string(),
            description: None,
            scope: Some(RuleScope::Global),
            scope_ref: None,
            priority: Some(20),
            enabled: Some(true),
            conditions_json: json!({"type": "sender_domain", "value": "example.com"}),
            action_type: "archive".to_string(),
            action_parameters_json: None,
            safe_mode: None,
        };
        let response_b = create_deterministic_rule(State(state.clone()), Json(create_request_b))
            .await
            .into_response();
        let body_bytes = to_bytes(response_b.into_body(), usize::MAX).await.unwrap();
        let rule_b: DeterministicRule = serde_json::from_slice(&body_bytes).unwrap();

        // Try to swap with non-existent rule A
        let swap_request = SwapPrioritiesRequest {
            rule_a_id: "nonexistent-rule-id".to_string(),
            rule_b_id: rule_b.id.clone(),
        };
        let swap_response =
            swap_deterministic_rule_priorities(State(state.clone()), Json(swap_request))
                .await
                .into_response();

        assert_eq!(swap_response.status(), StatusCode::NOT_FOUND);
    }

    /// Test swap with non-existent rule B returns 404.
    #[tokio::test]
    async fn swap_deterministic_rule_priorities_rule_b_not_found() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // Create only rule A
        let create_request_a = CreateDeterministicRuleRequest {
            name: "Rule A".to_string(),
            description: None,
            scope: Some(RuleScope::Global),
            scope_ref: None,
            priority: Some(10),
            enabled: Some(true),
            conditions_json: json!({"type": "sender_domain", "value": "example.com"}),
            action_type: "archive".to_string(),
            action_parameters_json: None,
            safe_mode: None,
        };
        let response_a = create_deterministic_rule(State(state.clone()), Json(create_request_a))
            .await
            .into_response();
        let body_bytes = to_bytes(response_a.into_body(), usize::MAX).await.unwrap();
        let rule_a: DeterministicRule = serde_json::from_slice(&body_bytes).unwrap();

        // Try to swap with non-existent rule B
        let swap_request = SwapPrioritiesRequest {
            rule_a_id: rule_a.id.clone(),
            rule_b_id: "nonexistent-rule-id".to_string(),
        };
        let swap_response =
            swap_deterministic_rule_priorities(State(state.clone()), Json(swap_request))
                .await
                .into_response();

        assert_eq!(swap_response.status(), StatusCode::NOT_FOUND);
    }

    /// Test swap with empty rule_a_id returns 400 bad request.
    #[tokio::test]
    async fn swap_deterministic_rule_priorities_empty_rule_a_id() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        let swap_request = SwapPrioritiesRequest {
            rule_a_id: "".to_string(),
            rule_b_id: "some-id".to_string(),
        };
        let swap_response =
            swap_deterministic_rule_priorities(State(state.clone()), Json(swap_request))
                .await
                .into_response();

        assert_eq!(swap_response.status(), StatusCode::BAD_REQUEST);
    }

    /// Test swap with empty rule_b_id returns 400 bad request.
    #[tokio::test]
    async fn swap_deterministic_rule_priorities_empty_rule_b_id() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        let swap_request = SwapPrioritiesRequest {
            rule_a_id: "some-id".to_string(),
            rule_b_id: "  ".to_string(), // whitespace only
        };
        let swap_response =
            swap_deterministic_rule_priorities(State(state.clone()), Json(swap_request))
                .await
                .into_response();

        assert_eq!(swap_response.status(), StatusCode::BAD_REQUEST);
    }

    /// Test swap with same rule ID for both returns 400 bad request.
    #[tokio::test]
    async fn swap_deterministic_rule_priorities_same_rule_id() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // Create a rule
        let create_request = CreateDeterministicRuleRequest {
            name: "Test Rule".to_string(),
            description: None,
            scope: Some(RuleScope::Global),
            scope_ref: None,
            priority: Some(10),
            enabled: Some(true),
            conditions_json: json!({"type": "sender_domain", "value": "example.com"}),
            action_type: "archive".to_string(),
            action_parameters_json: None,
            safe_mode: None,
        };
        let response = create_deterministic_rule(State(state.clone()), Json(create_request))
            .await
            .into_response();
        let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let rule: DeterministicRule = serde_json::from_slice(&body_bytes).unwrap();

        // Try to swap with itself
        let swap_request = SwapPrioritiesRequest {
            rule_a_id: rule.id.clone(),
            rule_b_id: rule.id.clone(),
        };
        let swap_response =
            swap_deterministic_rule_priorities(State(state.clone()), Json(swap_request))
                .await
                .into_response();

        assert_eq!(swap_response.status(), StatusCode::BAD_REQUEST);
    }

    /// Test that swap is atomic - verifies both priorities are updated correctly.
    #[tokio::test]
    async fn swap_deterministic_rule_priorities_is_atomic() {
        let (db, _dir) = setup_db().await;
        let state = crate::AppState { db: db.clone() };

        // Create three rules with different priorities
        let mut rule_ids = Vec::new();
        for (name, priority) in [("Rule 1", 10), ("Rule 2", 20), ("Rule 3", 30)] {
            let create_request = CreateDeterministicRuleRequest {
                name: name.to_string(),
                description: None,
                scope: Some(RuleScope::Global),
                scope_ref: None,
                priority: Some(priority),
                enabled: Some(true),
                conditions_json: json!({"type": "sender_domain", "value": "example.com"}),
                action_type: "archive".to_string(),
                action_parameters_json: None,
                safe_mode: None,
            };
            let response = create_deterministic_rule(State(state.clone()), Json(create_request))
                .await
                .into_response();
            let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let rule: DeterministicRule = serde_json::from_slice(&body_bytes).unwrap();
            rule_ids.push(rule.id);
        }

        // Swap first and third rules
        let swap_request = SwapPrioritiesRequest {
            rule_a_id: rule_ids[0].clone(),
            rule_b_id: rule_ids[2].clone(),
        };
        let swap_response =
            swap_deterministic_rule_priorities(State(state.clone()), Json(swap_request))
                .await
                .into_response();

        assert_eq!(swap_response.status(), StatusCode::OK);

        // Verify the list is correctly ordered
        let list_response = list_deterministic_rules(State(state.clone()))
            .await
            .into_response();
        let body_bytes = to_bytes(list_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let rules: Vec<DeterministicRule> = serde_json::from_slice(&body_bytes).unwrap();

        // Should be ordered by priority ASC: Rule 3 (10), Rule 2 (20), Rule 1 (30)
        assert_eq!(rules[0].name, "Rule 3");
        assert_eq!(rules[0].priority, 10);
        assert_eq!(rules[1].name, "Rule 2");
        assert_eq!(rules[1].priority, 20);
        assert_eq!(rules[2].name, "Rule 1");
        assert_eq!(rules[2].priority, 30);
    }
}
