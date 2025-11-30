use chrono::{DateTime, SecondsFormat, Utc};
use libsql::{Row, params};
use thiserror::Error;
use uuid::Uuid;

use crate::db::{Database, DbError};

use super::types::{
    Action, ActionLink, ActionLinkRelationType, ActionStatus, Decision, DecisionSource, NewAction,
    NewActionLink, NewDecision,
};

const DECISION_COLUMNS: &str = "id, account_id, message_id, source, decision_json, action_type, confidence, needs_approval, rationale, telemetry_json, created_at, updated_at";
const ACTION_COLUMNS: &str = "id, account_id, message_id, decision_id, action_type, parameters_json, status, error_message, executed_at, undo_hint_json, trace_id, created_at, updated_at";
const ACTION_LINK_COLUMNS: &str = "id, cause_action_id, effect_action_id, relation_type";
const RECENT_DECISION_LIMIT: i64 = 50;

#[derive(Debug, Error)]
pub enum DecisionError {
    #[error("database error: {0}")]
    Database(#[from] DbError),
    #[error("sql error: {0}")]
    Sql(#[from] libsql::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("datetime parse error: {0}")]
    DateTimeParse(#[from] chrono::ParseError),
    #[error("decision not found: {0}")]
    NotFound(String),
    #[error("invalid source value {0}")]
    InvalidSource(String),
}

#[derive(Debug, Error)]
pub enum ActionError {
    #[error("database error: {0}")]
    Database(#[from] DbError),
    #[error("sql error: {0}")]
    Sql(#[from] libsql::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("datetime parse error: {0}")]
    DateTimeParse(#[from] chrono::ParseError),
    #[error("action not found: {0}")]
    NotFound(String),
    #[error("invalid status value {0}")]
    InvalidStatus(String),
    #[error("invalid initial status {0:?}")]
    InvalidInitialStatus(ActionStatus),
    #[error("invalid status transition from {from:?} to {to:?}")]
    InvalidStatusTransition {
        from: ActionStatus,
        to: ActionStatus,
    },
}

#[derive(Debug, Error)]
pub enum ActionLinkError {
    #[error("database error: {0}")]
    Database(#[from] DbError),
    #[error("sql error: {0}")]
    Sql(#[from] libsql::Error),
    #[error("action link not found: {0}")]
    NotFound(String),
    #[error("invalid relation type {0}")]
    InvalidRelationType(String),
}

#[derive(Clone)]
pub struct DecisionRepository {
    db: Database,
}

impl DecisionRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn create(&self, new_decision: NewDecision) -> Result<Decision, DecisionError> {
        let id = Uuid::new_v4().to_string();
        let now = now_rfc3339();
        let decision_json = serde_json::to_string(&new_decision.decision_json)?;
        let telemetry_json = serde_json::to_string(&new_decision.telemetry_json)?;
        let needs_approval = new_decision.needs_approval as i64;

        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "INSERT INTO decisions (
                        id, account_id, message_id, source, decision_json, action_type, confidence, needs_approval, rationale, telemetry_json, created_at, updated_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)
                    RETURNING {DECISION_COLUMNS}"
                ),
                params![
                    id,
                    new_decision.account_id,
                    new_decision.message_id,
                    new_decision.source.as_str(),
                    decision_json,
                    new_decision.action_type,
                    new_decision.confidence,
                    needs_approval,
                    new_decision.rationale,
                    telemetry_json,
                    now
                ],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_decision(row),
            None => Err(DecisionError::NotFound("insert failed".into())),
        }
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Decision, DecisionError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!("SELECT {DECISION_COLUMNS} FROM decisions WHERE id = ?1"),
                params![id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_decision(row),
            None => Err(DecisionError::NotFound(id.to_string())),
        }
    }

    pub async fn get_by_message_id(&self, message_id: &str) -> Result<Decision, DecisionError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {DECISION_COLUMNS}
                     FROM decisions
                     WHERE message_id = ?1
                     ORDER BY created_at DESC
                     LIMIT 1"
                ),
                params![message_id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_decision(row),
            None => Err(DecisionError::NotFound(message_id.to_string())),
        }
    }

    pub async fn list_by_account(&self, account_id: &str) -> Result<Vec<Decision>, DecisionError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {DECISION_COLUMNS}
                     FROM decisions
                     WHERE account_id = ?1
                     ORDER BY created_at DESC"
                ),
                params![account_id],
            )
            .await?;

        let mut decisions = Vec::new();
        while let Some(row) = rows.next().await? {
            decisions.push(row_to_decision(row)?);
        }
        Ok(decisions)
    }

    pub async fn list_recent(&self, account_id: &str) -> Result<Vec<Decision>, DecisionError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {DECISION_COLUMNS}
                     FROM decisions
                     WHERE account_id = ?1
                     ORDER BY created_at DESC
                     LIMIT ?2"
                ),
                params![account_id, RECENT_DECISION_LIMIT],
            )
            .await?;

        let mut decisions = Vec::new();
        while let Some(row) = rows.next().await? {
            decisions.push(row_to_decision(row)?);
        }
        Ok(decisions)
    }
}

#[derive(Clone)]
pub struct ActionRepository {
    db: Database,
}

impl ActionRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn create(&self, new_action: NewAction) -> Result<Action, ActionError> {
        let initial_status = new_action.status.clone();
        if !is_valid_initial_status(&initial_status) {
            return Err(ActionError::InvalidInitialStatus(initial_status));
        }

        let id = Uuid::new_v4().to_string();
        let now = now_rfc3339();
        let parameters_json = serde_json::to_string(&new_action.parameters_json)?;
        let undo_hint_json = serde_json::to_string(&new_action.undo_hint_json)?;
        let status = initial_status.as_str();
        let executed_at = new_action.executed_at.map(to_rfc3339);

        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "INSERT INTO actions (
                        id, account_id, message_id, decision_id, action_type, parameters_json, status, error_message, executed_at, undo_hint_json, trace_id, created_at, updated_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)
                    RETURNING {ACTION_COLUMNS}"
                ),
                params![
                    id,
                    new_action.account_id,
                    new_action.message_id,
                    new_action.decision_id,
                    new_action.action_type,
                    parameters_json,
                    status,
                    new_action.error_message,
                    executed_at,
                    undo_hint_json,
                    new_action.trace_id,
                    now
                ],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_action(row),
            None => Err(ActionError::NotFound("insert failed".into())),
        }
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Action, ActionError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!("SELECT {ACTION_COLUMNS} FROM actions WHERE id = ?1"),
                params![id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_action(row),
            None => Err(ActionError::NotFound(id.to_string())),
        }
    }

    pub async fn get_by_decision_id(&self, decision_id: &str) -> Result<Vec<Action>, ActionError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {ACTION_COLUMNS}
                     FROM actions
                     WHERE decision_id = ?1
                     ORDER BY created_at"
                ),
                params![decision_id],
            )
            .await?;

        let mut actions = Vec::new();
        while let Some(row) = rows.next().await? {
            actions.push(row_to_action(row)?);
        }
        Ok(actions)
    }

    pub async fn list_by_message_id(&self, message_id: &str) -> Result<Vec<Action>, ActionError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {ACTION_COLUMNS}
                     FROM actions
                     WHERE message_id = ?1
                     ORDER BY created_at"
                ),
                params![message_id],
            )
            .await?;

        let mut actions = Vec::new();
        while let Some(row) = rows.next().await? {
            actions.push(row_to_action(row)?);
        }
        Ok(actions)
    }

    pub async fn list_by_status(&self, status: ActionStatus) -> Result<Vec<Action>, ActionError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {ACTION_COLUMNS}
                     FROM actions
                     WHERE status = ?1
                     ORDER BY created_at"
                ),
                params![status.as_str()],
            )
            .await?;

        let mut actions = Vec::new();
        while let Some(row) = rows.next().await? {
            actions.push(row_to_action(row)?);
        }
        Ok(actions)
    }

    pub async fn update_status(
        &self,
        id: &str,
        next_status: ActionStatus,
        error_message: Option<String>,
        executed_at: Option<DateTime<Utc>>,
    ) -> Result<Action, ActionError> {
        let current = self.get_by_id(id).await?;
        if !is_valid_transition(&current.status, &next_status) {
            return Err(ActionError::InvalidStatusTransition {
                from: current.status,
                to: next_status,
            });
        }

        let now = now_rfc3339();
        let executed_at_to_set = match (current.executed_at.clone(), executed_at) {
            (Some(existing), _) => Some(existing),
            (None, Some(new_value)) => Some(new_value),
            (None, None) => None,
        };
        let executed_at_str = executed_at_to_set.map(to_rfc3339);

        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "UPDATE actions
                     SET status = ?1,
                         error_message = ?2,
                         executed_at = COALESCE(?3, executed_at),
                         updated_at = ?4
                     WHERE id = ?5 AND status = ?6
                     RETURNING {ACTION_COLUMNS}"
                ),
                params![
                    next_status.as_str(),
                    error_message,
                    executed_at_str,
                    now,
                    id,
                    current.status.as_str(),
                ],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_action(row),
            None => match self.get_by_id(id).await {
                Ok(latest) => Err(ActionError::InvalidStatusTransition {
                    from: latest.status,
                    to: next_status,
                }),
                Err(err @ ActionError::NotFound(_)) => Err(err),
                Err(err) => Err(err),
            },
        }
    }

    pub async fn mark_executing(&self, id: &str) -> Result<Action, ActionError> {
        self.update_status(id, ActionStatus::Executing, None, Some(Utc::now()))
            .await
    }

    pub async fn mark_completed(&self, id: &str) -> Result<Action, ActionError> {
        self.update_status(id, ActionStatus::Completed, None, Some(Utc::now()))
            .await
    }

    pub async fn mark_failed(
        &self,
        id: &str,
        error_message: String,
    ) -> Result<Action, ActionError> {
        self.update_status(
            id,
            ActionStatus::Failed,
            Some(error_message),
            Some(Utc::now()),
        )
        .await
    }
}

#[derive(Clone)]
pub struct ActionLinkRepository {
    db: Database,
}

impl ActionLinkRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn create(&self, new_link: NewActionLink) -> Result<ActionLink, ActionLinkError> {
        let id = Uuid::new_v4().to_string();
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "INSERT INTO action_links (id, cause_action_id, effect_action_id, relation_type)
                     VALUES (?1, ?2, ?3, ?4)
                     RETURNING {ACTION_LINK_COLUMNS}"
                ),
                params![
                    id,
                    new_link.cause_action_id,
                    new_link.effect_action_id,
                    new_link.relation_type.as_str()
                ],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_action_link(row),
            None => Err(ActionLinkError::NotFound("insert failed".into())),
        }
    }

    pub async fn get_by_cause_action_id(
        &self,
        cause_action_id: &str,
    ) -> Result<Vec<ActionLink>, ActionLinkError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {ACTION_LINK_COLUMNS}
                     FROM action_links
                     WHERE cause_action_id = ?1"
                ),
                params![cause_action_id],
            )
            .await?;

        let mut links = Vec::new();
        while let Some(row) = rows.next().await? {
            links.push(row_to_action_link(row)?);
        }
        Ok(links)
    }

    pub async fn get_by_effect_action_id(
        &self,
        effect_action_id: &str,
    ) -> Result<Vec<ActionLink>, ActionLinkError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "SELECT {ACTION_LINK_COLUMNS}
                     FROM action_links
                     WHERE effect_action_id = ?1"
                ),
                params![effect_action_id],
            )
            .await?;

        let mut links = Vec::new();
        while let Some(row) = rows.next().await? {
            links.push(row_to_action_link(row)?);
        }
        Ok(links)
    }

    pub async fn delete(&self, id: &str) -> Result<(), ActionLinkError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                "DELETE FROM action_links WHERE id = ?1 RETURNING id",
                params![id],
            )
            .await?;

        match rows.next().await? {
            Some(_) => Ok(()),
            None => Err(ActionLinkError::NotFound(id.to_string())),
        }
    }
}

fn is_valid_transition(current: &ActionStatus, next: &ActionStatus) -> bool {
    use ActionStatus::*;
    match current {
        Queued => matches!(
            next,
            Executing | Canceled | Rejected | ApprovedPending | Failed
        ),
        Executing => matches!(next, Completed | Failed | Canceled),
        ApprovedPending => matches!(next, Queued | Canceled | Rejected),
        Completed | Failed | Canceled | Rejected => false,
    }
}

fn is_valid_initial_status(status: &ActionStatus) -> bool {
    matches!(
        status,
        ActionStatus::Queued | ActionStatus::Executing | ActionStatus::ApprovedPending
    )
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn to_rfc3339(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn row_to_decision(row: Row) -> Result<Decision, DecisionError> {
    let source: String = row.get(3)?;
    let decision_json: String = row.get(4)?;
    let needs_approval: i64 = row.get(7)?;
    let telemetry_json: String = row.get(9)?;
    let created_at: String = row.get(10)?;
    let updated_at: String = row.get(11)?;

    let source = DecisionSource::from_str(&source)
        .ok_or_else(|| DecisionError::InvalidSource(source.clone()))?;

    Ok(Decision {
        id: row.get(0)?,
        account_id: row.get(1)?,
        message_id: row.get(2)?,
        source,
        decision_json: serde_json::from_str(&decision_json)?,
        action_type: row.get(5)?,
        confidence: row.get(6)?,
        needs_approval: needs_approval != 0,
        rationale: row.get(8)?,
        telemetry_json: serde_json::from_str(&telemetry_json)?,
        created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at)?.with_timezone(&Utc),
    })
}

fn row_to_action(row: Row) -> Result<Action, ActionError> {
    let parameters_json: String = row.get(5)?;
    let status: String = row.get(6)?;
    let executed_at: Option<String> = row.get(8)?;
    let undo_hint_json: String = row.get(9)?;
    let created_at: String = row.get(11)?;
    let updated_at: String = row.get(12)?;

    let status = ActionStatus::from_str(&status)
        .ok_or_else(|| ActionError::InvalidStatus(status.clone()))?;

    Ok(Action {
        id: row.get(0)?,
        account_id: row.get(1)?,
        message_id: row.get(2)?,
        decision_id: row.get(3)?,
        action_type: row.get(4)?,
        parameters_json: serde_json::from_str(&parameters_json)?,
        status,
        error_message: row.get(7)?,
        executed_at: match executed_at {
            Some(value) => Some(DateTime::parse_from_rfc3339(&value)?.with_timezone(&Utc)),
            None => None,
        },
        undo_hint_json: serde_json::from_str(&undo_hint_json)?,
        trace_id: row.get(10)?,
        created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at)?.with_timezone(&Utc),
    })
}

fn row_to_action_link(row: Row) -> Result<ActionLink, ActionLinkError> {
    let relation_type: String = row.get(3)?;
    let relation_type = ActionLinkRelationType::from_str(&relation_type)
        .ok_or_else(|| ActionLinkError::InvalidRelationType(relation_type.clone()))?;

    Ok(ActionLink {
        id: row.get(0)?,
        cause_action_id: row.get(1)?,
        effect_action_id: row.get(2)?,
        relation_type,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::{AccountConfig, AccountRepository, PubsubConfig};
    use crate::gmail::OAuthTokens;
    use crate::migrations::run_migrations;
    use crate::threads::ThreadRepository;
    use crate::{Mailbox, MessageRepository, NewMessage};
    use libsql::params;
    use tempfile::TempDir;

    async fn setup_db() -> (Database, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        let db_name = format!("db_{}.sqlite", uuid::Uuid::new_v4());
        let db_path = dir.path().join(db_name);
        let db = Database::new(&db_path).await.expect("create db");
        run_migrations(&db).await.expect("migrations");
        (db, dir)
    }

    async fn seed_account(db: &Database) -> String {
        seed_account_with_email(db, "user@example.com").await
    }

    async fn seed_account_with_email(db: &Database, email: &str) -> String {
        let repo = AccountRepository::new(db.clone());
        let config = AccountConfig {
            client_id: "client".into(),
            client_secret: "secret".into(),
            oauth: OAuthTokens {
                access_token: "access".into(),
                refresh_token: "refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            },
            pubsub: PubsubConfig::default(),
        };
        repo.create(email, Some("User".into()), config)
            .await
            .expect("create account")
            .id
    }

    async fn seed_thread(db: &Database, account_id: &str, provider_thread_id: &str) -> String {
        let repo = ThreadRepository::new(db.clone());
        repo.upsert(
            account_id,
            provider_thread_id,
            Some("Subject".into()),
            Some("Snippet".into()),
            Some(Utc::now()),
            serde_json::json!({"raw": true}),
        )
        .await
        .expect("create thread")
        .id
    }

    async fn seed_message(
        db: &Database,
        account_id: &str,
        thread_id: &str,
        provider_message_id: &str,
    ) -> String {
        let repo = MessageRepository::new(db.clone());
        let msg = NewMessage {
            account_id: account_id.to_string(),
            thread_id: thread_id.to_string(),
            provider_message_id: provider_message_id.to_string(),
            from_email: Some("alice@example.com".into()),
            from_name: Some("Alice".into()),
            to: vec![Mailbox {
                email: "bob@example.com".into(),
                name: Some("Bob".into()),
            }],
            cc: vec![],
            bcc: vec![],
            subject: Some("Hello".into()),
            snippet: Some("Snippet".into()),
            received_at: Some(Utc::now()),
            internal_date: Some(Utc::now()),
            labels: vec!["INBOX".into()],
            headers: serde_json::json!({"Header": "value"}),
            body_plain: Some("Hi there".into()),
            body_html: Some("<p>Hi there</p>".into()),
            raw_json: serde_json::json!({"raw": true}),
        };

        repo.upsert(msg).await.expect("create message").id
    }

    fn sample_new_decision(account_id: &str, message_id: &str) -> NewDecision {
        NewDecision {
            account_id: account_id.to_string(),
            message_id: message_id.to_string(),
            source: DecisionSource::Llm,
            decision_json: serde_json::json!({
                "decision": {
                    "action": "archive",
                    "parameters": {}
                }
            }),
            action_type: Some("archive".into()),
            confidence: Some(0.9),
            needs_approval: false,
            rationale: Some("safe".into()),
            telemetry_json: serde_json::json!({"model": "test"}),
        }
    }

    fn sample_new_action(
        account_id: &str,
        message_id: &str,
        decision_id: Option<&str>,
        status: ActionStatus,
    ) -> NewAction {
        NewAction {
            account_id: account_id.to_string(),
            message_id: message_id.to_string(),
            decision_id: decision_id.map(|s| s.to_string()),
            action_type: "archive".into(),
            parameters_json: serde_json::json!({"label": "INBOX"}),
            status,
            error_message: None,
            executed_at: None,
            undo_hint_json: serde_json::json!({"inverse_action": "unarchive"}),
            trace_id: Some("trace-1".into()),
        }
    }

    #[tokio::test]
    async fn decision_create_and_lookup() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "t1").await;
        let message_id = seed_message(&db, &account_id, &thread_id, "m1").await;
        let repo = DecisionRepository::new(db.clone());

        let created = repo
            .create(sample_new_decision(&account_id, &message_id))
            .await
            .expect("create decision");

        let by_id = repo.get_by_id(&created.id).await.expect("get by id");
        assert_eq!(created, by_id);

        let by_message = repo
            .get_by_message_id(&message_id)
            .await
            .expect("get by message");
        assert_eq!(by_message.id, created.id);
    }

    #[tokio::test]
    async fn decision_list_by_account_and_recent() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let other_account_id = seed_account_with_email(&db, "second@example.com").await;
        let thread_id = seed_thread(&db, &account_id, "t1").await;
        let message_id1 = seed_message(&db, &account_id, &thread_id, "m1").await;
        let message_id2 = seed_message(&db, &account_id, &thread_id, "m2").await;
        let other_thread = seed_thread(&db, &other_account_id, "t2").await;
        let other_message = seed_message(&db, &other_account_id, &other_thread, "m3").await;
        let repo = DecisionRepository::new(db.clone());

        repo.create(sample_new_decision(&account_id, &message_id1))
            .await
            .expect("create first");
        repo.create(sample_new_decision(&account_id, &message_id2))
            .await
            .expect("create second");
        repo.create(sample_new_decision(&other_account_id, &other_message))
            .await
            .expect("create other");

        let by_account = repo
            .list_by_account(&account_id)
            .await
            .expect("list by account");
        assert_eq!(by_account.len(), 2);

        let recent = repo.list_recent(&account_id).await.expect("recent");
        assert_eq!(recent.len(), 2);
        assert!(
            recent
                .iter()
                .all(|decision| decision.account_id == account_id)
        );
    }

    #[tokio::test]
    async fn action_create_and_listing() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "t1").await;
        let message_id = seed_message(&db, &account_id, &thread_id, "m1").await;
        let decisions = DecisionRepository::new(db.clone());
        let decision = decisions
            .create(sample_new_decision(&account_id, &message_id))
            .await
            .expect("decision");

        let repo = ActionRepository::new(db.clone());
        let a1 = repo
            .create(sample_new_action(
                &account_id,
                &message_id,
                Some(&decision.id),
                ActionStatus::Queued,
            ))
            .await
            .expect("create a1");
        let _a2 = repo
            .create(sample_new_action(
                &account_id,
                &message_id,
                Some(&decision.id),
                ActionStatus::Executing,
            ))
            .await
            .expect("create a2");

        let fetched = repo.get_by_id(&a1.id).await.expect("get by id");
        assert_eq!(a1.id, fetched.id);
        assert_eq!(fetched.status, ActionStatus::Queued);

        let by_decision = repo
            .get_by_decision_id(&decision.id)
            .await
            .expect("by decision");
        assert_eq!(by_decision.len(), 2);

        let queued = repo
            .list_by_status(ActionStatus::Queued)
            .await
            .expect("queued");
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].id, a1.id);
    }

    #[tokio::test]
    async fn action_status_transitions_enforced() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "t1").await;
        let message_id = seed_message(&db, &account_id, &thread_id, "m1").await;
        let decisions = DecisionRepository::new(db.clone());
        let decision = decisions
            .create(sample_new_decision(&account_id, &message_id))
            .await
            .expect("decision");
        let repo = ActionRepository::new(db);

        let created = repo
            .create(sample_new_action(
                &account_id,
                &message_id,
                Some(&decision.id),
                ActionStatus::Queued,
            ))
            .await
            .expect("create");

        let executing = repo
            .mark_executing(&created.id)
            .await
            .expect("mark executing");
        assert_eq!(executing.status, ActionStatus::Executing);
        assert!(executing.executed_at.is_some());

        let completed = repo
            .mark_completed(&created.id)
            .await
            .expect("mark completed");
        assert_eq!(completed.status, ActionStatus::Completed);

        let err = repo
            .update_status(&created.id, ActionStatus::Queued, None, None)
            .await
            .expect_err("should reject transition");
        assert!(matches!(err, ActionError::InvalidStatusTransition { .. }));
    }

    #[tokio::test]
    async fn action_mark_failed_sets_error() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "t1").await;
        let message_id = seed_message(&db, &account_id, &thread_id, "m1").await;
        let decisions = DecisionRepository::new(db.clone());
        let decision = decisions
            .create(sample_new_decision(&account_id, &message_id))
            .await
            .expect("decision");
        let repo = ActionRepository::new(db);

        let created = repo
            .create(sample_new_action(
                &account_id,
                &message_id,
                Some(&decision.id),
                ActionStatus::Queued,
            ))
            .await
            .expect("create");

        let failed = repo
            .mark_failed(&created.id, "boom".into())
            .await
            .expect("fail");
        assert_eq!(failed.status, ActionStatus::Failed);
        assert_eq!(failed.error_message.as_deref(), Some("boom"));
        assert!(failed.executed_at.is_some());
    }

    #[tokio::test]
    async fn action_links_crud() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "t1").await;
        let message_id = seed_message(&db, &account_id, &thread_id, "m1").await;
        let decisions = DecisionRepository::new(db.clone());
        let decision = decisions
            .create(sample_new_decision(&account_id, &message_id))
            .await
            .expect("decision");
        let actions = ActionRepository::new(db.clone());

        let cause = actions
            .create(sample_new_action(
                &account_id,
                &message_id,
                Some(&decision.id),
                ActionStatus::Queued,
            ))
            .await
            .expect("cause");
        let effect = actions
            .create(sample_new_action(
                &account_id,
                &message_id,
                Some(&decision.id),
                ActionStatus::Queued,
            ))
            .await
            .expect("effect");

        let links = ActionLinkRepository::new(db);
        let link = links
            .create(NewActionLink {
                cause_action_id: cause.id.clone(),
                effect_action_id: effect.id.clone(),
                relation_type: ActionLinkRelationType::UndoOf,
            })
            .await
            .expect("create link");

        let by_cause = links
            .get_by_cause_action_id(&cause.id)
            .await
            .expect("by cause");
        assert_eq!(by_cause.len(), 1);
        assert_eq!(by_cause[0].id, link.id);

        let by_effect = links
            .get_by_effect_action_id(&effect.id)
            .await
            .expect("by effect");
        assert_eq!(by_effect.len(), 1);
        assert_eq!(by_effect[0].id, link.id);

        links.delete(&link.id).await.expect("delete");
        let none = links
            .get_by_cause_action_id(&cause.id)
            .await
            .expect("by cause");
        assert!(none.is_empty());

        let err = links
            .delete(&link.id)
            .await
            .expect_err("second delete should fail");
        assert!(matches!(err, ActionLinkError::NotFound(_)));
    }

    #[tokio::test]
    async fn decision_not_found_errors() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "t1").await;
        let message_id = seed_message(&db, &account_id, &thread_id, "m1").await;
        let repo = DecisionRepository::new(db);

        let err = repo
            .get_by_id("missing")
            .await
            .expect_err("missing id should error");
        assert!(matches!(err, DecisionError::NotFound(_)));

        let err = repo
            .get_by_message_id(&message_id)
            .await
            .expect_err("missing message should error");
        assert!(matches!(err, DecisionError::NotFound(_)));
    }

    #[tokio::test]
    async fn decision_invalid_source_rejected_by_db() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "t1").await;
        let message_id = seed_message(&db, &account_id, &thread_id, "m1").await;
        let now = now_rfc3339();

        let conn = db.connection().await.expect("conn");
        let err = conn
            .execute(
            "INSERT INTO decisions (id, account_id, message_id, source, decision_json, action_type, confidence, needs_approval, rationale, telemetry_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'unknown', '{}', NULL, NULL, 0, NULL, '{}', ?4, ?4)",
            params!["bad_decision", account_id, message_id, now],
        )
        .await
        .expect_err("insert invalid source");
        assert!(
            format!("{err}").contains("CHECK constraint failed"),
            "expected constraint failure, got {err}"
        );
    }

    #[tokio::test]
    async fn action_not_found_and_invalid_status_errors() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "t1").await;
        let message_id = seed_message(&db, &account_id, &thread_id, "m1").await;
        let repo = ActionRepository::new(db.clone());

        let err = repo
            .get_by_id("missing")
            .await
            .expect_err("missing action should error");
        assert!(matches!(err, ActionError::NotFound(_)));

        let err = repo
            .update_status("missing", ActionStatus::Queued, None, None)
            .await
            .expect_err("missing update should error");
        assert!(matches!(err, ActionError::NotFound(_)));

        let now = now_rfc3339();
        let conn = db.connection().await.expect("conn");
        let err = conn
            .execute(
            "INSERT INTO actions (id, account_id, message_id, decision_id, action_type, parameters_json, status, error_message, executed_at, undo_hint_json, trace_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, 'archive', '{}', 'bogus', NULL, NULL, '{}', NULL, ?4, ?4)",
            params!["bad_action", account_id, message_id, now],
        )
        .await
        .expect_err("insert invalid action");
        assert!(
            format!("{err}").contains("CHECK constraint failed"),
            "expected constraint failure, got {err}"
        );
    }

    #[tokio::test]
    async fn action_create_rejects_terminal_initial_status() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "t1").await;
        let message_id = seed_message(&db, &account_id, &thread_id, "m1").await;
        let decisions = DecisionRepository::new(db.clone());
        let decision = decisions
            .create(sample_new_decision(&account_id, &message_id))
            .await
            .expect("decision");
        let repo = ActionRepository::new(db);

        let err = repo
            .create(sample_new_action(
                &account_id,
                &message_id,
                Some(&decision.id),
                ActionStatus::Completed,
            ))
            .await
            .expect_err("completed should be rejected");
        assert!(matches!(err, ActionError::InvalidInitialStatus(_)));
    }

    #[tokio::test]
    async fn action_list_by_message_and_approved_pending_transitions() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "t1").await;
        let message_id = seed_message(&db, &account_id, &thread_id, "m1").await;
        let decisions = DecisionRepository::new(db.clone());
        let decision = decisions
            .create(sample_new_decision(&account_id, &message_id))
            .await
            .expect("decision");
        let repo = ActionRepository::new(db);

        let a1 = repo
            .create(sample_new_action(
                &account_id,
                &message_id,
                Some(&decision.id),
                ActionStatus::ApprovedPending,
            ))
            .await
            .expect("approved pending");
        let a2 = repo
            .create(sample_new_action(
                &account_id,
                &message_id,
                Some(&decision.id),
                ActionStatus::Queued,
            ))
            .await
            .expect("queued");

        let by_message = repo
            .list_by_message_id(&message_id)
            .await
            .expect("by message");
        let ids: Vec<String> = by_message.iter().map(|a| a.id.clone()).collect();
        assert!(ids.contains(&a1.id));
        assert!(ids.contains(&a2.id));

        let queued = repo
            .update_status(&a1.id, ActionStatus::Queued, None, None)
            .await
            .expect("approved pending -> queued");
        assert_eq!(queued.status, ActionStatus::Queued);
        assert!(queued.executed_at.is_none());

        let err = repo
            .update_status(&queued.id, ActionStatus::Completed, None, None)
            .await
            .expect_err("queued -> completed should be rejected");
        assert!(matches!(err, ActionError::InvalidStatusTransition { .. }));
    }
}
