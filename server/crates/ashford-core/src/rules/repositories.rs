use chrono::{DateTime, SecondsFormat, Utc};
use libsql::{Row, params};
use std::borrow::Cow;
use thiserror::Error;
use uuid::Uuid;

use crate::db::{Database, DbError};

use super::types::{
    DeterministicRule, Direction, LlmRule, NewDeterministicRule, NewDirection, NewLlmRule,
    NewRulesChatMessage, NewRulesChatSession, RuleScope, RulesChatMessage, RulesChatRole,
    RulesChatSession, SafeMode,
};

const DETERMINISTIC_RULE_COLUMNS: &str = "id, name, description, scope, scope_ref, priority, enabled, disabled_reason, conditions_json, action_type, action_parameters_json, safe_mode, created_at, updated_at, org_id, user_id";
const LLM_RULE_COLUMNS: &str = "id, name, description, scope, scope_ref, rule_text, enabled, metadata_json, created_at, updated_at, org_id, user_id";
const DIRECTION_COLUMNS: &str = "id, content, enabled, created_at, updated_at, org_id, user_id";
const RULES_CHAT_SESSION_COLUMNS: &str = "id, title, created_at, updated_at, org_id, user_id";
const RULES_CHAT_MESSAGE_COLUMNS: &str =
    "id, session_id, role, content, created_at, org_id, user_id";

#[derive(Debug, Error)]
pub enum DeterministicRuleError {
    #[error("database error: {0}")]
    Database(#[from] DbError),
    #[error("sql error: {0}")]
    Sql(#[from] libsql::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("datetime parse error: {0}")]
    DateTimeParse(#[from] chrono::ParseError),
    #[error("deterministic rule not found: {0}")]
    NotFound(String),
    #[error("invalid scope value {0}")]
    InvalidScope(String),
    #[error("invalid safe_mode value {0}")]
    InvalidSafeMode(String),
}

#[derive(Debug, Error)]
pub enum LlmRuleError {
    #[error("database error: {0}")]
    Database(#[from] DbError),
    #[error("sql error: {0}")]
    Sql(#[from] libsql::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("datetime parse error: {0}")]
    DateTimeParse(#[from] chrono::ParseError),
    #[error("llm rule not found: {0}")]
    NotFound(String),
    #[error("invalid scope value {0}")]
    InvalidScope(String),
}

#[derive(Debug, Error)]
pub enum DirectionError {
    #[error("database error: {0}")]
    Database(#[from] DbError),
    #[error("sql error: {0}")]
    Sql(#[from] libsql::Error),
    #[error("datetime parse error: {0}")]
    DateTimeParse(#[from] chrono::ParseError),
    #[error("direction not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Error)]
pub enum RulesChatSessionError {
    #[error("database error: {0}")]
    Database(#[from] DbError),
    #[error("sql error: {0}")]
    Sql(#[from] libsql::Error),
    #[error("datetime parse error: {0}")]
    DateTimeParse(#[from] chrono::ParseError),
    #[error("rules chat session not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Error)]
pub enum RulesChatMessageError {
    #[error("database error: {0}")]
    Database(#[from] DbError),
    #[error("sql error: {0}")]
    Sql(#[from] libsql::Error),
    #[error("datetime parse error: {0}")]
    DateTimeParse(#[from] chrono::ParseError),
    #[error("invalid role value {0}")]
    InvalidRole(String),
    #[error("rules chat message not found: {0}")]
    NotFound(String),
}

#[derive(Clone)]
pub struct DeterministicRuleRepository {
    db: Database,
}

impl DeterministicRuleRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn create(
        &self,
        new_rule: NewDeterministicRule,
    ) -> Result<DeterministicRule, DeterministicRuleError> {
        let id = Uuid::new_v4().to_string();
        let now = now_rfc3339();
        let conditions_json = serde_json::to_string(&new_rule.conditions_json)?;
        let action_parameters_json = serde_json::to_string(&new_rule.action_parameters_json)?;
        let enabled = new_rule.enabled as i64;
        let scope_ref = normalize_scope_ref(&new_rule.scope, &new_rule.scope_ref);
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "INSERT INTO deterministic_rules (
                        id, name, description, scope, scope_ref, priority, enabled, disabled_reason, conditions_json, action_type, action_parameters_json, safe_mode, created_at, updated_at, org_id, user_id
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13, ?14, ?15)
                    RETURNING {DETERMINISTIC_RULE_COLUMNS}"
                ),
                params![
                    id,
                    new_rule.name,
                    new_rule.description,
                    new_rule.scope.as_str(),
                    scope_ref,
                    new_rule.priority,
                    enabled,
                    new_rule.disabled_reason,
                    conditions_json,
                    new_rule.action_type,
                    action_parameters_json,
                    new_rule.safe_mode.as_str(),
                    now,
                    new_rule.org_id,
                    new_rule.user_id
                ],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_deterministic_rule(row),
            None => Err(DeterministicRuleError::NotFound("insert failed".into())),
        }
    }

    pub async fn get_by_id(
        &self,
        org_id: i64,
        user_id: i64,
        id: &str,
    ) -> Result<DeterministicRule, DeterministicRuleError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {DETERMINISTIC_RULE_COLUMNS}
                     FROM deterministic_rules
                     WHERE id = ?1
                       AND org_id = ?2
                       AND (user_id IS NULL OR user_id = ?3)"
                ),
                params![id, org_id, user_id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_deterministic_rule(row),
            None => Err(DeterministicRuleError::NotFound(id.to_string())),
        }
    }

    pub async fn list_all(
        &self,
        org_id: i64,
        user_id: i64,
    ) -> Result<Vec<DeterministicRule>, DeterministicRuleError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {DETERMINISTIC_RULE_COLUMNS}
                     FROM deterministic_rules
                     WHERE org_id = ?1 AND (user_id IS NULL OR user_id = ?2)
                     ORDER BY priority ASC, created_at"
                ),
                params![org_id, user_id],
            )
            .await?;

        let mut rules = Vec::new();
        while let Some(row) = rows.next().await? {
            rules.push(row_to_deterministic_rule(row)?);
        }
        Ok(rules)
    }

    pub async fn list_enabled_by_scope(
        &self,
        org_id: i64,
        user_id: i64,
        scope: RuleScope,
        scope_ref: Option<&str>,
    ) -> Result<Vec<DeterministicRule>, DeterministicRuleError> {
        let conn = self.db.connection().await?;
        let scope_value = scope.as_str();
        let is_case_insensitive_scope = matches!(scope, RuleScope::Domain | RuleScope::Sender);
        let normalized_ref: Option<Cow<'_, str>> = match (is_case_insensitive_scope, scope_ref) {
            (true, Some(reference)) => Some(Cow::Owned(reference.to_lowercase())),
            (false, Some(reference)) => Some(Cow::Borrowed(reference)),
            (_, None) => None,
        };

        let mut rows = if let Some(reference) = normalized_ref.as_deref() {
            let scope_ref_clause = if is_case_insensitive_scope {
                "LOWER(scope_ref) = ?4"
            } else {
                "scope_ref = ?4"
            };

            conn.query(
                &format!(
                    "SELECT {DETERMINISTIC_RULE_COLUMNS}
                     FROM deterministic_rules
                     WHERE org_id = ?1
                       AND (user_id IS NULL OR user_id = ?2)
                       AND enabled = 1
                       AND scope = ?3
                       AND {scope_ref_clause}
                     ORDER BY priority ASC, created_at"
                ),
                params![org_id, user_id, scope_value, reference],
            )
            .await?
        } else {
            conn.query(
                &format!(
                    "SELECT {DETERMINISTIC_RULE_COLUMNS}
                     FROM deterministic_rules
                     WHERE org_id = ?1
                       AND (user_id IS NULL OR user_id = ?2)
                       AND enabled = 1
                       AND scope = ?3
                       AND scope_ref IS NULL
                     ORDER BY priority ASC, created_at"
                ),
                params![org_id, user_id, scope_value],
            )
            .await?
        };

        let mut rules = Vec::new();
        while let Some(row) = rows.next().await? {
            rules.push(row_to_deterministic_rule(row)?);
        }
        Ok(rules)
    }

    pub async fn update(
        &self,
        org_id: i64,
        user_id: i64,
        id: &str,
        updated: NewDeterministicRule,
    ) -> Result<DeterministicRule, DeterministicRuleError> {
        let now = now_rfc3339();
        let conditions_json = serde_json::to_string(&updated.conditions_json)?;
        let action_parameters_json = serde_json::to_string(&updated.action_parameters_json)?;
        let enabled = updated.enabled as i64;
        let scope_ref = normalize_scope_ref(&updated.scope, &updated.scope_ref);
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "UPDATE deterministic_rules
                     SET name = ?1,
                         description = ?2,
                         scope = ?3,
                         scope_ref = ?4,
                         priority = ?5,
                         enabled = ?6,
                         disabled_reason = ?7,
                         conditions_json = ?8,
                         action_type = ?9,
                         action_parameters_json = ?10,
                         safe_mode = ?11,
                         user_id = ?12,
                         updated_at = ?13
                     WHERE id = ?14
                       AND org_id = ?15
                       AND (user_id IS NULL OR user_id = ?16)
                     RETURNING {DETERMINISTIC_RULE_COLUMNS}"
                ),
                params![
                    updated.name,
                    updated.description,
                    updated.scope.as_str(),
                    scope_ref,
                    updated.priority,
                    enabled,
                    updated.disabled_reason,
                    conditions_json,
                    updated.action_type,
                    action_parameters_json,
                    updated.safe_mode.as_str(),
                    updated.user_id,
                    now,
                    id,
                    org_id,
                    user_id
                ],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_deterministic_rule(row),
            None => Err(DeterministicRuleError::NotFound(id.to_string())),
        }
    }

    pub async fn delete(
        &self,
        org_id: i64,
        user_id: i64,
        id: &str,
    ) -> Result<(), DeterministicRuleError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                "DELETE FROM deterministic_rules WHERE id = ?1 AND org_id = ?2 AND (user_id IS NULL OR user_id = ?3) RETURNING id",
                params![id, org_id, user_id],
            )
            .await?;

        match rows.next().await? {
            Some(_) => Ok(()),
            None => Err(DeterministicRuleError::NotFound(id.to_string())),
        }
    }

    /// Disable a rule and set a reason explaining why it was disabled.
    /// This sets enabled=false and disabled_reason to the provided reason.
    pub async fn disable_rule_with_reason(
        &self,
        org_id: i64,
        user_id: i64,
        id: &str,
        reason: &str,
    ) -> Result<DeterministicRule, DeterministicRuleError> {
        let now = now_rfc3339();
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "UPDATE deterministic_rules
                     SET enabled = 0,
                         disabled_reason = ?1,
                         updated_at = ?2
                     WHERE id = ?3
                       AND org_id = ?4
                       AND (user_id IS NULL OR user_id = ?5)
                     RETURNING {DETERMINISTIC_RULE_COLUMNS}"
                ),
                params![reason, now, id, org_id, user_id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_deterministic_rule(row),
            None => Err(DeterministicRuleError::NotFound(id.to_string())),
        }
    }

    /// Find rules that reference a label by provider_label_id in their conditions or action parameters.
    /// This searches for the label ID in:
    /// 1. LabelPresent conditions (conditions_json contains the label ID)
    /// 2. apply_label actions (action_parameters_json contains the label ID)
    ///
    /// The search uses quoted JSON string matching (e.g., `"Label_1"`) to avoid false positives
    /// where a label ID is a prefix of another (e.g., searching for "Label_1" won't match "Label_10").
    pub async fn find_rules_referencing_label(
        &self,
        org_id: i64,
        user_id: i64,
        label_provider_id: &str,
    ) -> Result<Vec<DeterministicRule>, DeterministicRuleError> {
        let conn = self.db.connection().await?;
        // Search for the label ID as a quoted JSON string value
        // This prevents false positives like "Label_1" matching "Label_10"
        let search_pattern = format!("%\"{}\"%", label_provider_id);
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {DETERMINISTIC_RULE_COLUMNS}
                     FROM deterministic_rules
                     WHERE org_id = ?1
                       AND (user_id IS NULL OR user_id = ?2)
                       AND (conditions_json LIKE ?3 OR action_parameters_json LIKE ?3)
                     ORDER BY priority ASC, created_at"
                ),
                params![org_id, user_id, search_pattern],
            )
            .await?;

        let mut rules = Vec::new();
        while let Some(row) = rows.next().await? {
            rules.push(row_to_deterministic_rule(row)?);
        }
        Ok(rules)
    }
}

fn normalize_scope_ref(scope: &RuleScope, scope_ref: &Option<String>) -> Option<String> {
    match scope {
        RuleScope::Domain | RuleScope::Sender => {
            scope_ref.as_ref().map(|value| value.to_lowercase())
        }
        _ => scope_ref.clone(),
    }
}

#[derive(Clone)]
pub struct LlmRuleRepository {
    db: Database,
}

impl LlmRuleRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn create(&self, new_rule: NewLlmRule) -> Result<LlmRule, LlmRuleError> {
        let id = Uuid::new_v4().to_string();
        let now = now_rfc3339();
        let metadata_json = serde_json::to_string(&new_rule.metadata_json)?;
        let enabled = new_rule.enabled as i64;

        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "INSERT INTO llm_rules (
                        id, name, description, scope, scope_ref, rule_text, enabled, metadata_json, created_at, updated_at, org_id, user_id
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9, ?10, ?11)
                    RETURNING {LLM_RULE_COLUMNS}"
                ),
                params![
                    id,
                    new_rule.name,
                    new_rule.description,
                    new_rule.scope.as_str(),
                    new_rule.scope_ref,
                    new_rule.rule_text,
                    enabled,
                    metadata_json,
                    now,
                    new_rule.org_id,
                    new_rule.user_id
                ],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_llm_rule(row),
            None => Err(LlmRuleError::NotFound("insert failed".into())),
        }
    }

    pub async fn get_by_id(
        &self,
        org_id: i64,
        user_id: i64,
        id: &str,
    ) -> Result<LlmRule, LlmRuleError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {LLM_RULE_COLUMNS}
                     FROM llm_rules
                     WHERE id = ?1
                       AND org_id = ?2
                       AND (user_id IS NULL OR user_id = ?3)"
                ),
                params![id, org_id, user_id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_llm_rule(row),
            None => Err(LlmRuleError::NotFound(id.to_string())),
        }
    }

    pub async fn list_all(&self, org_id: i64, user_id: i64) -> Result<Vec<LlmRule>, LlmRuleError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {LLM_RULE_COLUMNS}
                     FROM llm_rules
                     WHERE org_id = ?1 AND (user_id IS NULL OR user_id = ?2)
                     ORDER BY created_at"
                ),
                params![org_id, user_id],
            )
            .await?;

        let mut rules = Vec::new();
        while let Some(row) = rows.next().await? {
            rules.push(row_to_llm_rule(row)?);
        }
        Ok(rules)
    }

    pub async fn list_enabled_by_scope(
        &self,
        org_id: i64,
        user_id: i64,
        scope: RuleScope,
        scope_ref: Option<&str>,
    ) -> Result<Vec<LlmRule>, LlmRuleError> {
        let conn = self.db.connection().await?;
        let scope_value = scope.as_str();

        let mut rows = if let Some(reference) = scope_ref {
            conn.query(
                &format!(
                    "SELECT {LLM_RULE_COLUMNS}
                     FROM llm_rules
                     WHERE org_id = ?1
                       AND (user_id IS NULL OR user_id = ?2)
                       AND enabled = 1
                       AND scope = ?3
                       AND scope_ref = ?4
                     ORDER BY created_at"
                ),
                params![org_id, user_id, scope_value, reference],
            )
            .await?
        } else {
            conn.query(
                &format!(
                    "SELECT {LLM_RULE_COLUMNS}
                     FROM llm_rules
                     WHERE org_id = ?1
                       AND (user_id IS NULL OR user_id = ?2)
                       AND enabled = 1
                       AND scope = ?3
                       AND scope_ref IS NULL
                     ORDER BY created_at"
                ),
                params![org_id, user_id, scope_value],
            )
            .await?
        };

        let mut rules = Vec::new();
        while let Some(row) = rows.next().await? {
            rules.push(row_to_llm_rule(row)?);
        }
        Ok(rules)
    }

    pub async fn update(
        &self,
        org_id: i64,
        user_id: i64,
        id: &str,
        updated: NewLlmRule,
    ) -> Result<LlmRule, LlmRuleError> {
        let now = now_rfc3339();
        let metadata_json = serde_json::to_string(&updated.metadata_json)?;
        let enabled = updated.enabled as i64;

        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "UPDATE llm_rules
                     SET name = ?1,
                         description = ?2,
                         scope = ?3,
                         scope_ref = ?4,
                         rule_text = ?5,
                         enabled = ?6,
                         metadata_json = ?7,
                         user_id = ?8,
                         updated_at = ?9
                     WHERE id = ?10
                       AND org_id = ?11
                       AND (user_id IS NULL OR user_id = ?12)
                     RETURNING {LLM_RULE_COLUMNS}"
                ),
                params![
                    updated.name,
                    updated.description,
                    updated.scope.as_str(),
                    updated.scope_ref,
                    updated.rule_text,
                    enabled,
                    metadata_json,
                    updated.user_id,
                    now,
                    id,
                    org_id,
                    user_id
                ],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_llm_rule(row),
            None => Err(LlmRuleError::NotFound(id.to_string())),
        }
    }

    pub async fn delete(&self, org_id: i64, user_id: i64, id: &str) -> Result<(), LlmRuleError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                "DELETE FROM llm_rules WHERE id = ?1 AND org_id = ?2 AND (user_id IS NULL OR user_id = ?3) RETURNING id",
                params![id, org_id, user_id],
            )
            .await?;

        match rows.next().await? {
            Some(_) => Ok(()),
            None => Err(LlmRuleError::NotFound(id.to_string())),
        }
    }
}

#[derive(Clone)]
pub struct DirectionsRepository {
    db: Database,
}

impl DirectionsRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn create(&self, new_direction: NewDirection) -> Result<Direction, DirectionError> {
        let id = Uuid::new_v4().to_string();
        let now = now_rfc3339();
        let enabled = new_direction.enabled as i64;

        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "INSERT INTO directions (id, content, enabled, created_at, updated_at, org_id, user_id)
                     VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?6)
                     RETURNING {DIRECTION_COLUMNS}"
                ),
                params![
                    id,
                    new_direction.content,
                    enabled,
                    now,
                    new_direction.org_id,
                    new_direction.user_id
                ],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_direction(row),
            None => Err(DirectionError::NotFound("insert failed".into())),
        }
    }

    pub async fn get_by_id(
        &self,
        org_id: i64,
        user_id: i64,
        id: &str,
    ) -> Result<Direction, DirectionError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {DIRECTION_COLUMNS}
                     FROM directions
                     WHERE id = ?1
                       AND org_id = ?2
                       AND (user_id IS NULL OR user_id = ?3)"
                ),
                params![id, org_id, user_id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_direction(row),
            None => Err(DirectionError::NotFound(id.to_string())),
        }
    }

    pub async fn list_all(
        &self,
        org_id: i64,
        user_id: i64,
    ) -> Result<Vec<Direction>, DirectionError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {DIRECTION_COLUMNS}
                     FROM directions
                     WHERE org_id = ?1 AND (user_id IS NULL OR user_id = ?2)
                     ORDER BY created_at"
                ),
                params![org_id, user_id],
            )
            .await?;

        let mut directions = Vec::new();
        while let Some(row) = rows.next().await? {
            directions.push(row_to_direction(row)?);
        }
        Ok(directions)
    }

    pub async fn list_enabled(
        &self,
        org_id: i64,
        user_id: i64,
    ) -> Result<Vec<Direction>, DirectionError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {DIRECTION_COLUMNS}
                     FROM directions
                     WHERE org_id = ?1
                       AND (user_id IS NULL OR user_id = ?2)
                       AND enabled = 1
                     ORDER BY created_at"
                ),
                params![org_id, user_id],
            )
            .await?;

        let mut directions = Vec::new();
        while let Some(row) = rows.next().await? {
            directions.push(row_to_direction(row)?);
        }
        Ok(directions)
    }

    pub async fn update(
        &self,
        org_id: i64,
        user_id: i64,
        id: &str,
        updated: NewDirection,
    ) -> Result<Direction, DirectionError> {
        let now = now_rfc3339();
        let enabled = updated.enabled as i64;
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "UPDATE directions
                     SET content = ?1,
                         enabled = ?2,
                         user_id = ?3,
                         updated_at = ?4
                     WHERE id = ?5
                       AND org_id = ?6
                       AND (user_id IS NULL OR user_id = ?7)
                     RETURNING {DIRECTION_COLUMNS}"
                ),
                params![
                    updated.content,
                    enabled,
                    updated.user_id,
                    now,
                    id,
                    org_id,
                    user_id
                ],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_direction(row),
            None => Err(DirectionError::NotFound(id.to_string())),
        }
    }

    pub async fn delete(&self, org_id: i64, user_id: i64, id: &str) -> Result<(), DirectionError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                "DELETE FROM directions WHERE id = ?1 AND org_id = ?2 AND (user_id IS NULL OR user_id = ?3) RETURNING id",
                params![id, org_id, user_id],
            )
            .await?;

        match rows.next().await? {
            Some(_) => Ok(()),
            None => Err(DirectionError::NotFound(id.to_string())),
        }
    }
}

#[derive(Clone)]
pub struct RulesChatSessionRepository {
    db: Database,
}

impl RulesChatSessionRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn create(
        &self,
        new_session: NewRulesChatSession,
    ) -> Result<RulesChatSession, RulesChatSessionError> {
        let id = Uuid::new_v4().to_string();
        let now = now_rfc3339();
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "INSERT INTO rules_chat_sessions (id, title, created_at, updated_at, org_id, user_id)
                     VALUES (?1, ?2, ?3, ?3, ?4, ?5)
                     RETURNING {RULES_CHAT_SESSION_COLUMNS}"
                ),
                params![id, new_session.title, now, new_session.org_id, new_session.user_id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_rules_chat_session(row),
            None => Err(RulesChatSessionError::NotFound("insert failed".into())),
        }
    }

    pub async fn get_by_id(
        &self,
        org_id: i64,
        user_id: i64,
        id: &str,
    ) -> Result<RulesChatSession, RulesChatSessionError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {RULES_CHAT_SESSION_COLUMNS}
                     FROM rules_chat_sessions
                     WHERE id = ?1 AND org_id = ?2 AND user_id = ?3"
                ),
                params![id, org_id, user_id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_rules_chat_session(row),
            None => Err(RulesChatSessionError::NotFound(id.to_string())),
        }
    }

    pub async fn list_for_user(
        &self,
        org_id: i64,
        user_id: i64,
    ) -> Result<Vec<RulesChatSession>, RulesChatSessionError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {RULES_CHAT_SESSION_COLUMNS}
                     FROM rules_chat_sessions
                     WHERE org_id = ?1 AND user_id = ?2
                     ORDER BY updated_at DESC"
                ),
                params![org_id, user_id],
            )
            .await?;

        let mut sessions = Vec::new();
        while let Some(row) = rows.next().await? {
            sessions.push(row_to_rules_chat_session(row)?);
        }
        Ok(sessions)
    }
}

#[derive(Clone)]
pub struct RulesChatMessageRepository {
    db: Database,
}

impl RulesChatMessageRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn create(
        &self,
        new_message: NewRulesChatMessage,
    ) -> Result<RulesChatMessage, RulesChatMessageError> {
        let id = Uuid::new_v4().to_string();
        let now = now_rfc3339();
        let NewRulesChatMessage {
            org_id,
            user_id,
            session_id,
            role,
            content,
        } = new_message;
        let conn = self.db.connection().await?;

        let mut session_rows = conn
            .query(
                "SELECT 1 FROM rules_chat_sessions WHERE id = ?1 AND org_id = ?2 AND user_id = ?3",
                params![session_id.as_str(), org_id, user_id],
            )
            .await?;

        if session_rows.next().await?.is_none() {
            return Err(RulesChatMessageError::NotFound(session_id));
        }

        let mut rows = conn
            .query(
                &format!(
                    "INSERT INTO rules_chat_messages (id, session_id, role, content, created_at, org_id, user_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                     RETURNING {RULES_CHAT_MESSAGE_COLUMNS}"
                ),
                params![
                    id,
                    session_id.as_str(),
                    role.as_str(),
                    content,
                    now.as_str(),
                    org_id,
                    user_id
                ],
            )
            .await?;

        let mut update_rows = conn
            .query(
                "UPDATE rules_chat_sessions SET updated_at = ?1 WHERE id = ?2 AND org_id = ?3 AND user_id = ?4 RETURNING id",
                params![now.as_str(), session_id.as_str(), org_id, user_id],
            )
            .await?;
        let _ = update_rows.next().await?;

        match rows.next().await? {
            Some(row) => row_to_rules_chat_message(row),
            None => Err(RulesChatMessageError::NotFound("insert failed".into())),
        }
    }

    pub async fn get_by_id(
        &self,
        org_id: i64,
        user_id: i64,
        id: &str,
    ) -> Result<RulesChatMessage, RulesChatMessageError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {RULES_CHAT_MESSAGE_COLUMNS}
                     FROM rules_chat_messages
                     WHERE id = ?1 AND org_id = ?2 AND user_id = ?3"
                ),
                params![id, org_id, user_id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_rules_chat_message(row),
            None => Err(RulesChatMessageError::NotFound(id.to_string())),
        }
    }

    pub async fn list_for_session(
        &self,
        org_id: i64,
        user_id: i64,
        session_id: &str,
    ) -> Result<Vec<RulesChatMessage>, RulesChatMessageError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {RULES_CHAT_MESSAGE_COLUMNS}
                     FROM rules_chat_messages
                     WHERE org_id = ?1 AND user_id = ?2 AND session_id = ?3
                     ORDER BY created_at ASC"
                ),
                params![org_id, user_id, session_id],
            )
            .await?;

        let mut messages = Vec::new();
        while let Some(row) = rows.next().await? {
            messages.push(row_to_rules_chat_message(row)?);
        }
        Ok(messages)
    }
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn row_to_deterministic_rule(row: Row) -> Result<DeterministicRule, DeterministicRuleError> {
    let scope: String = row.get(3)?;
    let enabled: i64 = row.get(6)?;
    let disabled_reason: Option<String> = row.get(7)?;
    let conditions_json: String = row.get(8)?;
    let action_parameters_json: String = row.get(10)?;
    let safe_mode: String = row.get(11)?;
    let created_at: String = row.get(12)?;
    let updated_at: String = row.get(13)?;
    let org_id: i64 = row.get(14)?;
    let user_id: Option<i64> = row.get(15)?;

    let scope = RuleScope::from_str(&scope)
        .ok_or_else(|| DeterministicRuleError::InvalidScope(scope.clone()))?;
    let safe_mode = SafeMode::from_str(&safe_mode)
        .ok_or_else(|| DeterministicRuleError::InvalidSafeMode(safe_mode.clone()))?;

    Ok(DeterministicRule {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        scope,
        scope_ref: row.get(4)?,
        priority: row.get(5)?,
        enabled: enabled != 0,
        disabled_reason,
        conditions_json: serde_json::from_str(&conditions_json)?,
        action_type: row.get(9)?,
        action_parameters_json: serde_json::from_str(&action_parameters_json)?,
        safe_mode,
        created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at)?.with_timezone(&Utc),
        org_id,
        user_id,
    })
}

fn row_to_llm_rule(row: Row) -> Result<LlmRule, LlmRuleError> {
    let scope: String = row.get(3)?;
    let enabled: i64 = row.get(6)?;
    let metadata_json: String = row.get(7)?;
    let created_at: String = row.get(8)?;
    let updated_at: String = row.get(9)?;
    let org_id: i64 = row.get(10)?;
    let user_id: Option<i64> = row.get(11)?;

    let scope =
        RuleScope::from_str(&scope).ok_or_else(|| LlmRuleError::InvalidScope(scope.clone()))?;

    Ok(LlmRule {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        scope,
        scope_ref: row.get(4)?,
        rule_text: row.get(5)?,
        enabled: enabled != 0,
        metadata_json: serde_json::from_str(&metadata_json)?,
        created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at)?.with_timezone(&Utc),
        org_id,
        user_id,
    })
}

fn row_to_direction(row: Row) -> Result<Direction, DirectionError> {
    let enabled: i64 = row.get(2)?;
    let created_at: String = row.get(3)?;
    let updated_at: String = row.get(4)?;
    let org_id: i64 = row.get(5)?;
    let user_id: Option<i64> = row.get(6)?;

    Ok(Direction {
        id: row.get(0)?,
        content: row.get(1)?,
        enabled: enabled != 0,
        created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at)?.with_timezone(&Utc),
        org_id,
        user_id,
    })
}

fn row_to_rules_chat_session(row: Row) -> Result<RulesChatSession, RulesChatSessionError> {
    let created_at: String = row.get(2)?;
    let updated_at: String = row.get(3)?;
    let org_id: i64 = row.get(4)?;
    let user_id: i64 = row.get(5)?;

    Ok(RulesChatSession {
        id: row.get(0)?,
        title: row.get(1)?,
        created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at)?.with_timezone(&Utc),
        org_id,
        user_id,
    })
}

fn row_to_rules_chat_message(row: Row) -> Result<RulesChatMessage, RulesChatMessageError> {
    let role: String = row.get(2)?;
    let created_at: String = row.get(4)?;
    let org_id: i64 = row.get(5)?;
    let user_id: i64 = row.get(6)?;

    let role = RulesChatRole::from_str(&role)
        .ok_or_else(|| RulesChatMessageError::InvalidRole(role.clone()))?;

    Ok(RulesChatMessage {
        id: row.get(0)?,
        session_id: row.get(1)?,
        role,
        content: row.get(3)?,
        created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
        org_id,
        user_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
    use crate::migrations::run_migrations;
    use std::time::Duration;
    use tempfile::TempDir;

    async fn setup_db() -> (Database, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        let db_name = format!("db_{}.sqlite", uuid::Uuid::new_v4());
        let db_path = dir.path().join(db_name);
        let db = Database::new(&db_path).await.expect("create db");
        run_migrations(&db).await.expect("migrations");
        (db, dir)
    }

    fn sample_new_det_rule(scope: RuleScope, scope_ref: Option<&str>) -> NewDeterministicRule {
        NewDeterministicRule {
            org_id: DEFAULT_ORG_ID,
            user_id: Some(DEFAULT_USER_ID),
            name: "Block dangerous".into(),
            description: Some("Test rule".into()),
            scope,
            scope_ref: scope_ref.map(|s| s.to_string()),
            priority: 10,
            enabled: true,
            disabled_reason: None,
            conditions_json: serde_json::json!({"all": true}),
            action_type: "flag".into(),
            action_parameters_json: serde_json::json!({"level": "high"}),
            safe_mode: SafeMode::Default,
        }
    }

    fn sample_new_llm_rule(scope: RuleScope, scope_ref: Option<&str>) -> NewLlmRule {
        NewLlmRule {
            org_id: DEFAULT_ORG_ID,
            user_id: Some(DEFAULT_USER_ID),
            name: "LLM guidance".into(),
            description: None,
            scope,
            scope_ref: scope_ref.map(|s| s.to_string()),
            rule_text: "Always be concise.".into(),
            enabled: true,
            metadata_json: serde_json::json!({"kind": "concise"}),
        }
    }

    fn sample_new_direction(enabled: bool) -> NewDirection {
        NewDirection {
            org_id: DEFAULT_ORG_ID,
            user_id: Some(DEFAULT_USER_ID),
            content: "Never send credentials.".into(),
            enabled,
        }
    }

    fn sample_new_chat_session() -> NewRulesChatSession {
        NewRulesChatSession {
            org_id: DEFAULT_ORG_ID,
            user_id: DEFAULT_USER_ID,
            title: Some("Rules assistant".into()),
        }
    }

    fn sample_new_chat_message(session_id: &str, role: RulesChatRole) -> NewRulesChatMessage {
        NewRulesChatMessage {
            org_id: DEFAULT_ORG_ID,
            user_id: DEFAULT_USER_ID,
            session_id: session_id.to_string(),
            role,
            content: "Hello assistant".into(),
        }
    }

    #[tokio::test]
    async fn deterministic_rule_create_and_get() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db);
        let created = repo
            .create(sample_new_det_rule(RuleScope::Global, None))
            .await
            .expect("create");

        let fetched = repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &created.id)
            .await
            .expect("fetch");
        assert_eq!(created, fetched);
        assert!(fetched.enabled);
        assert_eq!(fetched.safe_mode, SafeMode::Default);
        assert_eq!(fetched.conditions_json["all"], serde_json::json!(true));
    }

    #[tokio::test]
    async fn deterministic_rule_list_enabled_by_scope_filters_correctly() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db.clone());

        repo.create(sample_new_det_rule(RuleScope::Global, None))
            .await
            .expect("create global");

        repo.create(sample_new_det_rule(RuleScope::Account, Some("acct1")))
            .await
            .expect("create account");

        let mut disabled = sample_new_det_rule(RuleScope::Account, Some("acct1"));
        disabled.enabled = false;
        repo.create(disabled).await.expect("create disabled");

        let global = repo
            .list_enabled_by_scope(DEFAULT_ORG_ID, DEFAULT_USER_ID, RuleScope::Global, None)
            .await
            .expect("list global");
        assert_eq!(global.len(), 1);

        let account = repo
            .list_enabled_by_scope(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                RuleScope::Account,
                Some("acct1"),
            )
            .await
            .expect("list account");
        assert_eq!(account.len(), 1);
        assert_eq!(account[0].scope_ref.as_deref(), Some("acct1"));
    }

    #[tokio::test]
    async fn deterministic_rule_list_all_orders_by_priority() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db);

        let mut high = sample_new_det_rule(RuleScope::Global, None);
        high.priority = 30;
        let mut low = sample_new_det_rule(RuleScope::Global, None);
        low.priority = 10;
        let mut mid = sample_new_det_rule(RuleScope::Global, None);
        mid.priority = 20;

        repo.create(high).await.expect("create high");
        repo.create(low).await.expect("create low");
        repo.create(mid).await.expect("create mid");

        let priorities: Vec<i64> = repo
            .list_all(DEFAULT_ORG_ID, DEFAULT_USER_ID)
            .await
            .expect("list")
            .into_iter()
            .map(|r| r.priority)
            .collect();

        assert_eq!(priorities, vec![10, 20, 30]);
    }

    #[tokio::test]
    async fn deterministic_rule_update_and_delete() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db);

        let created = repo
            .create(sample_new_det_rule(RuleScope::Global, None))
            .await
            .expect("create");

        let mut updated_data = sample_new_det_rule(RuleScope::Domain, Some("example.com"));
        updated_data.priority = 5;
        updated_data.enabled = false;
        updated_data.safe_mode = SafeMode::AlwaysSafe;
        updated_data.action_type = "quarantine".into();
        updated_data.action_parameters_json = serde_json::json!({"level": "medium"});

        let updated = repo
            .update(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                &created.id,
                updated_data.clone(),
            )
            .await
            .expect("update");

        assert_eq!(updated.scope, RuleScope::Domain);
        assert_eq!(updated.scope_ref.as_deref(), Some("example.com"));
        assert!(!updated.enabled);
        assert_eq!(updated.priority, 5);
        assert_eq!(updated.action_type, "quarantine");
        assert_eq!(updated.safe_mode, SafeMode::AlwaysSafe);

        repo.delete(DEFAULT_ORG_ID, DEFAULT_USER_ID, &created.id)
            .await
            .expect("delete");
        let err = repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &created.id)
            .await
            .expect_err("should be gone");
        assert!(matches!(err, DeterministicRuleError::NotFound(_)));
    }

    #[tokio::test]
    async fn deterministic_rules_filter_by_org_and_user() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db);

        let mut org_wide_rule = sample_new_det_rule(RuleScope::Global, None);
        org_wide_rule.user_id = None;
        let org_wide = repo
            .create(org_wide_rule)
            .await
            .expect("create org-wide rule");

        let user_rule = repo
            .create(sample_new_det_rule(RuleScope::Global, None))
            .await
            .expect("create user rule");

        let mut other_org_rule = sample_new_det_rule(RuleScope::Global, None);
        other_org_rule.org_id = DEFAULT_ORG_ID + 1;
        let other_org = repo
            .create(other_org_rule)
            .await
            .expect("create other org rule");

        let all = repo
            .list_all(DEFAULT_ORG_ID, DEFAULT_USER_ID)
            .await
            .expect("list all");
        assert_eq!(all.len(), 2);
        assert!(all.iter().any(|r| r.id == org_wide.id));
        assert!(all.iter().any(|r| r.id == user_rule.id));

        let scoped = repo
            .list_enabled_by_scope(DEFAULT_ORG_ID, DEFAULT_USER_ID, RuleScope::Global, None)
            .await
            .expect("list scoped");
        assert_eq!(scoped.len(), 2);

        let err = repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &other_org.id)
            .await
            .expect_err("should not return other org data");
        assert!(matches!(err, DeterministicRuleError::NotFound(_)));
    }

    #[tokio::test]
    async fn deterministic_rules_filter_respects_user_specificity() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db);

        let mut org_wide_rule = sample_new_det_rule(RuleScope::Global, None);
        org_wide_rule.user_id = None;
        repo.create(org_wide_rule)
            .await
            .expect("create org-wide rule");

        repo.create(sample_new_det_rule(RuleScope::Global, None))
            .await
            .expect("create user1 rule");

        let mut user2_rule = sample_new_det_rule(RuleScope::Global, None);
        user2_rule.user_id = Some(DEFAULT_USER_ID + 1);
        repo.create(user2_rule).await.expect("create user2 rule");

        let user1_visible = repo
            .list_all(DEFAULT_ORG_ID, DEFAULT_USER_ID)
            .await
            .expect("list user1");
        assert_eq!(user1_visible.len(), 2, "user1 should see org-wide + own");

        let user2_visible = repo
            .list_all(DEFAULT_ORG_ID, DEFAULT_USER_ID + 1)
            .await
            .expect("list user2");
        assert_eq!(user2_visible.len(), 2, "user2 should see org-wide + own");

        let scoped_user1 = repo
            .list_enabled_by_scope(DEFAULT_ORG_ID, DEFAULT_USER_ID, RuleScope::Global, None)
            .await
            .expect("scoped user1");
        assert_eq!(scoped_user1.len(), 2);

        let scoped_user2 = repo
            .list_enabled_by_scope(DEFAULT_ORG_ID, DEFAULT_USER_ID + 1, RuleScope::Global, None)
            .await
            .expect("scoped user2");
        assert_eq!(scoped_user2.len(), 2);
    }

    #[tokio::test]
    async fn deterministic_rule_update_delete_enforce_user_scope() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db);

        let mut user2_rule = sample_new_det_rule(RuleScope::Global, None);
        user2_rule.user_id = Some(DEFAULT_USER_ID + 1);
        let created = repo
            .create(user2_rule.clone())
            .await
            .expect("create user2 rule");

        let update_attempt = repo
            .update(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                &created.id,
                user2_rule.clone(),
            )
            .await
            .expect_err("update with wrong user should fail");
        assert!(matches!(
            update_attempt,
            DeterministicRuleError::NotFound(_)
        ));

        let delete_attempt = repo
            .delete(DEFAULT_ORG_ID, DEFAULT_USER_ID, &created.id)
            .await
            .expect_err("delete with wrong user should fail");
        assert!(matches!(
            delete_attempt,
            DeterministicRuleError::NotFound(_)
        ));

        let still_there = repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID + 1, &created.id)
            .await
            .expect("rule should remain for owner");
        assert_eq!(still_there.id, created.id);
    }

    #[tokio::test]
    async fn llm_rule_crud_and_scope_filter() {
        let (db, _dir) = setup_db().await;
        let repo = LlmRuleRepository::new(db);

        let global = repo
            .create(sample_new_llm_rule(RuleScope::Global, None))
            .await
            .expect("create global");

        let account = repo
            .create(sample_new_llm_rule(RuleScope::Account, Some("acct1")))
            .await
            .expect("create account");

        let by_id = repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &global.id)
            .await
            .expect("fetch by id");
        assert_eq!(global, by_id);

        let enabled_global = repo
            .list_enabled_by_scope(DEFAULT_ORG_ID, DEFAULT_USER_ID, RuleScope::Global, None)
            .await
            .expect("list global");
        assert_eq!(enabled_global.len(), 1);

        let enabled_account = repo
            .list_enabled_by_scope(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                RuleScope::Account,
                Some("acct1"),
            )
            .await
            .expect("list account");
        assert_eq!(enabled_account.len(), 1);
        assert_eq!(enabled_account[0].id, account.id);

        let mut updated = sample_new_llm_rule(RuleScope::Account, Some("acct1"));
        updated.enabled = false;
        updated.rule_text = "Be verbose.".into();
        let stored = repo
            .update(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account.id, updated)
            .await
            .expect("update");
        assert!(!stored.enabled);
        assert_eq!(stored.rule_text, "Be verbose.");

        repo.delete(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account.id)
            .await
            .expect("delete");
        let err = repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account.id)
            .await
            .expect_err("should be deleted");
        assert!(matches!(err, LlmRuleError::NotFound(_)));
    }

    #[tokio::test]
    async fn llm_rule_list_all_returns_all_rules() {
        let (db, _dir) = setup_db().await;
        let repo = LlmRuleRepository::new(db);

        let first = repo
            .create(sample_new_llm_rule(RuleScope::Global, None))
            .await
            .expect("create first");
        let mut second_rule = sample_new_llm_rule(RuleScope::Account, Some("acct1"));
        second_rule.enabled = false;
        let second = repo.create(second_rule).await.expect("create second");

        let all = repo
            .list_all(DEFAULT_ORG_ID, DEFAULT_USER_ID)
            .await
            .expect("list all");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, first.id);
        assert_eq!(all[1].id, second.id);
        assert!(!all[1].enabled);
    }

    #[tokio::test]
    async fn llm_rules_filter_by_org_and_user() {
        let (db, _dir) = setup_db().await;
        let repo = LlmRuleRepository::new(db);

        let mut org_wide_rule = sample_new_llm_rule(RuleScope::Global, None);
        org_wide_rule.user_id = None;
        let org_wide = repo
            .create(org_wide_rule)
            .await
            .expect("create org-wide rule");

        let user_rule = repo
            .create(sample_new_llm_rule(RuleScope::Global, None))
            .await
            .expect("create user rule");

        let mut other_org_rule = sample_new_llm_rule(RuleScope::Global, None);
        other_org_rule.org_id = DEFAULT_ORG_ID + 1;
        let other_org = repo
            .create(other_org_rule)
            .await
            .expect("create other org rule");

        let all = repo
            .list_all(DEFAULT_ORG_ID, DEFAULT_USER_ID)
            .await
            .expect("list all");
        assert_eq!(all.len(), 2);
        assert!(all.iter().any(|r| r.id == org_wide.id));
        assert!(all.iter().any(|r| r.id == user_rule.id));

        let scoped = repo
            .list_enabled_by_scope(DEFAULT_ORG_ID, DEFAULT_USER_ID, RuleScope::Global, None)
            .await
            .expect("list scoped");
        assert_eq!(scoped.len(), 2);

        let err = repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &other_org.id)
            .await
            .expect_err("should not return other org data");
        assert!(matches!(err, LlmRuleError::NotFound(_)));
    }

    #[tokio::test]
    async fn llm_rules_filter_respects_user_specificity() {
        let (db, _dir) = setup_db().await;
        let repo = LlmRuleRepository::new(db);

        let mut org_wide_rule = sample_new_llm_rule(RuleScope::Global, None);
        org_wide_rule.user_id = None;
        repo.create(org_wide_rule)
            .await
            .expect("create org-wide rule");

        repo.create(sample_new_llm_rule(RuleScope::Global, None))
            .await
            .expect("create user1 rule");

        let mut user2_rule = sample_new_llm_rule(RuleScope::Global, None);
        user2_rule.user_id = Some(DEFAULT_USER_ID + 1);
        repo.create(user2_rule).await.expect("create user2 rule");

        let user1_visible = repo
            .list_all(DEFAULT_ORG_ID, DEFAULT_USER_ID)
            .await
            .expect("list user1");
        assert_eq!(user1_visible.len(), 2);

        let user2_visible = repo
            .list_all(DEFAULT_ORG_ID, DEFAULT_USER_ID + 1)
            .await
            .expect("list user2");
        assert_eq!(user2_visible.len(), 2);

        let scoped_user1 = repo
            .list_enabled_by_scope(DEFAULT_ORG_ID, DEFAULT_USER_ID, RuleScope::Global, None)
            .await
            .expect("scoped user1");
        assert_eq!(scoped_user1.len(), 2);

        let scoped_user2 = repo
            .list_enabled_by_scope(DEFAULT_ORG_ID, DEFAULT_USER_ID + 1, RuleScope::Global, None)
            .await
            .expect("scoped user2");
        assert_eq!(scoped_user2.len(), 2);
    }

    #[tokio::test]
    async fn deterministic_rule_list_enabled_by_scope_sender_and_domain() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db.clone());

        // Create rules for different scopes
        repo.create(sample_new_det_rule(RuleScope::Global, None))
            .await
            .expect("create global");

        repo.create(sample_new_det_rule(
            RuleScope::Sender,
            Some("alice@example.com"),
        ))
        .await
        .expect("create sender alice");

        repo.create(sample_new_det_rule(
            RuleScope::Sender,
            Some("bob@example.com"),
        ))
        .await
        .expect("create sender bob");

        repo.create(sample_new_det_rule(RuleScope::Domain, Some("example.com")))
            .await
            .expect("create domain example.com");

        repo.create(sample_new_det_rule(RuleScope::Domain, Some("other.org")))
            .await
            .expect("create domain other.org");

        // Create a disabled sender rule to verify enabled filtering
        let mut disabled_sender =
            sample_new_det_rule(RuleScope::Sender, Some("disabled@example.com"));
        disabled_sender.enabled = false;
        repo.create(disabled_sender)
            .await
            .expect("create disabled sender");

        // Test Sender scope filtering
        let sender_alice = repo
            .list_enabled_by_scope(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                RuleScope::Sender,
                Some("alice@example.com"),
            )
            .await
            .expect("list sender alice");
        assert_eq!(sender_alice.len(), 1);
        assert_eq!(sender_alice[0].scope, RuleScope::Sender);
        assert_eq!(
            sender_alice[0].scope_ref.as_deref(),
            Some("alice@example.com")
        );

        let sender_bob = repo
            .list_enabled_by_scope(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                RuleScope::Sender,
                Some("bob@example.com"),
            )
            .await
            .expect("list sender bob");
        assert_eq!(sender_bob.len(), 1);
        assert_eq!(sender_bob[0].scope_ref.as_deref(), Some("bob@example.com"));

        // Verify disabled sender is not returned
        let sender_disabled = repo
            .list_enabled_by_scope(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                RuleScope::Sender,
                Some("disabled@example.com"),
            )
            .await
            .expect("list disabled sender");
        assert_eq!(sender_disabled.len(), 0);

        // Test Domain scope filtering
        let domain_example = repo
            .list_enabled_by_scope(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                RuleScope::Domain,
                Some("example.com"),
            )
            .await
            .expect("list domain example.com");
        assert_eq!(domain_example.len(), 1);
        assert_eq!(domain_example[0].scope, RuleScope::Domain);
        assert_eq!(domain_example[0].scope_ref.as_deref(), Some("example.com"));

        let domain_other = repo
            .list_enabled_by_scope(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                RuleScope::Domain,
                Some("other.org"),
            )
            .await
            .expect("list domain other.org");
        assert_eq!(domain_other.len(), 1);
        assert_eq!(domain_other[0].scope_ref.as_deref(), Some("other.org"));

        // Verify non-existent scope_ref returns empty
        let domain_nonexistent = repo
            .list_enabled_by_scope(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                RuleScope::Domain,
                Some("nonexistent.net"),
            )
            .await
            .expect("list nonexistent domain");
        assert_eq!(domain_nonexistent.len(), 0);
    }

    #[tokio::test]
    async fn deterministic_rule_list_enabled_by_scope_domain_is_case_insensitive() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db.clone());

        repo.create(sample_new_det_rule(RuleScope::Domain, Some("example.com")))
            .await
            .expect("create domain rule");

        // Query with mixed-case scope_ref should still match.
        let domain_rules = repo
            .list_enabled_by_scope(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                RuleScope::Domain,
                Some("Example.COM"),
            )
            .await
            .expect("list mixed case domain");

        assert_eq!(domain_rules.len(), 1);
        assert_eq!(
            domain_rules[0].scope_ref.as_deref(),
            Some("example.com"),
            "domain scope_ref is normalized to lowercase on read"
        );
    }

    #[tokio::test]
    async fn deterministic_rule_list_enabled_by_scope_sender_matches_legacy_uppercase_ref() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db.clone());

        let rule = repo
            .create(sample_new_det_rule(
                RuleScope::Sender,
                Some("alice@example.com"),
            ))
            .await
            .expect("create sender rule");

        // Simulate legacy data where scope_ref was stored with different casing.
        let conn = db.connection().await.expect("connection");
        conn.execute(
            "UPDATE deterministic_rules SET scope_ref = 'Alice@Example.COM' WHERE id = ?1",
            params![rule.id],
        )
        .await
        .expect("uppercase scope_ref");

        let sender_rules = repo
            .list_enabled_by_scope(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                RuleScope::Sender,
                Some("alice@example.com"),
            )
            .await
            .expect("list sender case-insensitive");

        assert_eq!(sender_rules.len(), 1);
        assert_eq!(
            sender_rules[0]
                .scope_ref
                .as_deref()
                .map(|s| s.to_ascii_lowercase()),
            Some("alice@example.com".into())
        );
    }

    #[tokio::test]
    async fn llm_rule_list_enabled_by_scope_sender_and_domain() {
        let (db, _dir) = setup_db().await;
        let repo = LlmRuleRepository::new(db.clone());

        // Create rules for different scopes
        repo.create(sample_new_llm_rule(RuleScope::Global, None))
            .await
            .expect("create global");

        repo.create(sample_new_llm_rule(
            RuleScope::Sender,
            Some("alice@example.com"),
        ))
        .await
        .expect("create sender alice");

        repo.create(sample_new_llm_rule(
            RuleScope::Sender,
            Some("bob@example.com"),
        ))
        .await
        .expect("create sender bob");

        repo.create(sample_new_llm_rule(RuleScope::Domain, Some("example.com")))
            .await
            .expect("create domain example.com");

        repo.create(sample_new_llm_rule(RuleScope::Domain, Some("other.org")))
            .await
            .expect("create domain other.org");

        // Create a disabled sender rule to verify enabled filtering
        let mut disabled_sender =
            sample_new_llm_rule(RuleScope::Sender, Some("disabled@example.com"));
        disabled_sender.enabled = false;
        repo.create(disabled_sender)
            .await
            .expect("create disabled sender");

        // Test Sender scope filtering
        let sender_alice = repo
            .list_enabled_by_scope(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                RuleScope::Sender,
                Some("alice@example.com"),
            )
            .await
            .expect("list sender alice");
        assert_eq!(sender_alice.len(), 1);
        assert_eq!(sender_alice[0].scope, RuleScope::Sender);
        assert_eq!(
            sender_alice[0].scope_ref.as_deref(),
            Some("alice@example.com")
        );

        let sender_bob = repo
            .list_enabled_by_scope(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                RuleScope::Sender,
                Some("bob@example.com"),
            )
            .await
            .expect("list sender bob");
        assert_eq!(sender_bob.len(), 1);
        assert_eq!(sender_bob[0].scope_ref.as_deref(), Some("bob@example.com"));

        // Verify disabled sender is not returned
        let sender_disabled = repo
            .list_enabled_by_scope(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                RuleScope::Sender,
                Some("disabled@example.com"),
            )
            .await
            .expect("list disabled sender");
        assert_eq!(sender_disabled.len(), 0);

        // Test Domain scope filtering
        let domain_example = repo
            .list_enabled_by_scope(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                RuleScope::Domain,
                Some("example.com"),
            )
            .await
            .expect("list domain example.com");
        assert_eq!(domain_example.len(), 1);
        assert_eq!(domain_example[0].scope, RuleScope::Domain);
        assert_eq!(domain_example[0].scope_ref.as_deref(), Some("example.com"));

        let domain_other = repo
            .list_enabled_by_scope(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                RuleScope::Domain,
                Some("other.org"),
            )
            .await
            .expect("list domain other.org");
        assert_eq!(domain_other.len(), 1);
        assert_eq!(domain_other[0].scope_ref.as_deref(), Some("other.org"));

        // Verify non-existent scope_ref returns empty
        let domain_nonexistent = repo
            .list_enabled_by_scope(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                RuleScope::Domain,
                Some("nonexistent.net"),
            )
            .await
            .expect("list nonexistent domain");
        assert_eq!(domain_nonexistent.len(), 0);
    }

    #[tokio::test]
    async fn directions_crud_and_enabled_filter() {
        let (db, _dir) = setup_db().await;
        let repo = DirectionsRepository::new(db);

        let dir1 = repo
            .create(sample_new_direction(true))
            .await
            .expect("create dir1");
        let dir2 = repo
            .create(sample_new_direction(false))
            .await
            .expect("create dir2");

        let all = repo
            .list_all(DEFAULT_ORG_ID, DEFAULT_USER_ID)
            .await
            .expect("list all");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, dir1.id);
        assert_eq!(all[1].id, dir2.id);

        let enabled = repo
            .list_enabled(DEFAULT_ORG_ID, DEFAULT_USER_ID)
            .await
            .expect("enabled");
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].id, dir1.id);

        let mut update_data = sample_new_direction(true);
        update_data.content = "Updated content".into();
        let updated = repo
            .update(DEFAULT_ORG_ID, DEFAULT_USER_ID, &dir2.id, update_data)
            .await
            .expect("update");
        assert!(updated.enabled);
        assert_eq!(updated.content, "Updated content");

        repo.delete(DEFAULT_ORG_ID, DEFAULT_USER_ID, &dir1.id)
            .await
            .expect("delete");
        let err = repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &dir1.id)
            .await
            .expect_err("should be gone");
        assert!(matches!(err, DirectionError::NotFound(_)));
    }

    #[tokio::test]
    async fn directions_filter_by_org_and_user() {
        let (db, _dir) = setup_db().await;
        let repo = DirectionsRepository::new(db);

        let mut org_wide = sample_new_direction(true);
        org_wide.user_id = None;
        let org_dir = repo
            .create(org_wide)
            .await
            .expect("create org-wide direction");

        let user_dir = repo
            .create(sample_new_direction(true))
            .await
            .expect("create user direction");

        let mut other_org_dir = sample_new_direction(true);
        other_org_dir.org_id = DEFAULT_ORG_ID + 1;
        let other = repo
            .create(other_org_dir)
            .await
            .expect("create other org direction");

        let all = repo
            .list_all(DEFAULT_ORG_ID, DEFAULT_USER_ID)
            .await
            .expect("list all");
        assert_eq!(all.len(), 2);
        assert!(all.iter().any(|d| d.id == org_dir.id));
        assert!(all.iter().any(|d| d.id == user_dir.id));

        let enabled = repo
            .list_enabled(DEFAULT_ORG_ID, DEFAULT_USER_ID)
            .await
            .expect("list enabled");
        assert_eq!(enabled.len(), 2);

        let err = repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &other.id)
            .await
            .expect_err("should not return other org data");
        assert!(matches!(err, DirectionError::NotFound(_)));
    }

    #[tokio::test]
    async fn directions_filter_respects_user_specificity() {
        let (db, _dir) = setup_db().await;
        let repo = DirectionsRepository::new(db);

        let mut org_wide = sample_new_direction(true);
        org_wide.user_id = None;
        repo.create(org_wide)
            .await
            .expect("create org-wide direction");

        repo.create(sample_new_direction(true))
            .await
            .expect("create user1 direction");

        let mut user2_dir = sample_new_direction(true);
        user2_dir.user_id = Some(DEFAULT_USER_ID + 1);
        repo.create(user2_dir)
            .await
            .expect("create user2 direction");

        let user1_all = repo
            .list_all(DEFAULT_ORG_ID, DEFAULT_USER_ID)
            .await
            .expect("list user1");
        assert_eq!(user1_all.len(), 2);

        let user2_all = repo
            .list_all(DEFAULT_ORG_ID, DEFAULT_USER_ID + 1)
            .await
            .expect("list user2");
        assert_eq!(user2_all.len(), 2);

        let enabled_user1 = repo
            .list_enabled(DEFAULT_ORG_ID, DEFAULT_USER_ID)
            .await
            .expect("enabled user1");
        assert_eq!(enabled_user1.len(), 2);

        let enabled_user2 = repo
            .list_enabled(DEFAULT_ORG_ID, DEFAULT_USER_ID + 1)
            .await
            .expect("enabled user2");
        assert_eq!(enabled_user2.len(), 2);
    }

    #[tokio::test]
    async fn rules_chat_sessions_are_scoped_by_org_and_user() {
        let (db, _dir) = setup_db().await;
        let repo = RulesChatSessionRepository::new(db);

        let session = repo
            .create(sample_new_chat_session())
            .await
            .expect("create session");

        let mut other_org = sample_new_chat_session();
        other_org.org_id = DEFAULT_ORG_ID + 1;
        let other_org_session = repo.create(other_org).await.expect("create other org");

        let mut other_user = sample_new_chat_session();
        other_user.user_id = DEFAULT_USER_ID + 1;
        let other_user_session = repo.create(other_user).await.expect("create other user");

        let fetched = repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &session.id)
            .await
            .expect("fetch by id");
        assert_eq!(fetched.id, session.id);

        let user1_sessions = repo
            .list_for_user(DEFAULT_ORG_ID, DEFAULT_USER_ID)
            .await
            .expect("list user1");
        assert_eq!(user1_sessions.len(), 1);
        assert_eq!(user1_sessions[0].id, session.id);

        let user2_sessions = repo
            .list_for_user(DEFAULT_ORG_ID, DEFAULT_USER_ID + 1)
            .await
            .expect("list user2");
        assert_eq!(user2_sessions.len(), 1);
        assert_eq!(user2_sessions[0].id, other_user_session.id);

        let err = repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &other_org_session.id)
            .await
            .expect_err("should not fetch other org");
        assert!(matches!(err, RulesChatSessionError::NotFound(_)));
    }

    #[tokio::test]
    async fn rules_chat_message_create_requires_matching_session_owner() {
        let (db, _dir) = setup_db().await;
        let session_repo = RulesChatSessionRepository::new(db.clone());
        let message_repo = RulesChatMessageRepository::new(db);

        let session = session_repo
            .create(sample_new_chat_session())
            .await
            .expect("create session");

        let mut wrong_org = sample_new_chat_message(&session.id, RulesChatRole::User);
        wrong_org.org_id = DEFAULT_ORG_ID + 1;
        let err = message_repo
            .create(wrong_org)
            .await
            .expect_err("should reject mismatched org");
        assert!(matches!(err, RulesChatMessageError::NotFound(_)));

        let mut wrong_user = sample_new_chat_message(&session.id, RulesChatRole::User);
        wrong_user.user_id = DEFAULT_USER_ID + 1;
        let err = message_repo
            .create(wrong_user)
            .await
            .expect_err("should reject mismatched user");
        assert!(matches!(err, RulesChatMessageError::NotFound(_)));

        let messages = message_repo
            .list_for_session(DEFAULT_ORG_ID, DEFAULT_USER_ID, &session.id)
            .await
            .expect("list messages");
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn rules_chat_session_updated_at_bumps_when_message_added() {
        let (db, _dir) = setup_db().await;
        let session_repo = RulesChatSessionRepository::new(db.clone());
        let message_repo = RulesChatMessageRepository::new(db);

        let session = session_repo
            .create(sample_new_chat_session())
            .await
            .expect("create session");

        let initial_updated_at = session.updated_at;

        tokio::time::sleep(Duration::from_millis(10)).await;

        message_repo
            .create(sample_new_chat_message(&session.id, RulesChatRole::User))
            .await
            .expect("create message");

        let refreshed = session_repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &session.id)
            .await
            .expect("fetch refreshed session");
        assert!(
            refreshed.updated_at > initial_updated_at,
            "updated_at should advance after message"
        );
    }

    #[tokio::test]
    async fn rules_chat_messages_enforce_scoping() {
        let (db, _dir) = setup_db().await;
        let session_repo = RulesChatSessionRepository::new(db.clone());
        let message_repo = RulesChatMessageRepository::new(db);

        let session = session_repo
            .create(sample_new_chat_session())
            .await
            .expect("create session");

        let message = message_repo
            .create(sample_new_chat_message(&session.id, RulesChatRole::User))
            .await
            .expect("create message");

        let fetched = message_repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message.id)
            .await
            .expect("fetch message");
        assert_eq!(fetched.role, RulesChatRole::User);
        assert_eq!(fetched.session_id, session.id);

        let mut other_session = sample_new_chat_session();
        other_session.org_id = DEFAULT_ORG_ID + 1;
        let other_session = session_repo
            .create(other_session)
            .await
            .expect("create other session");

        let other_message = message_repo
            .create(NewRulesChatMessage {
                org_id: DEFAULT_ORG_ID + 1,
                user_id: DEFAULT_USER_ID,
                session_id: other_session.id.clone(),
                role: RulesChatRole::Assistant,
                content: "Other org message".into(),
            })
            .await
            .expect("create other message");

        let messages = message_repo
            .list_for_session(DEFAULT_ORG_ID, DEFAULT_USER_ID, &session.id)
            .await
            .expect("list messages");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].id, message.id);

        let err = message_repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &other_message.id)
            .await
            .expect_err("should not fetch other org message");
        assert!(matches!(err, RulesChatMessageError::NotFound(_)));
    }

    #[tokio::test]
    async fn disable_rule_with_reason_disables_and_sets_reason() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db);

        let created = repo
            .create(sample_new_det_rule(RuleScope::Global, None))
            .await
            .expect("create rule");

        assert!(created.enabled);
        assert!(created.disabled_reason.is_none());

        let disabled = repo
            .disable_rule_with_reason(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                &created.id,
                "Label 'Work' was deleted from Gmail",
            )
            .await
            .expect("disable rule");

        assert!(!disabled.enabled);
        assert_eq!(
            disabled.disabled_reason.as_deref(),
            Some("Label 'Work' was deleted from Gmail")
        );
        assert!(disabled.updated_at > created.updated_at);
    }

    #[tokio::test]
    async fn disable_rule_with_reason_returns_not_found_for_nonexistent() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db);

        let err = repo
            .disable_rule_with_reason(DEFAULT_ORG_ID, DEFAULT_USER_ID, "nonexistent", "reason")
            .await
            .expect_err("should not find nonexistent rule");

        assert!(matches!(err, DeterministicRuleError::NotFound(_)));
    }

    #[tokio::test]
    async fn disable_rule_with_reason_enforces_user_scope() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db);

        let mut user2_rule = sample_new_det_rule(RuleScope::Global, None);
        user2_rule.user_id = Some(DEFAULT_USER_ID + 1);
        let created = repo.create(user2_rule).await.expect("create rule");

        // Wrong user should not be able to disable the rule
        let err = repo
            .disable_rule_with_reason(DEFAULT_ORG_ID, DEFAULT_USER_ID, &created.id, "reason")
            .await
            .expect_err("wrong user should fail");

        assert!(matches!(err, DeterministicRuleError::NotFound(_)));

        // Rule should still be enabled for the actual owner
        let still_enabled = repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID + 1, &created.id)
            .await
            .expect("fetch as owner");
        assert!(still_enabled.enabled);
    }

    #[tokio::test]
    async fn find_rules_referencing_label_finds_by_condition() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db);

        // Create a rule with a LabelPresent condition containing the label ID
        let mut rule_with_label = sample_new_det_rule(RuleScope::Global, None);
        rule_with_label.conditions_json =
            serde_json::json!({"type": "LabelPresent", "value": "Label_123"});
        let created = repo
            .create(rule_with_label)
            .await
            .expect("create rule with label");

        // Create another rule without the label
        let rule_without_label = sample_new_det_rule(RuleScope::Global, None);
        repo.create(rule_without_label)
            .await
            .expect("create rule without label");

        let found = repo
            .find_rules_referencing_label(DEFAULT_ORG_ID, DEFAULT_USER_ID, "Label_123")
            .await
            .expect("find rules");

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, created.id);
    }

    #[tokio::test]
    async fn find_rules_referencing_label_finds_by_action_parameters() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db);

        // Create a rule with apply_label action containing the label ID
        let mut rule_with_apply_label = sample_new_det_rule(RuleScope::Global, None);
        rule_with_apply_label.action_type = "apply_label".to_string();
        rule_with_apply_label.action_parameters_json = serde_json::json!({"label_id": "Label_456"});
        let created = repo
            .create(rule_with_apply_label)
            .await
            .expect("create rule with apply_label");

        // Create another rule without the label
        let rule_without_label = sample_new_det_rule(RuleScope::Global, None);
        repo.create(rule_without_label)
            .await
            .expect("create rule without label");

        let found = repo
            .find_rules_referencing_label(DEFAULT_ORG_ID, DEFAULT_USER_ID, "Label_456")
            .await
            .expect("find rules");

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, created.id);
    }

    #[tokio::test]
    async fn find_rules_referencing_label_finds_both_condition_and_action() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db);

        // Rule 1: references label in conditions
        let mut rule1 = sample_new_det_rule(RuleScope::Global, None);
        rule1.conditions_json = serde_json::json!({"type": "LabelPresent", "value": "Label_789"});
        let created1 = repo.create(rule1).await.expect("create rule1");

        // Rule 2: references label in action parameters
        let mut rule2 = sample_new_det_rule(RuleScope::Global, None);
        rule2.action_type = "apply_label".to_string();
        rule2.action_parameters_json = serde_json::json!({"label_id": "Label_789"});
        let created2 = repo.create(rule2).await.expect("create rule2");

        // Rule 3: references a different label
        let mut rule3 = sample_new_det_rule(RuleScope::Global, None);
        rule3.conditions_json = serde_json::json!({"type": "LabelPresent", "value": "INBOX"});
        repo.create(rule3).await.expect("create rule3");

        let found = repo
            .find_rules_referencing_label(DEFAULT_ORG_ID, DEFAULT_USER_ID, "Label_789")
            .await
            .expect("find rules");

        assert_eq!(found.len(), 2);
        let found_ids: Vec<&str> = found.iter().map(|r| r.id.as_str()).collect();
        assert!(found_ids.contains(&created1.id.as_str()));
        assert!(found_ids.contains(&created2.id.as_str()));
    }

    #[tokio::test]
    async fn find_rules_referencing_label_returns_empty_when_no_matches() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db);

        // Create some rules without the label we're searching for
        repo.create(sample_new_det_rule(RuleScope::Global, None))
            .await
            .expect("create rule");

        let found = repo
            .find_rules_referencing_label(DEFAULT_ORG_ID, DEFAULT_USER_ID, "NonexistentLabel")
            .await
            .expect("find rules");

        assert!(found.is_empty());
    }

    #[tokio::test]
    async fn find_rules_referencing_label_enforces_org_user_scope() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db);

        // Create a rule for user 2 with a label reference
        let mut rule = sample_new_det_rule(RuleScope::Global, None);
        rule.user_id = Some(DEFAULT_USER_ID + 1);
        rule.conditions_json = serde_json::json!({"type": "LabelPresent", "value": "Label_abc"});
        repo.create(rule).await.expect("create rule");

        // User 1 should not find user 2's rules
        let found = repo
            .find_rules_referencing_label(DEFAULT_ORG_ID, DEFAULT_USER_ID, "Label_abc")
            .await
            .expect("find rules");

        assert!(found.is_empty());

        // User 2 should find their own rule
        let found = repo
            .find_rules_referencing_label(DEFAULT_ORG_ID, DEFAULT_USER_ID + 1, "Label_abc")
            .await
            .expect("find rules");

        assert_eq!(found.len(), 1);
    }

    #[tokio::test]
    async fn find_rules_referencing_label_does_not_match_prefix() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db);

        // Create a rule with "Label_1" - this should NOT be found when searching for "Label_1"
        // if we're searching for "Label_10"
        let mut rule_label_1 = sample_new_det_rule(RuleScope::Global, None);
        rule_label_1.conditions_json =
            serde_json::json!({"type": "LabelPresent", "value": "Label_1"});
        let created_label_1 = repo
            .create(rule_label_1)
            .await
            .expect("create rule with Label_1");

        // Create a rule with "Label_10" - this should be found when searching for "Label_10"
        let mut rule_label_10 = sample_new_det_rule(RuleScope::Global, None);
        rule_label_10.conditions_json =
            serde_json::json!({"type": "LabelPresent", "value": "Label_10"});
        let created_label_10 = repo
            .create(rule_label_10)
            .await
            .expect("create rule with Label_10");

        // Create a rule with "Label_123" - this should NOT be found when searching for "Label_1"
        let mut rule_label_123 = sample_new_det_rule(RuleScope::Global, None);
        rule_label_123.action_type = "apply_label".to_string();
        rule_label_123.action_parameters_json = serde_json::json!({"label_id": "Label_123"});
        repo.create(rule_label_123)
            .await
            .expect("create rule with Label_123");

        // Searching for "Label_1" should only find the rule with exactly "Label_1"
        let found_label_1 = repo
            .find_rules_referencing_label(DEFAULT_ORG_ID, DEFAULT_USER_ID, "Label_1")
            .await
            .expect("find rules for Label_1");

        assert_eq!(
            found_label_1.len(),
            1,
            "Should find exactly 1 rule for Label_1, not rules with Label_10 or Label_123"
        );
        assert_eq!(found_label_1[0].id, created_label_1.id);

        // Searching for "Label_10" should only find the rule with exactly "Label_10"
        let found_label_10 = repo
            .find_rules_referencing_label(DEFAULT_ORG_ID, DEFAULT_USER_ID, "Label_10")
            .await
            .expect("find rules for Label_10");

        assert_eq!(
            found_label_10.len(),
            1,
            "Should find exactly 1 rule for Label_10"
        );
        assert_eq!(found_label_10[0].id, created_label_10.id);
    }

    #[tokio::test]
    async fn deterministic_rule_disabled_reason_is_stored_on_create() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db);

        let mut rule = sample_new_det_rule(RuleScope::Global, None);
        rule.enabled = false;
        rule.disabled_reason = Some("Pre-disabled for testing".to_string());

        let created = repo.create(rule).await.expect("create rule");

        assert!(!created.enabled);
        assert_eq!(
            created.disabled_reason.as_deref(),
            Some("Pre-disabled for testing")
        );
    }

    #[tokio::test]
    async fn deterministic_rule_disabled_reason_is_updated() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db);

        let created = repo
            .create(sample_new_det_rule(RuleScope::Global, None))
            .await
            .expect("create rule");

        assert!(created.disabled_reason.is_none());

        let mut updated_rule = sample_new_det_rule(RuleScope::Global, None);
        updated_rule.enabled = false;
        updated_rule.disabled_reason = Some("Label deleted".to_string());

        let updated = repo
            .update(DEFAULT_ORG_ID, DEFAULT_USER_ID, &created.id, updated_rule)
            .await
            .expect("update rule");

        assert!(!updated.enabled);
        assert_eq!(updated.disabled_reason.as_deref(), Some("Label deleted"));
    }

    #[tokio::test]
    async fn disabled_reason_can_be_cleared_via_update() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db);

        // Create rule with disabled_reason
        let mut rule = sample_new_det_rule(RuleScope::Global, None);
        rule.enabled = false;
        rule.disabled_reason = Some("Initially disabled".to_string());

        let created = repo.create(rule).await.expect("create rule");
        assert_eq!(
            created.disabled_reason.as_deref(),
            Some("Initially disabled")
        );

        // Re-enable and clear disabled_reason via update
        let mut updated_rule = sample_new_det_rule(RuleScope::Global, None);
        updated_rule.enabled = true;
        updated_rule.disabled_reason = None;

        let updated = repo
            .update(DEFAULT_ORG_ID, DEFAULT_USER_ID, &created.id, updated_rule)
            .await
            .expect("update rule");

        assert!(updated.enabled);
        assert!(updated.disabled_reason.is_none());
    }

    #[tokio::test]
    async fn find_rules_referencing_label_matches_exact_label_id() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db);

        // Create rules with distinct label IDs
        let mut rule_abc = sample_new_det_rule(RuleScope::Global, None);
        rule_abc.conditions_json =
            serde_json::json!({"type": "LabelPresent", "value": "Label_ABC"});
        repo.create(rule_abc).await.expect("create rule abc");

        let mut rule_xyz = sample_new_det_rule(RuleScope::Global, None);
        rule_xyz.conditions_json =
            serde_json::json!({"type": "LabelPresent", "value": "Label_XYZ"});
        repo.create(rule_xyz).await.expect("create rule xyz");

        // Search with exact label ID - should find only the matching rule
        let found_abc = repo
            .find_rules_referencing_label(DEFAULT_ORG_ID, DEFAULT_USER_ID, "Label_ABC")
            .await
            .expect("find abc");
        assert_eq!(found_abc.len(), 1);
        assert!(
            found_abc[0]
                .conditions_json
                .to_string()
                .contains("Label_ABC")
        );

        let found_xyz = repo
            .find_rules_referencing_label(DEFAULT_ORG_ID, DEFAULT_USER_ID, "Label_XYZ")
            .await
            .expect("find xyz");
        assert_eq!(found_xyz.len(), 1);
        assert!(
            found_xyz[0]
                .conditions_json
                .to_string()
                .contains("Label_XYZ")
        );

        // Note: SQLite LIKE is case-insensitive by default. This test documents
        // the current behavior: searching "label_abc" will match "Label_ABC".
        // Gmail label IDs in practice use consistent casing, so this should
        // not cause issues in production usage.
        let found_lower = repo
            .find_rules_referencing_label(DEFAULT_ORG_ID, DEFAULT_USER_ID, "label_abc")
            .await
            .expect("find lowercase");
        // SQLite LIKE is case-insensitive, so this will find the rule
        assert_eq!(found_lower.len(), 1);
    }

    #[tokio::test]
    async fn find_rules_referencing_label_no_partial_matches() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db);

        // This test verifies that the search uses quoted JSON matching to avoid false positives.
        // Searching for "Label_1" should NOT match "Label_10", "Label_123", etc.

        let mut rule1 = sample_new_det_rule(RuleScope::Global, None);
        rule1.conditions_json = serde_json::json!({"type": "LabelPresent", "value": "Label_1"});
        let created1 = repo.create(rule1).await.expect("create rule1");

        let mut rule2 = sample_new_det_rule(RuleScope::Global, None);
        rule2.conditions_json = serde_json::json!({"type": "LabelPresent", "value": "Label_10"});
        repo.create(rule2).await.expect("create rule2");

        let mut rule3 = sample_new_det_rule(RuleScope::Global, None);
        rule3.conditions_json = serde_json::json!({"type": "LabelPresent", "value": "Label_123"});
        repo.create(rule3).await.expect("create rule3");

        // Searching for "Label_1" should only find rule1 with exactly "Label_1"
        // It should NOT find rules with "Label_10" or "Label_123"
        let found = repo
            .find_rules_referencing_label(DEFAULT_ORG_ID, DEFAULT_USER_ID, "Label_1")
            .await
            .expect("find rules");

        // Only the exact match should be found
        assert_eq!(
            found.len(),
            1,
            "Should only find exact matches, not prefix matches"
        );
        assert_eq!(found[0].id, created1.id);
    }

    #[tokio::test]
    async fn find_rules_referencing_label_nested_condition() {
        let (db, _dir) = setup_db().await;
        let repo = DeterministicRuleRepository::new(db);

        // Create a rule with a nested condition structure containing the label
        let mut rule = sample_new_det_rule(RuleScope::Global, None);
        rule.conditions_json = serde_json::json!({
            "type": "And",
            "children": [
                {"type": "LabelPresent", "value": "Label_NESTED"},
                {"type": "SenderEmail", "value": "test@example.com"}
            ]
        });
        let created = repo.create(rule).await.expect("create rule");

        let found = repo
            .find_rules_referencing_label(DEFAULT_ORG_ID, DEFAULT_USER_ID, "Label_NESTED")
            .await
            .expect("find rules");

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, created.id);
    }
}
