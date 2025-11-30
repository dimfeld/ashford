use chrono::{DateTime, SecondsFormat, Utc};
use libsql::{Row, params};
use thiserror::Error;
use uuid::Uuid;

use crate::db::{Database, DbError};

use super::types::{
    DeterministicRule, Direction, LlmRule, NewDeterministicRule, NewDirection, NewLlmRule,
    RuleScope, SafeMode,
};

const DETERMINISTIC_RULE_COLUMNS: &str = "id, name, description, scope, scope_ref, priority, enabled, conditions_json, action_type, action_parameters_json, safe_mode, created_at, updated_at";
const LLM_RULE_COLUMNS: &str = "id, name, description, scope, scope_ref, rule_text, enabled, metadata_json, created_at, updated_at";
const DIRECTION_COLUMNS: &str = "id, content, enabled, created_at, updated_at";

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
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "INSERT INTO deterministic_rules (
                        id, name, description, scope, scope_ref, priority, enabled, conditions_json, action_type, action_parameters_json, safe_mode, created_at, updated_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)
                    RETURNING {DETERMINISTIC_RULE_COLUMNS}"
                ),
                params![
                    id,
                    new_rule.name,
                    new_rule.description,
                    new_rule.scope.as_str(),
                    new_rule.scope_ref,
                    new_rule.priority,
                    enabled,
                    conditions_json,
                    new_rule.action_type,
                    action_parameters_json,
                    new_rule.safe_mode.as_str(),
                    now
                ],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_deterministic_rule(row),
            None => Err(DeterministicRuleError::NotFound("insert failed".into())),
        }
    }

    pub async fn get_by_id(&self, id: &str) -> Result<DeterministicRule, DeterministicRuleError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {DETERMINISTIC_RULE_COLUMNS} FROM deterministic_rules WHERE id = ?1"
                ),
                params![id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_deterministic_rule(row),
            None => Err(DeterministicRuleError::NotFound(id.to_string())),
        }
    }

    pub async fn list_all(&self) -> Result<Vec<DeterministicRule>, DeterministicRuleError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {DETERMINISTIC_RULE_COLUMNS}
                     FROM deterministic_rules
                     ORDER BY priority ASC, created_at"
                ),
                (),
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
        scope: RuleScope,
        scope_ref: Option<&str>,
    ) -> Result<Vec<DeterministicRule>, DeterministicRuleError> {
        let conn = self.db.connection().await?;
        let scope_value = scope.as_str();
        let mut rows = if let Some(reference) = scope_ref {
            conn.query(
                &format!(
                    "SELECT {DETERMINISTIC_RULE_COLUMNS}
                     FROM deterministic_rules
                     WHERE enabled = 1 AND scope = ?1 AND scope_ref = ?2
                     ORDER BY priority ASC, created_at"
                ),
                params![scope_value, reference],
            )
            .await?
        } else {
            conn.query(
                &format!(
                    "SELECT {DETERMINISTIC_RULE_COLUMNS}
                     FROM deterministic_rules
                     WHERE enabled = 1 AND scope = ?1 AND scope_ref IS NULL
                     ORDER BY priority ASC, created_at"
                ),
                params![scope_value],
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
        id: &str,
        updated: NewDeterministicRule,
    ) -> Result<DeterministicRule, DeterministicRuleError> {
        let now = now_rfc3339();
        let conditions_json = serde_json::to_string(&updated.conditions_json)?;
        let action_parameters_json = serde_json::to_string(&updated.action_parameters_json)?;
        let enabled = updated.enabled as i64;
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
                         conditions_json = ?7,
                         action_type = ?8,
                         action_parameters_json = ?9,
                         safe_mode = ?10,
                         updated_at = ?11
                     WHERE id = ?12
                     RETURNING {DETERMINISTIC_RULE_COLUMNS}"
                ),
                params![
                    updated.name,
                    updated.description,
                    updated.scope.as_str(),
                    updated.scope_ref,
                    updated.priority,
                    enabled,
                    conditions_json,
                    updated.action_type,
                    action_parameters_json,
                    updated.safe_mode.as_str(),
                    now,
                    id
                ],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_deterministic_rule(row),
            None => Err(DeterministicRuleError::NotFound(id.to_string())),
        }
    }

    pub async fn delete(&self, id: &str) -> Result<(), DeterministicRuleError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                "DELETE FROM deterministic_rules WHERE id = ?1 RETURNING id",
                params![id],
            )
            .await?;

        match rows.next().await? {
            Some(_) => Ok(()),
            None => Err(DeterministicRuleError::NotFound(id.to_string())),
        }
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
                        id, name, description, scope, scope_ref, rule_text, enabled, metadata_json, created_at, updated_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
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
                    now
                ],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_llm_rule(row),
            None => Err(LlmRuleError::NotFound("insert failed".into())),
        }
    }

    pub async fn get_by_id(&self, id: &str) -> Result<LlmRule, LlmRuleError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!("SELECT {LLM_RULE_COLUMNS} FROM llm_rules WHERE id = ?1"),
                params![id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_llm_rule(row),
            None => Err(LlmRuleError::NotFound(id.to_string())),
        }
    }

    pub async fn list_all(&self) -> Result<Vec<LlmRule>, LlmRuleError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {LLM_RULE_COLUMNS}
                     FROM llm_rules
                     ORDER BY created_at"
                ),
                (),
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
                     WHERE enabled = 1 AND scope = ?1 AND scope_ref = ?2
                     ORDER BY created_at"
                ),
                params![scope_value, reference],
            )
            .await?
        } else {
            conn.query(
                &format!(
                    "SELECT {LLM_RULE_COLUMNS}
                     FROM llm_rules
                     WHERE enabled = 1 AND scope = ?1 AND scope_ref IS NULL
                     ORDER BY created_at"
                ),
                params![scope_value],
            )
            .await?
        };

        let mut rules = Vec::new();
        while let Some(row) = rows.next().await? {
            rules.push(row_to_llm_rule(row)?);
        }
        Ok(rules)
    }

    pub async fn update(&self, id: &str, updated: NewLlmRule) -> Result<LlmRule, LlmRuleError> {
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
                         updated_at = ?8
                     WHERE id = ?9
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
                    now,
                    id
                ],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_llm_rule(row),
            None => Err(LlmRuleError::NotFound(id.to_string())),
        }
    }

    pub async fn delete(&self, id: &str) -> Result<(), LlmRuleError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                "DELETE FROM llm_rules WHERE id = ?1 RETURNING id",
                params![id],
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
                    "INSERT INTO directions (id, content, enabled, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?4)
                     RETURNING {DIRECTION_COLUMNS}"
                ),
                params![id, new_direction.content, enabled, now],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_direction(row),
            None => Err(DirectionError::NotFound("insert failed".into())),
        }
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Direction, DirectionError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!("SELECT {DIRECTION_COLUMNS} FROM directions WHERE id = ?1"),
                params![id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_direction(row),
            None => Err(DirectionError::NotFound(id.to_string())),
        }
    }

    pub async fn list_all(&self) -> Result<Vec<Direction>, DirectionError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {DIRECTION_COLUMNS}
                     FROM directions
                     ORDER BY created_at"
                ),
                (),
            )
            .await?;

        let mut directions = Vec::new();
        while let Some(row) = rows.next().await? {
            directions.push(row_to_direction(row)?);
        }
        Ok(directions)
    }

    pub async fn list_enabled(&self) -> Result<Vec<Direction>, DirectionError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {DIRECTION_COLUMNS}
                     FROM directions
                     WHERE enabled = 1
                     ORDER BY created_at"
                ),
                (),
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
                         updated_at = ?3
                     WHERE id = ?4
                     RETURNING {DIRECTION_COLUMNS}"
                ),
                params![updated.content, enabled, now, id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_direction(row),
            None => Err(DirectionError::NotFound(id.to_string())),
        }
    }

    pub async fn delete(&self, id: &str) -> Result<(), DirectionError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                "DELETE FROM directions WHERE id = ?1 RETURNING id",
                params![id],
            )
            .await?;

        match rows.next().await? {
            Some(_) => Ok(()),
            None => Err(DirectionError::NotFound(id.to_string())),
        }
    }
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn row_to_deterministic_rule(row: Row) -> Result<DeterministicRule, DeterministicRuleError> {
    let scope: String = row.get(3)?;
    let enabled: i64 = row.get(6)?;
    let conditions_json: String = row.get(7)?;
    let action_parameters_json: String = row.get(9)?;
    let safe_mode: String = row.get(10)?;
    let created_at: String = row.get(11)?;
    let updated_at: String = row.get(12)?;

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
        conditions_json: serde_json::from_str(&conditions_json)?,
        action_type: row.get(8)?,
        action_parameters_json: serde_json::from_str(&action_parameters_json)?,
        safe_mode,
        created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at)?.with_timezone(&Utc),
    })
}

fn row_to_llm_rule(row: Row) -> Result<LlmRule, LlmRuleError> {
    let scope: String = row.get(3)?;
    let enabled: i64 = row.get(6)?;
    let metadata_json: String = row.get(7)?;
    let created_at: String = row.get(8)?;
    let updated_at: String = row.get(9)?;

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
    })
}

fn row_to_direction(row: Row) -> Result<Direction, DirectionError> {
    let enabled: i64 = row.get(2)?;
    let created_at: String = row.get(3)?;
    let updated_at: String = row.get(4)?;

    Ok(Direction {
        id: row.get(0)?,
        content: row.get(1)?,
        enabled: enabled != 0,
        created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at)?.with_timezone(&Utc),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::run_migrations;
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
            name: "Block dangerous".into(),
            description: Some("Test rule".into()),
            scope,
            scope_ref: scope_ref.map(|s| s.to_string()),
            priority: 10,
            enabled: true,
            conditions_json: serde_json::json!({"all": true}),
            action_type: "flag".into(),
            action_parameters_json: serde_json::json!({"level": "high"}),
            safe_mode: SafeMode::Default,
        }
    }

    fn sample_new_llm_rule(scope: RuleScope, scope_ref: Option<&str>) -> NewLlmRule {
        NewLlmRule {
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
            content: "Never send credentials.".into(),
            enabled,
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

        let fetched = repo.get_by_id(&created.id).await.expect("fetch");
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
            .list_enabled_by_scope(RuleScope::Global, None)
            .await
            .expect("list global");
        assert_eq!(global.len(), 1);

        let account = repo
            .list_enabled_by_scope(RuleScope::Account, Some("acct1"))
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
            .list_all()
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
            .update(&created.id, updated_data.clone())
            .await
            .expect("update");

        assert_eq!(updated.scope, RuleScope::Domain);
        assert_eq!(updated.scope_ref.as_deref(), Some("example.com"));
        assert!(!updated.enabled);
        assert_eq!(updated.priority, 5);
        assert_eq!(updated.action_type, "quarantine");
        assert_eq!(updated.safe_mode, SafeMode::AlwaysSafe);

        repo.delete(&created.id).await.expect("delete");
        let err = repo
            .get_by_id(&created.id)
            .await
            .expect_err("should be gone");
        assert!(matches!(err, DeterministicRuleError::NotFound(_)));
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

        let by_id = repo.get_by_id(&global.id).await.expect("fetch by id");
        assert_eq!(global, by_id);

        let enabled_global = repo
            .list_enabled_by_scope(RuleScope::Global, None)
            .await
            .expect("list global");
        assert_eq!(enabled_global.len(), 1);

        let enabled_account = repo
            .list_enabled_by_scope(RuleScope::Account, Some("acct1"))
            .await
            .expect("list account");
        assert_eq!(enabled_account.len(), 1);
        assert_eq!(enabled_account[0].id, account.id);

        let mut updated = sample_new_llm_rule(RuleScope::Account, Some("acct1"));
        updated.enabled = false;
        updated.rule_text = "Be verbose.".into();
        let stored = repo.update(&account.id, updated).await.expect("update");
        assert!(!stored.enabled);
        assert_eq!(stored.rule_text, "Be verbose.");

        repo.delete(&account.id).await.expect("delete");
        let err = repo
            .get_by_id(&account.id)
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

        let all = repo.list_all().await.expect("list all");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, first.id);
        assert_eq!(all[1].id, second.id);
        assert!(!all[1].enabled);
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

        let all = repo.list_all().await.expect("list all");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, dir1.id);
        assert_eq!(all[1].id, dir2.id);

        let enabled = repo.list_enabled().await.expect("enabled");
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].id, dir1.id);

        let mut update_data = sample_new_direction(true);
        update_data.content = "Updated content".into();
        let updated = repo.update(&dir2.id, update_data).await.expect("update");
        assert!(updated.enabled);
        assert_eq!(updated.content, "Updated content");

        repo.delete(&dir1.id).await.expect("delete");
        let err = repo.get_by_id(&dir1.id).await.expect_err("should be gone");
        assert!(matches!(err, DirectionError::NotFound(_)));
    }
}
