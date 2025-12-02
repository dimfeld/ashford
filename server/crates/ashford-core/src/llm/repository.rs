use chrono::{DateTime, SecondsFormat, Utc};
use libsql::{Row, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use crate::db::{Database, DbError};

const LLM_CALL_COLUMNS: &str = "id, org_id, user_id, feature, context_json, model, request_json, response_json, input_tokens, output_tokens, latency_ms, error, trace_id, created_at";
const DEFAULT_LIST_LIMIT: i64 = 100;

#[derive(Debug, Error)]
pub enum LlmCallError {
    #[error("database error: {0}")]
    Database(#[from] DbError),
    #[error("sql error: {0}")]
    Sql(#[from] libsql::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("datetime parse error: {0}")]
    DateTimeParse(#[from] chrono::ParseError),
    #[error("insert failed: {0}")]
    InsertFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct LlmCallContext {
    /// High-level feature name, e.g., "classification" or "rules_assistant".
    pub feature: String,
    pub org_id: Option<i64>,
    pub user_id: Option<i64>,
    pub account_id: Option<String>,
    pub message_id: Option<String>,
    pub thread_id: Option<String>,
    pub rule_name: Option<String>,
    pub rule_id: Option<String>,
}

impl LlmCallContext {
    pub fn new(feature: impl Into<String>) -> Self {
        Self {
            feature: feature.into(),
            org_id: None,
            user_id: None,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NewLlmCall {
    pub org_id: i64,
    pub user_id: i64,
    pub context: LlmCallContext,
    pub model: String,
    pub request_json: Value,
    pub response_json: Option<Value>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmCall {
    pub id: String,
    pub org_id: i64,
    pub user_id: i64,
    pub feature: String,
    pub context: LlmCallContext,
    pub model: String,
    pub request_json: Value,
    pub response_json: Option<Value>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
    pub trace_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct LlmCallRepository {
    db: Database,
}

impl LlmCallRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn create(&self, call: NewLlmCall) -> Result<LlmCall, LlmCallError> {
        let id = Uuid::new_v4().to_string();
        let now = now_rfc3339();
        let context_json = serde_json::to_string(&call.context)?;
        let request_json = serde_json::to_string(&call.request_json)?;
        let response_json = match &call.response_json {
            Some(value) => Some(serde_json::to_string(value)?),
            None => None,
        };
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "INSERT INTO llm_calls (
                        id, org_id, user_id, feature, context_json, model, request_json, response_json,
                        input_tokens, output_tokens, latency_ms, error, trace_id, created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                    RETURNING {LLM_CALL_COLUMNS}"
                ),
                params![
                    id,
                    call.org_id,
                    call.user_id,
                    call.context.feature,
                    context_json,
                    call.model,
                    request_json,
                    response_json,
                    call.input_tokens.map(|v| v as i64),
                    call.output_tokens.map(|v| v as i64),
                    call.latency_ms.map(|v| v as i64),
                    call.error,
                    call.trace_id,
                    now,
                ],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_llm_call(row),
            None => Err(LlmCallError::InsertFailed(
                "insert failed: no rows returned".into(),
            )),
        }
    }

    pub async fn list(
        &self,
        org_id: i64,
        user_id: i64,
        feature: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<LlmCall>, LlmCallError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {LLM_CALL_COLUMNS}
                     FROM llm_calls
                     WHERE org_id = ?1 AND user_id = ?2 AND (?3 IS NULL OR feature = ?3)
                     ORDER BY created_at DESC
                     LIMIT ?4"
                ),
                params![
                    org_id,
                    user_id,
                    feature,
                    limit.unwrap_or(DEFAULT_LIST_LIMIT)
                ],
            )
            .await?;

        let mut calls = Vec::new();
        while let Some(row) = rows.next().await? {
            calls.push(row_to_llm_call(row)?);
        }
        Ok(calls)
    }

    /// List LLM calls by org_id only, useful for admin auditing use cases
    /// where you want to see all calls within an org regardless of user.
    pub async fn list_by_org(
        &self,
        org_id: i64,
        feature: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<LlmCall>, LlmCallError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {LLM_CALL_COLUMNS}
                     FROM llm_calls
                     WHERE org_id = ?1 AND (?2 IS NULL OR feature = ?2)
                     ORDER BY created_at DESC
                     LIMIT ?3"
                ),
                params![org_id, feature, limit.unwrap_or(DEFAULT_LIST_LIMIT)],
            )
            .await?;

        let mut calls = Vec::new();
        while let Some(row) = rows.next().await? {
            calls.push(row_to_llm_call(row)?);
        }
        Ok(calls)
    }
}

fn row_to_llm_call(row: Row) -> Result<LlmCall, LlmCallError> {
    let context_json: String = row.get(4)?;
    let request_json: String = row.get(6)?;
    let response_json: Option<String> = row.get(7)?;
    let input_tokens: Option<i64> = row.get(8)?;
    let output_tokens: Option<i64> = row.get(9)?;
    let latency_ms: Option<i64> = row.get(10)?;
    let created_at: String = row.get(13)?;

    Ok(LlmCall {
        id: row.get(0)?,
        org_id: row.get(1)?,
        user_id: row.get(2)?,
        feature: row.get(3)?,
        context: serde_json::from_str(&context_json)?,
        model: row.get(5)?,
        request_json: serde_json::from_str(&request_json)?,
        response_json: response_json
            .as_deref()
            .map(serde_json::from_str)
            .transpose()?,
        input_tokens: input_tokens.and_then(|v| u32::try_from(v).ok()),
        output_tokens: output_tokens.and_then(|v| u32::try_from(v).ok()),
        latency_ms: latency_ms.and_then(|v| u64::try_from(v).ok()),
        error: row.get(11)?,
        trace_id: row.get(12)?,
        created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
    })
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
    use crate::migrations::run_migrations;
    use tempfile::TempDir;

    async fn setup_repo() -> (LlmCallRepository, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        let db_path = dir.path().join("db.sqlite");
        let db = Database::new(&db_path).await.expect("create db");
        run_migrations(&db).await.expect("migrations");
        (LlmCallRepository::new(db), dir)
    }

    fn sample_context() -> LlmCallContext {
        LlmCallContext {
            feature: "classification".into(),
            org_id: Some(DEFAULT_ORG_ID),
            user_id: Some(DEFAULT_USER_ID),
            account_id: Some("acc-1".into()),
            message_id: Some("msg-1".into()),
            thread_id: None,
            rule_name: Some("rule-a".into()),
            rule_id: None,
        }
    }

    #[tokio::test]
    async fn create_and_list_calls() {
        let (repo, _dir) = setup_repo().await;
        let new_call = NewLlmCall {
            org_id: DEFAULT_ORG_ID,
            user_id: DEFAULT_USER_ID,
            context: sample_context(),
            model: "openai::gpt-4o-mini".into(),
            request_json: serde_json::json!({"messages": [{"role": "user", "content": "hi"}]}),
            response_json: Some(serde_json::json!({"content": "hello"})),
            input_tokens: Some(10),
            output_tokens: Some(2),
            latency_ms: Some(120),
            error: None,
            trace_id: Some("trace-1".into()),
        };

        let created = repo.create(new_call.clone()).await.expect("create");
        assert_eq!(created.feature, new_call.context.feature);
        assert_eq!(created.model, new_call.model);
        assert_eq!(created.input_tokens, new_call.input_tokens);
        assert_eq!(created.output_tokens, new_call.output_tokens);
        assert_eq!(created.latency_ms, new_call.latency_ms);
        assert_eq!(created.trace_id, new_call.trace_id);
        assert!(created.error.is_none());
        assert!(created.created_at <= Utc::now());

        let list = repo
            .list(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                Some("classification"),
                Some(10),
            )
            .await
            .expect("list");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, created.id);
        assert_eq!(list[0].context.account_id, Some("acc-1".into()));
    }

    #[tokio::test]
    async fn list_filters_by_feature() {
        let (repo, _dir) = setup_repo().await;
        let base = NewLlmCall {
            org_id: DEFAULT_ORG_ID,
            user_id: DEFAULT_USER_ID,
            context: LlmCallContext::new("classification"),
            model: "openai::gpt-4o-mini".into(),
            request_json: serde_json::json!({"messages": []}),
            response_json: None,
            input_tokens: None,
            output_tokens: None,
            latency_ms: None,
            error: Some("failed".into()),
            trace_id: None,
        };

        repo.create(base.clone()).await.expect("create first");

        let mut second = base.clone();
        second.context.feature = "rules_assistant".into();
        repo.create(second).await.expect("create second");

        let classification = repo
            .list(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                Some("classification"),
                Some(10),
            )
            .await
            .expect("list classification");
        assert_eq!(classification.len(), 1);
        assert_eq!(classification[0].feature, "classification");

        let rules = repo
            .list(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                Some("rules_assistant"),
                Some(10),
            )
            .await
            .expect("list rules");
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].feature, "rules_assistant");
    }

    #[tokio::test]
    async fn list_by_org_returns_all_users_within_org() {
        let (repo, _dir) = setup_repo().await;

        // Create calls for user 1
        let call_user1 = NewLlmCall {
            org_id: DEFAULT_ORG_ID,
            user_id: DEFAULT_USER_ID,
            context: LlmCallContext::new("classification"),
            model: "openai::gpt-4o-mini".into(),
            request_json: serde_json::json!({"messages": []}),
            response_json: None,
            input_tokens: None,
            output_tokens: None,
            latency_ms: None,
            error: None,
            trace_id: None,
        };
        repo.create(call_user1).await.expect("create user1 call");

        // Create calls for user 2 in the same org
        let call_user2 = NewLlmCall {
            org_id: DEFAULT_ORG_ID,
            user_id: DEFAULT_USER_ID + 1,
            context: LlmCallContext::new("rules_assistant"),
            model: "openai::gpt-4o-mini".into(),
            request_json: serde_json::json!({"messages": []}),
            response_json: None,
            input_tokens: None,
            output_tokens: None,
            latency_ms: None,
            error: None,
            trace_id: None,
        };
        repo.create(call_user2).await.expect("create user2 call");

        // Create call for a different org (should not be included)
        let call_other_org = NewLlmCall {
            org_id: DEFAULT_ORG_ID + 1,
            user_id: DEFAULT_USER_ID,
            context: LlmCallContext::new("classification"),
            model: "openai::gpt-4o-mini".into(),
            request_json: serde_json::json!({"messages": []}),
            response_json: None,
            input_tokens: None,
            output_tokens: None,
            latency_ms: None,
            error: None,
            trace_id: None,
        };
        repo.create(call_other_org)
            .await
            .expect("create other org call");

        // list_by_org should return both users within the org
        let all_org_calls = repo
            .list_by_org(DEFAULT_ORG_ID, None, Some(10))
            .await
            .expect("list_by_org");
        assert_eq!(all_org_calls.len(), 2);

        // Verify both users are represented
        let user_ids: Vec<i64> = all_org_calls.iter().map(|c| c.user_id).collect();
        assert!(user_ids.contains(&DEFAULT_USER_ID));
        assert!(user_ids.contains(&(DEFAULT_USER_ID + 1)));

        // list_by_org with feature filter
        let classification_calls = repo
            .list_by_org(DEFAULT_ORG_ID, Some("classification"), Some(10))
            .await
            .expect("list_by_org classification");
        assert_eq!(classification_calls.len(), 1);
        assert_eq!(classification_calls[0].feature, "classification");

        // Original list method should still filter by user
        let user1_only = repo
            .list(DEFAULT_ORG_ID, DEFAULT_USER_ID, None, Some(10))
            .await
            .expect("list user1");
        assert_eq!(user1_only.len(), 1);
        assert_eq!(user1_only[0].user_id, DEFAULT_USER_ID);
    }
}
