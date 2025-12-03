use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleScope {
    Global,
    Account,
    Sender,
    Domain,
}

impl RuleScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuleScope::Global => "global",
            RuleScope::Account => "account",
            RuleScope::Sender => "sender",
            RuleScope::Domain => "domain",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "global" => Some(Self::Global),
            "account" => Some(Self::Account),
            "sender" => Some(Self::Sender),
            "domain" => Some(Self::Domain),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafeMode {
    Default,
    AlwaysSafe,
    DangerousOverride,
}

impl SafeMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            SafeMode::Default => "default",
            SafeMode::AlwaysSafe => "always_safe",
            SafeMode::DangerousOverride => "dangerous_override",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "default" => Some(Self::Default),
            "always_safe" => Some(Self::AlwaysSafe),
            "dangerous_override" => Some(Self::DangerousOverride),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RulesChatRole {
    User,
    Assistant,
    System,
}

impl RulesChatRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            RulesChatRole::User => "user",
            RulesChatRole::Assistant => "assistant",
            RulesChatRole::System => "system",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "user" => Some(Self::User),
            "assistant" => Some(Self::Assistant),
            "system" => Some(Self::System),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeterministicRule {
    pub id: String,
    pub org_id: i64,
    pub user_id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub scope: RuleScope,
    pub scope_ref: Option<String>,
    pub priority: i64,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
    pub conditions_json: Value,
    pub action_type: String,
    pub action_parameters_json: Value,
    pub safe_mode: SafeMode,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewDeterministicRule {
    pub org_id: i64,
    pub user_id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub scope: RuleScope,
    pub scope_ref: Option<String>,
    pub priority: i64,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
    pub conditions_json: Value,
    pub action_type: String,
    pub action_parameters_json: Value,
    pub safe_mode: SafeMode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmRule {
    pub id: String,
    pub org_id: i64,
    pub user_id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub scope: RuleScope,
    pub scope_ref: Option<String>,
    pub rule_text: String,
    pub enabled: bool,
    pub metadata_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewLlmRule {
    pub org_id: i64,
    pub user_id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub scope: RuleScope,
    pub scope_ref: Option<String>,
    pub rule_text: String,
    pub enabled: bool,
    pub metadata_json: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Direction {
    pub id: String,
    pub org_id: i64,
    pub user_id: Option<i64>,
    pub content: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewDirection {
    pub org_id: i64,
    pub user_id: Option<i64>,
    pub content: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RulesChatSession {
    pub id: String,
    pub org_id: i64,
    pub user_id: i64,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewRulesChatSession {
    pub org_id: i64,
    pub user_id: i64,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RulesChatMessage {
    pub id: String,
    pub org_id: i64,
    pub user_id: i64,
    pub session_id: String,
    pub role: RulesChatRole,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewRulesChatMessage {
    pub org_id: i64,
    pub user_id: i64,
    pub session_id: String,
    pub role: RulesChatRole,
    pub content: String,
}
