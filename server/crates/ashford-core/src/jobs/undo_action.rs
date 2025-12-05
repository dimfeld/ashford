use std::str::FromStr;

use chrono::Utc;
use libsql;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::{info, warn};

use crate::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
use crate::decisions::{
    Action, ActionLinkError, ActionLinkRelationType, ActionLinkRepository, ActionRepository,
    ActionStatus, NewAction, NewActionLink,
};
use crate::gmail::{GmailClientError, NoopTokenStore};
use crate::llm::decision::ActionType;
use crate::messages::{MessageError, MessageRepository};
use crate::queue::{JobQueue, QueueError};
use crate::{Job, JobError};

use super::action_gmail::create_gmail_client;
use super::{JobDispatcher, map_action_error, map_gmail_error};

pub const JOB_TYPE: &str = "undo.action";

#[derive(Debug, Deserialize)]
struct UndoPayload {
    pub account_id: String,
    pub original_action_id: String,
}

#[derive(Debug, PartialEq)]
enum UndoExecutionResult {
    Completed,
    NotFound(String),
}

pub async fn handle_undo_action(dispatcher: &JobDispatcher, job: Job) -> Result<(), JobError> {
    let payload: UndoPayload = serde_json::from_value(job.payload.clone())
        .map_err(|err| JobError::Fatal(format!("invalid undo.action payload: {err}")))?;

    let action_repo = ActionRepository::new(dispatcher.db.clone());
    let link_repo = ActionLinkRepository::new(dispatcher.db.clone());
    let message_repo = MessageRepository::new(dispatcher.db.clone());
    let queue = JobQueue::new(dispatcher.db.clone());

    let original_action = action_repo
        .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &payload.original_action_id)
        .await
        .map_err(|err| map_action_error("load original action", err))?;

    let existing_undo_action =
        find_existing_undo_action(&link_repo, &action_repo, &original_action.id).await?;

    if original_action.account_id != payload.account_id {
        return Err(JobError::Fatal(format!(
            "action {} belongs to account {}, not {}",
            payload.original_action_id, original_action.account_id, payload.account_id
        )));
    }

    validate_action_undoable(&original_action, existing_undo_action.as_ref())?;

    let inverse_action_str = original_action
        .undo_hint_json
        .get("inverse_action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| JobError::Fatal("undo_hint missing inverse_action".to_string()))?;
    let inverse_parameters = original_action
        .undo_hint_json
        .get("inverse_parameters")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let inverse_action = if inverse_action_str == "none" {
        None
    } else {
        Some(ActionType::from_str(inverse_action_str).map_err(|_| {
            JobError::Fatal(format!("unsupported inverse_action {}", inverse_action_str))
        })?)
    };

    let message = message_repo
        .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &original_action.message_id)
        .await
        .map_err(|err| map_message_error("load message", err))?;

    if message.account_id != payload.account_id {
        return Err(JobError::Fatal(format!(
            "message {} belongs to account {}, not {}",
            original_action.message_id, message.account_id, payload.account_id
        )));
    }

    let gmail_client: crate::gmail::GmailClient<NoopTokenStore> =
        create_gmail_client(dispatcher, &payload.account_id).await?;

    let undo_action = match existing_undo_action {
        Some(action) => action,
        None => {
            let created = action_repo
                .create(NewAction {
                    org_id: DEFAULT_ORG_ID,
                    user_id: DEFAULT_USER_ID,
                    account_id: payload.account_id.clone(),
                    message_id: original_action.message_id.clone(),
                    decision_id: original_action.decision_id.clone(),
                    action_type: format!("undo_{}", original_action.action_type),
                    parameters_json: json!({
                        "original_action_id": original_action.id,
                        "inverse_action": inverse_action_str,
                        "inverse_parameters": inverse_parameters,
                        "job_id": job.id,
                    }),
                    status: ActionStatus::Executing,
                    error_message: None,
                    executed_at: Some(Utc::now()),
                    undo_hint_json: json!({}),
                    trace_id: original_action.trace_id.clone(),
                })
                .await
                .map_err(|err| map_action_error("create undo action", err))?;

            ensure_undo_link(&link_repo, &action_repo, &original_action.id, created).await?
        }
    };

    validate_action_undoable(&original_action, Some(&undo_action))?;

    if undo_action.status == ActionStatus::Executing {
        match undo_owner_job_id(&undo_action) {
            Some(owner_job_id) if owner_job_id == job.id.as_str() => {}
            _ => {
                return Err(JobError::Fatal(format!(
                    "undo already in progress for action {}",
                    original_action.id
                )));
            }
        }
    }

    let execution_result = if original_action.action_type == "snooze" {
        undo_snooze(
            &queue,
            &gmail_client,
            &message.provider_message_id,
            &inverse_parameters,
        )
        .await
    } else if let Some(inverse_action) = inverse_action {
        execute_inverse_action(
            &gmail_client,
            &message.provider_message_id,
            inverse_action,
            &inverse_parameters,
        )
        .await
    } else {
        Err(JobError::Fatal(
            "no inverse action available for undo".to_string(),
        ))
    };

    match execution_result {
        Ok(UndoExecutionResult::Completed) => {
            let undo_hint = json!({"note": "undo action - not reversible"});
            action_repo
                .mark_completed_with_undo_hint(
                    DEFAULT_ORG_ID,
                    DEFAULT_USER_ID,
                    &undo_action.id,
                    undo_hint,
                )
                .await
                .map_err(|err| map_action_error("mark undo action completed", err))?;

            info!(
                account_id = %payload.account_id,
                undo_action_id = %undo_action.id,
                original_action_id = %original_action.id,
                "completed undo action"
            );
            Ok(())
        }
        Ok(UndoExecutionResult::NotFound(message)) => {
            warn!(
                account_id = %payload.account_id,
                undo_action_id = %undo_action.id,
                original_action_id = %original_action.id,
                "undo failed: {message}"
            );
            action_repo
                .mark_failed(DEFAULT_ORG_ID, DEFAULT_USER_ID, &undo_action.id, message)
                .await
                .map_err(|err| map_action_error("mark undo action failed", err))?;
            Ok(())
        }
        Err(err) => {
            let attempts_exhausted = job.attempts >= job.max_attempts;
            if !err.is_retryable() || attempts_exhausted {
                let _ = action_repo
                    .mark_failed(
                        DEFAULT_ORG_ID,
                        DEFAULT_USER_ID,
                        &undo_action.id,
                        err.to_string(),
                    )
                    .await;
            }
            Err(err)
        }
    }
}

async fn find_existing_undo_action(
    link_repo: &ActionLinkRepository,
    action_repo: &ActionRepository,
    original_action_id: &str,
) -> Result<Option<Action>, JobError> {
    let links = link_repo
        .get_by_effect_action_id(original_action_id)
        .await
        .map_err(|err| map_action_link_error("load undo links", err))?;

    let Some(link) = links
        .into_iter()
        .find(|link| link.relation_type == ActionLinkRelationType::UndoOf)
    else {
        return Ok(None);
    };

    let undo_action = action_repo
        .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &link.cause_action_id)
        .await
        .map_err(|err| map_action_error("load undo action", err))?;

    Ok(Some(undo_action))
}

async fn ensure_undo_link(
    link_repo: &ActionLinkRepository,
    action_repo: &ActionRepository,
    original_action_id: &str,
    undo_action: Action,
) -> Result<Action, JobError> {
    match link_repo
        .create(NewActionLink {
            cause_action_id: undo_action.id.clone(),
            effect_action_id: original_action_id.to_string(),
            relation_type: ActionLinkRelationType::UndoOf,
        })
        .await
    {
        Ok(_) => Ok(undo_action),
        Err(ActionLinkError::Sql(err)) if is_unique_violation(&err) => {
            let _ = action_repo
                .mark_failed(
                    DEFAULT_ORG_ID,
                    DEFAULT_USER_ID,
                    &undo_action.id,
                    "lost undo lock to concurrent job".to_string(),
                )
                .await;

            match find_existing_undo_action(link_repo, action_repo, original_action_id).await? {
                Some(existing) => Ok(existing),
                None => Err(JobError::retryable(
                    "undo lock conflict detected; existing undo link not found".to_string(),
                )),
            }
        }
        Err(err) => {
            let _ = action_repo
                .mark_failed(
                    DEFAULT_ORG_ID,
                    DEFAULT_USER_ID,
                    &undo_action.id,
                    format!("create undo link: {err}"),
                )
                .await;
            Err(map_action_link_error("create undo link", err))
        }
    }
}

fn undo_owner_job_id(action: &Action) -> Option<&str> {
    action
        .parameters_json
        .get("job_id")
        .and_then(|v| v.as_str())
}

fn is_unique_violation(err: &libsql::Error) -> bool {
    err.to_string()
        .to_ascii_lowercase()
        .contains("unique constraint failed")
}

fn validate_action_undoable(
    action: &Action,
    existing_undo_action: Option<&Action>,
) -> Result<(), JobError> {
    if action.status != ActionStatus::Completed {
        return Err(JobError::Fatal(format!(
            "action {} is not completed and cannot be undone",
            action.id
        )));
    }

    let hint = action
        .undo_hint_json
        .as_object()
        .ok_or_else(|| JobError::Fatal(format!("action {} has no undo_hint_json", action.id)))?;

    let inverse_action = hint
        .get("inverse_action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| JobError::Fatal("undo_hint missing inverse_action".to_string()))?;

    let irreversible = hint
        .get("irreversible")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if irreversible {
        return Err(JobError::Fatal(format!(
            "action {} is irreversible and cannot be undone",
            action.id
        )));
    }

    if inverse_action == "none" && action.action_type != "snooze" {
        return Err(JobError::Fatal(format!(
            "action {} does not support undo",
            action.id
        )));
    }

    if let Some(undo_action) = existing_undo_action {
        return match undo_action.status {
            ActionStatus::Completed => Err(JobError::Fatal(format!(
                "action {} has already been undone",
                action.id
            ))),
            ActionStatus::Failed | ActionStatus::Canceled | ActionStatus::Rejected => {
                Err(JobError::Fatal(format!(
                    "action {} undo already attempted (status {})",
                    action.id,
                    undo_action.status.as_str()
                )))
            }
            ActionStatus::Executing | ActionStatus::Queued | ActionStatus::ApprovedPending => {
                Ok(())
            }
        };
    }

    Ok(())
}

async fn execute_inverse_action(
    gmail_client: &crate::gmail::GmailClient<NoopTokenStore>,
    provider_message_id: &str,
    inverse_action: ActionType,
    inverse_parameters: &Value,
) -> Result<UndoExecutionResult, JobError> {
    let result = match inverse_action {
        ActionType::ApplyLabel => {
            let label = inverse_parameters
                .get("label")
                .and_then(|v| v.as_str())
                .filter(|v| !v.is_empty())
                .ok_or_else(|| {
                    JobError::Fatal("apply_label undo requires 'label' parameter".to_string())
                })?
                .to_string();

            gmail_client
                .modify_message(provider_message_id, Some(vec![label]), None)
                .await
        }
        ActionType::RemoveLabel => {
            let label = inverse_parameters
                .get("label")
                .and_then(|v| v.as_str())
                .filter(|v| !v.is_empty())
                .ok_or_else(|| {
                    JobError::Fatal("remove_label undo requires 'label' parameter".to_string())
                })?
                .to_string();

            gmail_client
                .modify_message(provider_message_id, None, Some(vec![label]))
                .await
        }
        ActionType::MarkRead => {
            gmail_client
                .modify_message(provider_message_id, None, Some(vec!["UNREAD".to_string()]))
                .await
        }
        ActionType::MarkUnread => {
            gmail_client
                .modify_message(provider_message_id, Some(vec!["UNREAD".to_string()]), None)
                .await
        }
        ActionType::Star => {
            gmail_client
                .modify_message(provider_message_id, Some(vec!["STARRED".to_string()]), None)
                .await
        }
        ActionType::Unstar => {
            gmail_client
                .modify_message(provider_message_id, None, Some(vec!["STARRED".to_string()]))
                .await
        }
        ActionType::Restore => gmail_client.untrash_message(provider_message_id).await,
        ActionType::Trash => gmail_client.trash_message(provider_message_id).await,
        other => {
            return Err(JobError::Fatal(format!(
                "inverse action {} is not supported for undo",
                other.as_str()
            )));
        }
    };

    match result {
        Ok(_) => Ok(UndoExecutionResult::Completed),
        Err(GmailClientError::Http(err)) if err.status() == Some(StatusCode::NOT_FOUND) => Ok(
            UndoExecutionResult::NotFound("gmail resource not found (404) during undo".to_string()),
        ),
        Err(err) => Err(map_gmail_error("execute inverse action", err)),
    }
}

async fn undo_snooze(
    queue: &JobQueue,
    gmail_client: &crate::gmail::GmailClient<NoopTokenStore>,
    provider_message_id: &str,
    inverse_parameters: &Value,
) -> Result<UndoExecutionResult, JobError> {
    if let Some(job_id) = inverse_parameters
        .get("cancel_unsnooze_job_id")
        .and_then(|v| v.as_str())
    {
        match queue.cancel(job_id).await {
            Ok(_) => {}
            Err(QueueError::NotRunning(_)) | Err(QueueError::JobNotFound(_)) => {}
            Err(err) => {
                return Err(JobError::retryable(format!(
                    "cancel unsnooze job {job_id}: {err}"
                )));
            }
        }
    }

    let add_labels = labels_from_array(inverse_parameters.get("add_labels"))?;
    let remove_labels = labels_from_array(inverse_parameters.get("remove_labels"))?;

    let result = gmail_client
        .modify_message(provider_message_id, add_labels, remove_labels)
        .await;

    match result {
        Ok(_) => Ok(UndoExecutionResult::Completed),
        Err(GmailClientError::Http(err)) if err.status() == Some(StatusCode::NOT_FOUND) => {
            Ok(UndoExecutionResult::NotFound(
                "gmail resource not found (404) during snooze undo".to_string(),
            ))
        }
        Err(err) => Err(map_gmail_error("undo snooze", err)),
    }
}

fn labels_from_array(value: Option<&Value>) -> Result<Option<Vec<String>>, JobError> {
    match value {
        None => Ok(None),
        Some(Value::Array(values)) => {
            let mut labels = Vec::new();
            for v in values {
                let label = v
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| JobError::Fatal("labels must be non-empty strings".into()))?;
                labels.push(label.to_string());
            }
            if labels.is_empty() {
                Ok(None)
            } else {
                Ok(Some(labels))
            }
        }
        _ => Err(JobError::Fatal(
            "labels must be provided as an array of strings".into(),
        )),
    }
}

fn map_message_error(context: &str, err: MessageError) -> JobError {
    match err {
        MessageError::NotFound(id) => JobError::Fatal(format!("{context}: message not found {id}")),
        MessageError::Database(err) => JobError::retryable(format!("{context}: db error {err}")),
        MessageError::Sql(err) => JobError::retryable(format!("{context}: db error {err}")),
        MessageError::Json(err) => JobError::Fatal(format!("{context}: decode error {err}")),
        MessageError::DateTimeParse(err) => {
            JobError::Fatal(format!("{context}: decode error {err}"))
        }
    }
}

fn map_action_link_error(context: &str, err: crate::decisions::ActionLinkError) -> JobError {
    match err {
        crate::decisions::ActionLinkError::Database(err) => {
            JobError::retryable(format!("{context}: db error {err}"))
        }
        crate::decisions::ActionLinkError::Sql(err) => {
            JobError::retryable(format!("{context}: db error {err}"))
        }
        crate::decisions::ActionLinkError::NotFound(id) => {
            JobError::Fatal(format!("{context}: action link not found {id}"))
        }
        crate::decisions::ActionLinkError::InvalidRelationType(value) => {
            JobError::Fatal(format!("{context}: invalid relation type {value}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::{AccountConfig, AccountRepository, PubsubConfig};
    use crate::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
    use crate::decisions::ActionLinkRepository;
    use crate::gmail::OAuthTokens;
    use crate::migrations::run_migrations;
    use crate::queue::JobQueue;
    use crate::threads::ThreadRepository;
    use crate::worker::JobExecutor;
    use crate::{JobContext, JobDispatcher, Mailbox, MessageRepository, NewMessage, PolicyConfig};
    use std::sync::Arc;
    use tempfile::TempDir;
    use uuid::Uuid;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_dispatcher(db: crate::Database) -> JobDispatcher {
        JobDispatcher::new(
            db,
            reqwest::Client::new(),
            Arc::new(crate::llm::MockLLMClient::new()),
            PolicyConfig::default(),
        )
    }

    async fn setup_account() -> (crate::Database, TempDir, String) {
        let dir = TempDir::new().expect("temp dir");
        let db_name = format!("db_{}.sqlite", Uuid::new_v4());
        let db_path = dir.path().join(db_name);
        let db = crate::Database::new(&db_path).await.expect("create db");
        run_migrations(&db).await.expect("migrations");

        let account_repo = AccountRepository::new(db.clone());
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

        let account = account_repo
            .create(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                "user@example.com",
                Some("User".into()),
                config,
            )
            .await
            .expect("create account");

        (db, dir, account.id)
    }

    async fn seed_message(
        db: &crate::Database,
        account_id: &str,
        provider_message_id: &str,
    ) -> String {
        let thread_repo = ThreadRepository::new(db.clone());
        let thread = thread_repo
            .upsert(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                account_id,
                "thread-1",
                Some("Subject".into()),
                Some("Snippet".into()),
                Some(Utc::now()),
                json!({}),
            )
            .await
            .expect("create thread");

        let message_repo = MessageRepository::new(db.clone());
        let message = message_repo
            .upsert(NewMessage {
                org_id: DEFAULT_ORG_ID,
                user_id: DEFAULT_USER_ID,
                account_id: account_id.to_string(),
                thread_id: thread.id,
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
                headers: vec![],
                body_plain: Some("Hi".into()),
                body_html: None,
                raw_json: json!({"raw": true}),
            })
            .await
            .expect("insert message");

        message.id
    }

    async fn seed_completed_action(
        db: &crate::Database,
        account_id: &str,
        message_id: &str,
        action_type: &str,
        undo_hint: Value,
    ) -> Action {
        let action_repo = ActionRepository::new(db.clone());
        let created = action_repo
            .create(NewAction {
                org_id: DEFAULT_ORG_ID,
                user_id: DEFAULT_USER_ID,
                account_id: account_id.to_string(),
                message_id: message_id.to_string(),
                decision_id: None,
                action_type: action_type.to_string(),
                parameters_json: json!({}),
                status: ActionStatus::Executing,
                error_message: None,
                executed_at: Some(Utc::now()),
                undo_hint_json: json!({}),
                trace_id: None,
            })
            .await
            .expect("create action");

        action_repo
            .mark_completed_with_undo_hint(DEFAULT_ORG_ID, DEFAULT_USER_ID, &created.id, undo_hint)
            .await
            .expect("complete action")
    }

    #[tokio::test]
    async fn undo_archive_restores_inbox() {
        let (db, _dir, account_id) = setup_account().await;
        let provider_message_id = "msg-1";
        let message_id = seed_message(&db, &account_id, provider_message_id).await;

        let undo_hint = json!({
            "inverse_action": "apply_label",
            "inverse_parameters": {"label": "INBOX"},
        });
        let original_action =
            seed_completed_action(&db, &account_id, &message_id, "archive", undo_hint).await;

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/gmail/v1/users/user@example.com/messages/{provider_message_id}/modify"
            )))
            .and(body_json(json!({"addLabelIds": ["INBOX"]})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": provider_message_id,
                "labelIds": ["INBOX"],
            })))
            .expect(1)
            .mount(&server)
            .await;

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "original_action_id": original_action.id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue undo job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");
        let dispatcher = make_dispatcher(db.clone())
            .with_gmail_api_base(format!("{}/gmail/v1/users", server.uri()));
        let ctx = JobContext::new(queue.clone(), job.clone());

        dispatcher.execute(job, ctx).await.expect("undo succeeds");

        let action_repo = ActionRepository::new(db.clone());
        let actions = action_repo
            .list_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("list actions");
        assert_eq!(actions.len(), 2);
        let undo_action = actions
            .iter()
            .find(|a| a.action_type == "undo_archive")
            .expect("undo action");
        assert_eq!(undo_action.status, ActionStatus::Completed);
        assert!(
            undo_action.undo_hint_json.get("note").is_some(),
            "undo hint stored"
        );

        let links = ActionLinkRepository::new(db)
            .get_by_effect_action_id(&original_action.id)
            .await
            .expect("links");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].relation_type, ActionLinkRelationType::UndoOf);
        assert_eq!(links[0].cause_action_id, undo_action.id);
    }

    #[tokio::test]
    async fn irreversible_actions_rejected() {
        let (db, _dir, account_id) = setup_account().await;
        let message_id = seed_message(&db, &account_id, "msg-2").await;

        let undo_hint = json!({
            "inverse_action": "none",
            "inverse_parameters": {"note": "cannot undo delete"},
            "irreversible": true
        });
        let original_action =
            seed_completed_action(&db, &account_id, &message_id, "delete", undo_hint).await;

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "original_action_id": original_action.id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue undo job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");
        let dispatcher = make_dispatcher(db.clone());
        let ctx = JobContext::new(queue.clone(), job.clone());

        let result = dispatcher.execute(job, ctx).await;
        assert!(matches!(result, Err(JobError::Fatal(_))));

        let action_repo = ActionRepository::new(db);
        let actions = action_repo
            .list_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("list actions");
        assert_eq!(actions.len(), 1, "no undo action should be created");
    }

    #[tokio::test]
    async fn already_undone_actions_are_rejected() {
        let (db, _dir, account_id) = setup_account().await;
        let message_id = seed_message(&db, &account_id, "msg-3").await;

        let undo_hint = json!({
            "inverse_action": "apply_label",
            "inverse_parameters": {"label": "INBOX"},
        });
        let original_action =
            seed_completed_action(&db, &account_id, &message_id, "archive", undo_hint).await;

        let link_repo = ActionLinkRepository::new(db.clone());
        // create a placeholder undo action to satisfy FK constraints
        let action_repo = ActionRepository::new(db.clone());
        let prior_undo = action_repo
            .create(NewAction {
                org_id: DEFAULT_ORG_ID,
                user_id: DEFAULT_USER_ID,
                account_id: account_id.clone(),
                message_id: message_id.clone(),
                decision_id: None,
                action_type: "undo_archive".into(),
                parameters_json: json!({}),
                status: ActionStatus::Executing,
                error_message: None,
                executed_at: Some(Utc::now()),
                undo_hint_json: json!({}),
                trace_id: None,
            })
            .await
            .expect("create prior undo action");
        link_repo
            .create(NewActionLink {
                cause_action_id: prior_undo.id,
                effect_action_id: original_action.id.clone(),
                relation_type: ActionLinkRelationType::UndoOf,
            })
            .await
            .expect("create link");

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "original_action_id": original_action.id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue undo job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");
        let dispatcher = make_dispatcher(db.clone());
        let ctx = JobContext::new(queue.clone(), job.clone());

        let result = dispatcher.execute(job, ctx).await;
        assert!(matches!(result, Err(JobError::Fatal(_))));
    }

    #[tokio::test]
    async fn undo_snooze_cancels_unsnooze_job_and_restores_labels() {
        let (db, _dir, account_id) = setup_account().await;
        let provider_message_id = "msg-4";
        let message_id = seed_message(&db, &account_id, provider_message_id).await;

        let queue = JobQueue::new(db.clone());
        let unsnooze_job_id = queue
            .enqueue(super::JOB_TYPE, json!({"test": true}), None, 0)
            .await
            .expect("enqueue unsnooze placeholder");

        let undo_hint = json!({
            "inverse_action": "none",
            "inverse_parameters": {
                "add_labels": ["INBOX"],
                "remove_labels": ["Label_Snoozed"],
                "cancel_unsnooze_job_id": unsnooze_job_id
            }
        });
        let original_action =
            seed_completed_action(&db, &account_id, &message_id, "snooze", undo_hint).await;

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/gmail/v1/users/user@example.com/messages/{provider_message_id}/modify"
            )))
            .and(body_json(json!({
                "addLabelIds": ["INBOX"],
                "removeLabelIds": ["Label_Snoozed"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": provider_message_id,
                "labelIds": ["INBOX"]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "original_action_id": original_action.id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue undo job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");
        let dispatcher = make_dispatcher(db.clone())
            .with_gmail_api_base(format!("{}/gmail/v1/users", server.uri()));
        let ctx = JobContext::new(queue.clone(), job.clone());

        dispatcher.execute(job, ctx).await.expect("undo snooze");

        let undo_action = ActionRepository::new(db.clone())
            .list_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("actions")
            .into_iter()
            .find(|a| a.action_type == "undo_snooze")
            .expect("undo action");
        assert_eq!(undo_action.status, ActionStatus::Completed);

        let canceled = queue
            .fetch_job(&unsnooze_job_id)
            .await
            .expect("fetch canceled job");
        assert_eq!(canceled.state, crate::queue::JobState::Canceled);

        let links = ActionLinkRepository::new(db)
            .get_by_effect_action_id(&original_action.id)
            .await
            .expect("links");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].relation_type, ActionLinkRelationType::UndoOf);
    }

    #[tokio::test]
    async fn undo_snooze_when_unsnooze_job_already_ran() {
        // Tests that snooze undo succeeds when the unsnooze job has already completed.
        // The code handles QueueError::NotRunning as a success case (line 447).
        let (db, _dir, account_id) = setup_account().await;
        let provider_message_id = "msg-4b";
        let message_id = seed_message(&db, &account_id, provider_message_id).await;

        let queue = JobQueue::new(db.clone());
        // Create an unsnooze job
        let unsnooze_job_id = queue
            .enqueue(super::JOB_TYPE, json!({"test": true}), None, 0)
            .await
            .expect("enqueue unsnooze placeholder");
        // Directly set job to completed state to simulate already-ran scenario
        // (This triggers NotRunning when cancel is attempted)
        {
            let conn = db.connection().await.expect("db connection");
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE jobs SET state = 'completed', finished_at = ?2, updated_at = ?2 WHERE id = ?1",
                libsql::params![unsnooze_job_id.clone(), now],
            )
            .await
            .expect("update job to completed");
        }

        let undo_hint = json!({
            "inverse_action": "none",
            "inverse_parameters": {
                "add_labels": ["INBOX"],
                "remove_labels": ["Label_Snoozed"],
                "cancel_unsnooze_job_id": unsnooze_job_id
            }
        });
        let original_action =
            seed_completed_action(&db, &account_id, &message_id, "snooze", undo_hint).await;

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/gmail/v1/users/user@example.com/messages/{provider_message_id}/modify"
            )))
            .and(body_json(json!({
                "addLabelIds": ["INBOX"],
                "removeLabelIds": ["Label_Snoozed"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": provider_message_id,
                "labelIds": ["INBOX"]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "original_action_id": original_action.id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue undo job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");
        let dispatcher = make_dispatcher(db.clone())
            .with_gmail_api_base(format!("{}/gmail/v1/users", server.uri()));
        let ctx = JobContext::new(queue.clone(), job.clone());

        // Should succeed even though the unsnooze job already ran
        dispatcher
            .execute(job, ctx)
            .await
            .expect("undo snooze succeeds when job already ran");

        let undo_action = ActionRepository::new(db.clone())
            .list_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("actions")
            .into_iter()
            .find(|a| a.action_type == "undo_snooze")
            .expect("undo action");
        assert_eq!(undo_action.status, ActionStatus::Completed);

        // Verify the unsnooze job is still completed (wasn't modified)
        let unsnooze_job = queue
            .fetch_job(&unsnooze_job_id)
            .await
            .expect("fetch unsnooze job");
        assert_eq!(unsnooze_job.state, crate::queue::JobState::Completed);

        let links = ActionLinkRepository::new(db)
            .get_by_effect_action_id(&original_action.id)
            .await
            .expect("links");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].relation_type, ActionLinkRelationType::UndoOf);
    }

    #[tokio::test]
    async fn undo_mark_read_marks_message_unread() {
        let (db, _dir, account_id) = setup_account().await;
        let provider_message_id = "msg-5";
        let message_id = seed_message(&db, &account_id, provider_message_id).await;

        let undo_hint = json!({
            "inverse_action": "mark_unread",
            "inverse_parameters": {}
        });
        let original_action =
            seed_completed_action(&db, &account_id, &message_id, "mark_read", undo_hint).await;

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/gmail/v1/users/user@example.com/messages/{provider_message_id}/modify"
            )))
            .and(body_json(json!({"addLabelIds": ["UNREAD"]})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": provider_message_id,
                "labelIds": ["UNREAD"]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "original_action_id": original_action.id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue undo job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");
        let dispatcher = make_dispatcher(db.clone())
            .with_gmail_api_base(format!("{}/gmail/v1/users", server.uri()));
        let ctx = JobContext::new(queue.clone(), job.clone());

        dispatcher.execute(job, ctx).await.expect("undo mark_read");

        let undo_action = ActionRepository::new(db.clone())
            .list_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("actions")
            .into_iter()
            .find(|a| a.action_type == "undo_mark_read")
            .expect("undo action");
        assert_eq!(undo_action.status, ActionStatus::Completed);

        let links = ActionLinkRepository::new(db)
            .get_by_effect_action_id(&original_action.id)
            .await
            .expect("links");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].relation_type, ActionLinkRelationType::UndoOf);
    }

    #[tokio::test]
    async fn undo_mark_unread_marks_message_read() {
        let (db, _dir, account_id) = setup_account().await;
        let provider_message_id = "msg-6";
        let message_id = seed_message(&db, &account_id, provider_message_id).await;

        let undo_hint = json!({
            "inverse_action": "mark_read",
            "inverse_parameters": {}
        });
        let original_action =
            seed_completed_action(&db, &account_id, &message_id, "mark_unread", undo_hint).await;

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/gmail/v1/users/user@example.com/messages/{provider_message_id}/modify"
            )))
            .and(body_json(json!({"removeLabelIds": ["UNREAD"]})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": provider_message_id,
                "labelIds": []
            })))
            .expect(1)
            .mount(&server)
            .await;

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "original_action_id": original_action.id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue undo job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");
        let dispatcher = make_dispatcher(db.clone())
            .with_gmail_api_base(format!("{}/gmail/v1/users", server.uri()));
        let ctx = JobContext::new(queue.clone(), job.clone());

        dispatcher
            .execute(job, ctx)
            .await
            .expect("undo mark_unread");

        let undo_action = ActionRepository::new(db.clone())
            .list_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("actions")
            .into_iter()
            .find(|a| a.action_type == "undo_mark_unread")
            .expect("undo action");
        assert_eq!(undo_action.status, ActionStatus::Completed);
    }

    #[tokio::test]
    async fn undo_remove_label_reapplies_label() {
        let (db, _dir, account_id) = setup_account().await;
        let provider_message_id = "msg-7";
        let message_id = seed_message(&db, &account_id, provider_message_id).await;

        let undo_hint = json!({
            "inverse_action": "apply_label",
            "inverse_parameters": {"label": "Project"}
        });
        let original_action =
            seed_completed_action(&db, &account_id, &message_id, "remove_label", undo_hint).await;

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/gmail/v1/users/user@example.com/messages/{provider_message_id}/modify"
            )))
            .and(body_json(json!({"addLabelIds": ["Project"]})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": provider_message_id,
                "labelIds": ["Project"]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "original_action_id": original_action.id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue undo job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");
        let dispatcher = make_dispatcher(db.clone())
            .with_gmail_api_base(format!("{}/gmail/v1/users", server.uri()));
        let ctx = JobContext::new(queue.clone(), job.clone());

        dispatcher
            .execute(job, ctx)
            .await
            .expect("undo remove_label");

        let undo_action = ActionRepository::new(db.clone())
            .list_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("actions")
            .into_iter()
            .find(|a| a.action_type == "undo_remove_label")
            .expect("undo action");
        assert_eq!(undo_action.status, ActionStatus::Completed);
    }

    #[tokio::test]
    async fn undo_apply_label_removes_label() {
        let (db, _dir, account_id) = setup_account().await;
        let provider_message_id = "msg-7b";
        let message_id = seed_message(&db, &account_id, provider_message_id).await;

        let undo_hint = json!({
            "inverse_action": "remove_label",
            "inverse_parameters": {"label": "Project"}
        });
        let original_action =
            seed_completed_action(&db, &account_id, &message_id, "apply_label", undo_hint).await;

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/gmail/v1/users/user@example.com/messages/{provider_message_id}/modify"
            )))
            .and(body_json(json!({"removeLabelIds": ["Project"]})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": provider_message_id,
                "labelIds": []
            })))
            .expect(1)
            .mount(&server)
            .await;

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "original_action_id": original_action.id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue undo job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");
        let dispatcher = make_dispatcher(db.clone())
            .with_gmail_api_base(format!("{}/gmail/v1/users", server.uri()));
        let ctx = JobContext::new(queue.clone(), job.clone());

        dispatcher
            .execute(job, ctx)
            .await
            .expect("undo apply_label");

        let undo_action = ActionRepository::new(db.clone())
            .list_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("actions")
            .into_iter()
            .find(|a| a.action_type == "undo_apply_label")
            .expect("undo action");
        assert_eq!(undo_action.status, ActionStatus::Completed);

        let links = ActionLinkRepository::new(db)
            .get_by_effect_action_id(&original_action.id)
            .await
            .expect("links");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].relation_type, ActionLinkRelationType::UndoOf);
        assert_eq!(links[0].cause_action_id, undo_action.id);
    }

    #[tokio::test]
    async fn undo_star_removes_star_label() {
        let (db, _dir, account_id) = setup_account().await;
        let provider_message_id = "msg-8";
        let message_id = seed_message(&db, &account_id, provider_message_id).await;

        let undo_hint = json!({
            "inverse_action": "unstar",
            "inverse_parameters": {}
        });
        let original_action =
            seed_completed_action(&db, &account_id, &message_id, "star", undo_hint).await;

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/gmail/v1/users/user@example.com/messages/{provider_message_id}/modify"
            )))
            .and(body_json(json!({"removeLabelIds": ["STARRED"]})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": provider_message_id,
                "labelIds": []
            })))
            .expect(1)
            .mount(&server)
            .await;

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "original_action_id": original_action.id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue undo job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");
        let dispatcher = make_dispatcher(db.clone())
            .with_gmail_api_base(format!("{}/gmail/v1/users", server.uri()));
        let ctx = JobContext::new(queue.clone(), job.clone());

        dispatcher.execute(job, ctx).await.expect("undo star");

        let undo_action = ActionRepository::new(db)
            .list_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("actions")
            .into_iter()
            .find(|a| a.action_type == "undo_star")
            .expect("undo action");
        assert_eq!(undo_action.status, ActionStatus::Completed);
    }

    #[tokio::test]
    async fn undo_unstar_adds_star_label() {
        let (db, _dir, account_id) = setup_account().await;
        let provider_message_id = "msg-9";
        let message_id = seed_message(&db, &account_id, provider_message_id).await;

        let undo_hint = json!({
            "inverse_action": "star",
            "inverse_parameters": {}
        });
        let original_action =
            seed_completed_action(&db, &account_id, &message_id, "unstar", undo_hint).await;

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/gmail/v1/users/user@example.com/messages/{provider_message_id}/modify"
            )))
            .and(body_json(json!({"addLabelIds": ["STARRED"]})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": provider_message_id,
                "labelIds": ["STARRED"]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "original_action_id": original_action.id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue undo job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");
        let dispatcher = make_dispatcher(db.clone())
            .with_gmail_api_base(format!("{}/gmail/v1/users", server.uri()));
        let ctx = JobContext::new(queue.clone(), job.clone());

        dispatcher.execute(job, ctx).await.expect("undo unstar");

        let undo_action = ActionRepository::new(db)
            .list_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("actions")
            .into_iter()
            .find(|a| a.action_type == "undo_unstar")
            .expect("undo action");
        assert_eq!(undo_action.status, ActionStatus::Completed);
    }

    #[tokio::test]
    async fn undo_trash_restores_message() {
        let (db, _dir, account_id) = setup_account().await;
        let provider_message_id = "msg-10";
        let message_id = seed_message(&db, &account_id, provider_message_id).await;

        let undo_hint = json!({
            "inverse_action": "restore",
            "inverse_parameters": {}
        });
        let original_action =
            seed_completed_action(&db, &account_id, &message_id, "trash", undo_hint).await;

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/gmail/v1/users/user@example.com/messages/{provider_message_id}/untrash"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": provider_message_id,
                "labelIds": ["INBOX"]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "original_action_id": original_action.id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue undo job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");
        let dispatcher = make_dispatcher(db.clone())
            .with_gmail_api_base(format!("{}/gmail/v1/users", server.uri()));
        let ctx = JobContext::new(queue.clone(), job.clone());

        dispatcher.execute(job, ctx).await.expect("undo trash");

        let undo_action = ActionRepository::new(db)
            .list_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("actions")
            .into_iter()
            .find(|a| a.action_type == "undo_trash")
            .expect("undo action");
        assert_eq!(undo_action.status, ActionStatus::Completed);
    }

    #[tokio::test]
    async fn undo_restore_moves_message_back_to_trash() {
        let (db, _dir, account_id) = setup_account().await;
        let provider_message_id = "msg-11";
        let message_id = seed_message(&db, &account_id, provider_message_id).await;

        let undo_hint = json!({
            "inverse_action": "trash",
            "inverse_parameters": {}
        });
        let original_action =
            seed_completed_action(&db, &account_id, &message_id, "restore", undo_hint).await;

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/gmail/v1/users/user@example.com/messages/{provider_message_id}/trash"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": provider_message_id,
                "labelIds": ["TRASH"]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "original_action_id": original_action.id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue undo job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");
        let dispatcher = make_dispatcher(db.clone())
            .with_gmail_api_base(format!("{}/gmail/v1/users", server.uri()));
        let ctx = JobContext::new(queue.clone(), job.clone());

        dispatcher.execute(job, ctx).await.expect("undo restore");

        let undo_action = ActionRepository::new(db)
            .list_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("actions")
            .into_iter()
            .find(|a| a.action_type == "undo_restore")
            .expect("undo action");
        assert_eq!(undo_action.status, ActionStatus::Completed);
    }

    #[tokio::test]
    async fn non_completed_actions_are_rejected_before_gmail_call() {
        let (db, _dir, account_id) = setup_account().await;
        let message_id = seed_message(&db, &account_id, "msg-12").await;

        let action_repo = ActionRepository::new(db.clone());
        let original_action = action_repo
            .create(NewAction {
                org_id: DEFAULT_ORG_ID,
                user_id: DEFAULT_USER_ID,
                account_id: account_id.clone(),
                message_id: message_id.clone(),
                decision_id: None,
                action_type: "archive".into(),
                parameters_json: json!({}),
                status: ActionStatus::Executing,
                error_message: None,
                executed_at: Some(Utc::now()),
                undo_hint_json: json!({
                    "inverse_action": "apply_label",
                    "inverse_parameters": {"label": "INBOX"}
                }),
                trace_id: None,
            })
            .await
            .expect("create action");

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "original_action_id": original_action.id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue undo job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");
        let dispatcher = make_dispatcher(db.clone());
        let ctx = JobContext::new(queue.clone(), job.clone());

        let result = dispatcher.execute(job, ctx).await;
        assert!(matches!(result, Err(JobError::Fatal(_))));

        let actions = ActionRepository::new(db)
            .list_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("actions");
        assert_eq!(actions.len(), 1, "undo action should not be created");
    }

    #[tokio::test]
    async fn gmail_404_marks_undo_action_failed() {
        let (db, _dir, account_id) = setup_account().await;
        let provider_message_id = "msg-13";
        let message_id = seed_message(&db, &account_id, provider_message_id).await;

        let undo_hint = json!({
            "inverse_action": "mark_unread",
            "inverse_parameters": {}
        });
        let original_action =
            seed_completed_action(&db, &account_id, &message_id, "mark_read", undo_hint).await;

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/gmail/v1/users/user@example.com/messages/{provider_message_id}/modify"
            )))
            .respond_with(ResponseTemplate::new(404))
            .expect(1)
            .mount(&server)
            .await;

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "original_action_id": original_action.id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue undo job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");
        let dispatcher = make_dispatcher(db.clone())
            .with_gmail_api_base(format!("{}/gmail/v1/users", server.uri()));
        let ctx = JobContext::new(queue.clone(), job.clone());

        dispatcher
            .execute(job, ctx)
            .await
            .expect("undo handles 404");

        let actions = ActionRepository::new(db.clone())
            .list_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("actions");
        assert_eq!(actions.len(), 2);
        let undo_action = actions
            .into_iter()
            .find(|a| a.action_type == "undo_mark_read")
            .expect("undo action");
        assert_eq!(undo_action.status, ActionStatus::Failed);
        assert!(
            undo_action
                .error_message
                .as_deref()
                .unwrap_or_default()
                .contains("gmail resource not found"),
            "error message stored"
        );

        let links = ActionLinkRepository::new(db)
            .get_by_effect_action_id(&original_action.id)
            .await
            .expect("links");
        assert_eq!(links.len(), 1, "undo link should be persisted for locking");
        assert_eq!(links[0].relation_type, ActionLinkRelationType::UndoOf);
        assert_eq!(links[0].cause_action_id, undo_action.id);
    }

    #[tokio::test]
    async fn retryable_error_on_final_attempt_marks_undo_failed() {
        let (db, _dir, account_id) = setup_account().await;
        let provider_message_id = "msg-14";
        let message_id = seed_message(&db, &account_id, provider_message_id).await;

        let undo_hint = json!({
            "inverse_action": "mark_unread",
            "inverse_parameters": {}
        });
        let original_action =
            seed_completed_action(&db, &account_id, &message_id, "mark_read", undo_hint).await;

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/gmail/v1/users/user@example.com/messages/{provider_message_id}/modify"
            )))
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&server)
            .await;

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "original_action_id": original_action.id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue undo job");
        let mut job = queue.fetch_job(&job_id).await.expect("fetch job");
        job.attempts = job.max_attempts;

        let dispatcher = make_dispatcher(db.clone())
            .with_gmail_api_base(format!("{}/gmail/v1/users", server.uri()));
        let ctx = JobContext::new(queue.clone(), job.clone());

        let result = dispatcher.execute(job, ctx).await;
        assert!(matches!(result, Err(JobError::Retryable { .. })));

        let actions = ActionRepository::new(db.clone())
            .list_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("actions");
        let undo_action = actions
            .into_iter()
            .find(|a| a.action_type == "undo_mark_read")
            .expect("undo action");
        assert_eq!(undo_action.status, ActionStatus::Failed);
        assert!(
            undo_action
                .error_message
                .as_deref()
                .unwrap_or_default()
                .contains("execute inverse action"),
            "error message stored"
        );
    }
}
