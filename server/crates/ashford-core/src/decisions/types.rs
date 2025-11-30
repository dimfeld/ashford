use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionSource {
    Llm,
    Deterministic,
}

impl DecisionSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            DecisionSource::Llm => "llm",
            DecisionSource::Deterministic => "deterministic",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "llm" => Some(Self::Llm),
            "deterministic" => Some(Self::Deterministic),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    Queued,
    Executing,
    Completed,
    Failed,
    Canceled,
    Rejected,
    ApprovedPending,
}

impl ActionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionStatus::Queued => "queued",
            ActionStatus::Executing => "executing",
            ActionStatus::Completed => "completed",
            ActionStatus::Failed => "failed",
            ActionStatus::Canceled => "canceled",
            ActionStatus::Rejected => "rejected",
            ActionStatus::ApprovedPending => "approved_pending",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "queued" => Some(Self::Queued),
            "executing" => Some(Self::Executing),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "canceled" => Some(Self::Canceled),
            "rejected" => Some(Self::Rejected),
            "approved_pending" => Some(Self::ApprovedPending),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionLinkRelationType {
    UndoOf,
    ApprovalFor,
    Spawned,
    Related,
}

impl ActionLinkRelationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionLinkRelationType::UndoOf => "undo_of",
            ActionLinkRelationType::ApprovalFor => "approval_for",
            ActionLinkRelationType::Spawned => "spawned",
            ActionLinkRelationType::Related => "related",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "undo_of" => Some(Self::UndoOf),
            "approval_for" => Some(Self::ApprovalFor),
            "spawned" => Some(Self::Spawned),
            "related" => Some(Self::Related),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Decision {
    pub id: String,
    pub org_id: i64,
    pub user_id: i64,
    pub account_id: String,
    pub message_id: String,
    pub source: DecisionSource,
    pub decision_json: Value,
    pub action_type: Option<String>,
    pub confidence: Option<f64>,
    pub needs_approval: bool,
    pub rationale: Option<String>,
    pub telemetry_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewDecision {
    pub org_id: i64,
    pub user_id: i64,
    pub account_id: String,
    pub message_id: String,
    pub source: DecisionSource,
    pub decision_json: Value,
    pub action_type: Option<String>,
    pub confidence: Option<f64>,
    pub needs_approval: bool,
    pub rationale: Option<String>,
    pub telemetry_json: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub org_id: i64,
    pub user_id: i64,
    pub account_id: String,
    pub message_id: String,
    pub decision_id: Option<String>,
    pub action_type: String,
    pub parameters_json: Value,
    pub status: ActionStatus,
    pub error_message: Option<String>,
    pub executed_at: Option<DateTime<Utc>>,
    pub undo_hint_json: Value,
    pub trace_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewAction {
    pub org_id: i64,
    pub user_id: i64,
    pub account_id: String,
    pub message_id: String,
    pub decision_id: Option<String>,
    pub action_type: String,
    pub parameters_json: Value,
    pub status: ActionStatus,
    pub error_message: Option<String>,
    pub executed_at: Option<DateTime<Utc>>,
    pub undo_hint_json: Value,
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionLink {
    pub id: String,
    pub cause_action_id: String,
    pub effect_action_id: String,
    pub relation_type: ActionLinkRelationType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewActionLink {
    pub cause_action_id: String,
    pub effect_action_id: String,
    pub relation_type: ActionLinkRelationType,
}
