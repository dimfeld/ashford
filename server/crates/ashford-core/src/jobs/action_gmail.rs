use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::info;

use crate::accounts::AccountRepository;
use crate::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
use crate::decisions::{Action, ActionRepository, ActionStatus};
use crate::gmail::{GmailClient, GmailClientError, NoopTokenStore};
use crate::labels::{LabelError, LabelRepository, NewLabel};
use crate::llm::decision::ActionType;
use crate::messages::{Message, MessageRepository};
use crate::queue::{JobQueue, QueueError};
use crate::{Job, JobError};

use super::{
    JOB_TYPE_UNSNOOZE_GMAIL, JobDispatcher, map_account_error, map_action_error, map_gmail_error,
};

pub const JOB_TYPE: &str = "action.gmail";

#[derive(Debug, Deserialize)]
struct ActionJobPayload {
    pub account_id: String,
    pub action_id: String,
}

/// Creates a GmailClient for executing Gmail actions.
///
/// This follows the same pattern used in ingest_gmail.rs:
/// 1. Refresh account tokens if needed via AccountRepository
/// 2. Create GmailClient with NoopTokenStore (tokens already refreshed)
/// 3. Configure API base URL from dispatcher settings
///
/// # Arguments
/// * `dispatcher` - The job dispatcher providing HTTP client and config
/// * `account_id` - The account ID to create the client for
///
/// # Returns
/// A configured GmailClient ready for API calls, or a JobError if token refresh fails
pub async fn create_gmail_client(
    dispatcher: &JobDispatcher,
    account_id: &str,
) -> Result<GmailClient<NoopTokenStore>, JobError> {
    let account_repo = AccountRepository::new(dispatcher.db.clone());
    let account = account_repo
        .refresh_tokens_if_needed(
            DEFAULT_ORG_ID,
            DEFAULT_USER_ID,
            account_id,
            &dispatcher.http,
        )
        .await
        .map_err(|err| map_account_error("refresh account tokens", err))?;

    Ok(GmailClient::new(
        dispatcher.http.clone(),
        account.email.clone(),
        account.config.client_id.clone(),
        account.config.client_secret.clone(),
        account.config.oauth.clone(),
        Arc::new(NoopTokenStore),
    )
    .with_api_base(
        dispatcher
            .gmail_api_base
            .clone()
            .unwrap_or_else(|| "https://gmail.googleapis.com/gmail/v1/users".to_string()),
    ))
}

/// Pre-image state captured before executing an action.
///
/// This captures the current Gmail message state so we can build
/// accurate undo hints after the action executes.
#[derive(Debug, Clone)]
pub struct PreImageState {
    /// The current label IDs on the message
    pub labels: Vec<String>,
    /// Whether the message is currently unread (has UNREAD label)
    pub is_unread: bool,
    /// Whether the message is currently starred (has STARRED label)
    pub is_starred: bool,
    /// Whether the message is currently in inbox (has INBOX label)
    pub is_in_inbox: bool,
    /// Whether the message is currently in trash (has TRASH label)
    pub is_in_trash: bool,
}

impl PreImageState {
    /// Creates a PreImageState from a Gmail message's labels.
    pub fn from_labels(labels: &[String]) -> Self {
        Self {
            labels: labels.to_vec(),
            is_unread: labels.iter().any(|l| l == "UNREAD"),
            is_starred: labels.iter().any(|l| l == "STARRED"),
            is_in_inbox: labels.iter().any(|l| l == "INBOX"),
            is_in_trash: labels.iter().any(|l| l == "TRASH"),
        }
    }

    /// Builds an undo hint JSON value with the pre-image state and inverse action info.
    ///
    /// # Arguments
    /// * `action_type` - The action type being executed
    /// * `inverse_action` - The inverse action type for undo
    /// * `inverse_parameters` - Parameters for the inverse action
    pub fn build_undo_hint(
        &self,
        action_type: ActionType,
        inverse_action: ActionType,
        inverse_parameters: Value,
    ) -> Value {
        json!({
            "pre_labels": self.labels,
            "pre_unread": self.is_unread,
            "pre_starred": self.is_starred,
            "pre_in_inbox": self.is_in_inbox,
            "pre_in_trash": self.is_in_trash,
            "action": action_type.as_str(),
            "inverse_action": inverse_action.as_str(),
            "inverse_parameters": inverse_parameters
        })
    }
}

fn parse_snooze_until(parameters: &Value) -> Result<DateTime<Utc>, GmailClientError> {
    let now = Utc::now();
    let has_until = parameters.get("until").is_some();
    let has_amount = parameters.get("amount").is_some();
    let has_units = parameters.get("units").is_some();

    if has_until && (has_amount || has_units) {
        return Err(GmailClientError::InvalidParameter(
            "provide either 'until' or 'amount'/'units', not both".to_string(),
        ));
    }

    if has_until {
        let until_str = parameters["until"].as_str().ok_or_else(|| {
            GmailClientError::InvalidParameter(
                "'until' must be an ISO8601 datetime string".to_string(),
            )
        })?;

        let until = DateTime::parse_from_rfc3339(until_str)
            .map_err(|_| {
                GmailClientError::InvalidParameter(
                    "unable to parse 'until' as RFC3339 datetime".to_string(),
                )
            })?
            .with_timezone(&Utc);

        if until <= now {
            return Err(GmailClientError::InvalidParameter(
                "snooze time must be in the future".to_string(),
            ));
        }

        if until - now > Duration::days(365) {
            return Err(GmailClientError::InvalidParameter(
                "snooze duration exceeds maximum of 1 year".to_string(),
            ));
        }

        return Ok(until);
    }

    if has_amount || has_units {
        let amount = parameters["amount"].as_i64().ok_or_else(|| {
            GmailClientError::InvalidParameter(
                "'amount' must be a positive integer when using duration format".to_string(),
            )
        })?;

        if amount <= 0 {
            return Err(GmailClientError::InvalidParameter(
                "'amount' must be greater than zero".to_string(),
            ));
        }

        let units = parameters["units"].as_str().ok_or_else(|| {
            GmailClientError::InvalidParameter(
                "'units' must be one of minutes|hours|days".to_string(),
            )
        })?;

        let duration = match units {
            "minutes" => Duration::minutes(amount),
            "hours" => Duration::hours(amount),
            "days" => Duration::days(amount),
            other => {
                return Err(GmailClientError::InvalidParameter(format!(
                    "unsupported units '{other}', use minutes|hours|days"
                )));
            }
        };

        if duration > Duration::days(365) {
            return Err(GmailClientError::InvalidParameter(
                "snooze duration exceeds maximum of 1 year".to_string(),
            ));
        }

        return Ok(now + duration);
    }

    Err(GmailClientError::InvalidParameter(
        "snooze requires either 'until' or 'amount' and 'units'".to_string(),
    ))
}

fn build_new_label_from_gmail(label: &crate::gmail::types::Label, account_id: &str) -> NewLabel {
    NewLabel {
        org_id: DEFAULT_ORG_ID,
        user_id: DEFAULT_USER_ID,
        account_id: account_id.to_string(),
        provider_label_id: label.id.clone(),
        name: label.name.clone(),
        label_type: label
            .label_type
            .clone()
            .unwrap_or_else(|| "user".to_string()),
        description: None,
        available_to_classifier: true,
        message_list_visibility: label.message_list_visibility.clone(),
        label_list_visibility: label.label_list_visibility.clone(),
        background_color: label
            .color
            .as_ref()
            .and_then(|c| c.background_color.clone()),
        text_color: label.color.as_ref().and_then(|c| c.text_color.clone()),
    }
}

async fn ensure_snooze_label(
    dispatcher: &JobDispatcher,
    gmail_client: &GmailClient<NoopTokenStore>,
    account_id: &str,
    snooze_label_name: &str,
) -> Result<String, JobError> {
    let repo = LabelRepository::new(dispatcher.db.clone());

    let existing = match repo
        .get_by_name(
            DEFAULT_ORG_ID,
            DEFAULT_USER_ID,
            account_id,
            snooze_label_name,
        )
        .await
    {
        Ok(label) => Some(label),
        Err(LabelError::NotFound(_)) => None,
        Err(err) => {
            return Err(JobError::retryable(format!(
                "failed to lookup snooze label '{snooze_label_name}': {err}"
            )));
        }
    };

    let labels = gmail_client
        .list_labels()
        .await
        .map_err(|err| map_gmail_error("list labels", err))?;

    if let Some(cached) = existing.as_ref() {
        if let Some(label) = labels
            .labels
            .iter()
            .find(|l| l.id == cached.provider_label_id)
        {
            repo.upsert(build_new_label_from_gmail(label, account_id))
                .await
                .map_err(|err| JobError::retryable(format!("store snooze label: {err}")))?;

            return Ok(label.id.clone());
        }
    }

    if let Some(label) = labels
        .labels
        .iter()
        .find(|l| l.name.eq_ignore_ascii_case(snooze_label_name))
    {
        if let Some(cached) = existing.as_ref() {
            if cached.provider_label_id != label.id {
                repo.delete_by_provider_id(
                    DEFAULT_ORG_ID,
                    DEFAULT_USER_ID,
                    account_id,
                    &cached.provider_label_id,
                )
                .await
                .map_err(|err| JobError::retryable(format!("remove stale snooze label: {err}")))?;
            }
        }

        repo.upsert(build_new_label_from_gmail(label, account_id))
            .await
            .map_err(|err| JobError::retryable(format!("store snooze label: {err}")))?;

        return Ok(label.id.clone());
    }

    let created_label = gmail_client
        .create_label(snooze_label_name)
        .await
        .map_err(|err| map_gmail_error("create snooze label", err))?;

    if let Some(cached) = existing.as_ref() {
        if cached.provider_label_id != created_label.id {
            repo.delete_by_provider_id(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                account_id,
                &cached.provider_label_id,
            )
            .await
            .map_err(|err| JobError::retryable(format!("remove stale snooze label: {err}")))?;
        }
    }

    repo.upsert(build_new_label_from_gmail(&created_label, account_id))
        .await
        .map_err(|err| JobError::retryable(format!("store snooze label: {err}")))?;

    Ok(created_label.id)
}

/// Captures the pre-image state of a message before executing an action.
///
/// This fetches the current message state from Gmail to enable accurate
/// undo hint generation. The pre-image includes label state, read/unread
/// state, and other relevant attributes.
///
/// # Arguments
/// * `gmail_client` - The Gmail client to use for API calls
/// * `provider_message_id` - The Gmail message ID
///
/// # Returns
/// The PreImageState containing current message attributes
pub async fn capture_pre_image<S: crate::gmail::oauth::TokenStore>(
    gmail_client: &GmailClient<S>,
    provider_message_id: &str,
) -> Result<PreImageState, GmailClientError> {
    let gmail_message = gmail_client
        .get_message_minimal(provider_message_id)
        .await?;
    Ok(PreImageState::from_labels(&gmail_message.label_ids))
}

/// Fetches the provider_message_id for an internal message ID.
///
/// Actions store the internal message UUID, but Gmail API requires the
/// provider's message ID. This helper looks up the mapping.
///
/// # Arguments
/// * `dispatcher` - The job dispatcher
/// * `message_id` - The internal message UUID
///
/// # Returns
/// A tuple of (provider_message_id, account_id) for use with Gmail API
pub async fn get_provider_message_id(
    dispatcher: &JobDispatcher,
    message_id: &str,
) -> Result<Message, JobError> {
    let msg_repo = MessageRepository::new(dispatcher.db.clone());
    msg_repo
        .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, message_id)
        .await
        .map_err(|err| match err {
            crate::messages::MessageError::NotFound(_) => {
                JobError::Fatal(format!("message not found: {}", message_id))
            }
            _ => JobError::retryable(format!("failed to load message: {err}")),
        })
}

/// Result of executing an action, containing the undo hint for reversibility.
#[derive(Debug)]
struct ActionExecutionResult {
    /// The undo hint JSON value to store with the action
    undo_hint: Value,
}

/// Execute the archive action: removes the INBOX label from the message.
async fn execute_archive(
    gmail_client: &GmailClient<NoopTokenStore>,
    provider_message_id: &str,
    pre_image: &PreImageState,
) -> Result<ActionExecutionResult, GmailClientError> {
    gmail_client
        .modify_message(provider_message_id, None, Some(vec!["INBOX".to_string()]))
        .await?;

    let undo_hint = pre_image.build_undo_hint(
        ActionType::Archive,
        ActionType::ApplyLabel,
        json!({"label": "INBOX"}),
    );

    Ok(ActionExecutionResult { undo_hint })
}

/// Execute the apply_label action: adds a label to the message.
async fn execute_apply_label(
    gmail_client: &GmailClient<NoopTokenStore>,
    provider_message_id: &str,
    pre_image: &PreImageState,
    parameters: &Value,
) -> Result<ActionExecutionResult, GmailClientError> {
    let label_id = parameters["label"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            GmailClientError::InvalidParameter(
                "apply_label action requires a non-empty 'label' parameter".to_string(),
            )
        })?
        .to_string();

    gmail_client
        .modify_message(provider_message_id, Some(vec![label_id.clone()]), None)
        .await?;

    let undo_hint = pre_image.build_undo_hint(
        ActionType::ApplyLabel,
        ActionType::RemoveLabel,
        json!({"label": label_id}),
    );

    Ok(ActionExecutionResult { undo_hint })
}

/// Execute the remove_label action: removes a label from the message.
async fn execute_remove_label(
    gmail_client: &GmailClient<NoopTokenStore>,
    provider_message_id: &str,
    pre_image: &PreImageState,
    parameters: &Value,
) -> Result<ActionExecutionResult, GmailClientError> {
    let label_id = parameters["label"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            GmailClientError::InvalidParameter(
                "remove_label action requires a non-empty 'label' parameter".to_string(),
            )
        })?
        .to_string();

    gmail_client
        .modify_message(provider_message_id, None, Some(vec![label_id.clone()]))
        .await?;

    let undo_hint = pre_image.build_undo_hint(
        ActionType::RemoveLabel,
        ActionType::ApplyLabel,
        json!({"label": label_id}),
    );

    Ok(ActionExecutionResult { undo_hint })
}

/// Execute the mark_read action: removes the UNREAD label from the message.
async fn execute_mark_read(
    gmail_client: &GmailClient<NoopTokenStore>,
    provider_message_id: &str,
    pre_image: &PreImageState,
) -> Result<ActionExecutionResult, GmailClientError> {
    gmail_client
        .modify_message(provider_message_id, None, Some(vec!["UNREAD".to_string()]))
        .await?;

    let undo_hint =
        pre_image.build_undo_hint(ActionType::MarkRead, ActionType::MarkUnread, json!({}));

    Ok(ActionExecutionResult { undo_hint })
}

/// Execute the mark_unread action: adds the UNREAD label to the message.
async fn execute_mark_unread(
    gmail_client: &GmailClient<NoopTokenStore>,
    provider_message_id: &str,
    pre_image: &PreImageState,
) -> Result<ActionExecutionResult, GmailClientError> {
    gmail_client
        .modify_message(provider_message_id, Some(vec!["UNREAD".to_string()]), None)
        .await?;

    let undo_hint =
        pre_image.build_undo_hint(ActionType::MarkUnread, ActionType::MarkRead, json!({}));

    Ok(ActionExecutionResult { undo_hint })
}

/// Execute the star action: adds the STARRED label to the message.
async fn execute_star(
    gmail_client: &GmailClient<NoopTokenStore>,
    provider_message_id: &str,
    pre_image: &PreImageState,
) -> Result<ActionExecutionResult, GmailClientError> {
    gmail_client
        .modify_message(provider_message_id, Some(vec!["STARRED".to_string()]), None)
        .await?;

    let undo_hint = pre_image.build_undo_hint(ActionType::Star, ActionType::Unstar, json!({}));

    Ok(ActionExecutionResult { undo_hint })
}

/// Execute the unstar action: removes the STARRED label from the message.
async fn execute_unstar(
    gmail_client: &GmailClient<NoopTokenStore>,
    provider_message_id: &str,
    pre_image: &PreImageState,
) -> Result<ActionExecutionResult, GmailClientError> {
    gmail_client
        .modify_message(provider_message_id, None, Some(vec!["STARRED".to_string()]))
        .await?;

    let undo_hint = pre_image.build_undo_hint(ActionType::Unstar, ActionType::Star, json!({}));

    Ok(ActionExecutionResult { undo_hint })
}

/// Execute the trash action: moves the message to trash.
async fn execute_trash(
    gmail_client: &GmailClient<NoopTokenStore>,
    provider_message_id: &str,
    pre_image: &PreImageState,
) -> Result<ActionExecutionResult, GmailClientError> {
    gmail_client.trash_message(provider_message_id).await?;

    let undo_hint = pre_image.build_undo_hint(ActionType::Trash, ActionType::Restore, json!({}));

    Ok(ActionExecutionResult { undo_hint })
}

/// Execute the delete action: permanently deletes the message.
/// WARNING: This action cannot be undone.
async fn execute_delete(
    gmail_client: &GmailClient<NoopTokenStore>,
    provider_message_id: &str,
) -> Result<ActionExecutionResult, GmailClientError> {
    gmail_client.delete_message(provider_message_id).await?;

    // Delete is irreversible - no pre-image needed, undo is not possible
    let undo_hint = json!({
        "action": "delete",
        "inverse_action": "none",
        "inverse_parameters": {"note": "cannot undo delete - message permanently deleted"},
        "irreversible": true
    });

    Ok(ActionExecutionResult { undo_hint })
}

/// Execute the restore action: removes a message from trash.
async fn execute_restore(
    gmail_client: &GmailClient<NoopTokenStore>,
    provider_message_id: &str,
    pre_image: &PreImageState,
) -> Result<ActionExecutionResult, GmailClientError> {
    gmail_client.untrash_message(provider_message_id).await?;

    let undo_hint = pre_image.build_undo_hint(ActionType::Restore, ActionType::Trash, json!({}));

    Ok(ActionExecutionResult { undo_hint })
}

async fn execute_snooze(
    dispatcher: &JobDispatcher,
    gmail_client: &GmailClient<NoopTokenStore>,
    message: &Message,
    action: &Action,
) -> Result<ActionExecutionResult, JobError> {
    let snooze_until = parse_snooze_until(&action.parameters_json)
        .map_err(|err| map_gmail_error("parse snooze parameters", err))?;

    let pre_image = capture_pre_image(gmail_client, &message.provider_message_id)
        .await
        .map_err(|err| map_gmail_error("capture pre-image", err))?;

    let snooze_label_name = dispatcher.gmail_config.snooze_label.clone();
    let snooze_label_id = ensure_snooze_label(
        dispatcher,
        gmail_client,
        &message.account_id,
        &snooze_label_name,
    )
    .await?;

    gmail_client
        .modify_message(
            &message.provider_message_id,
            Some(vec![snooze_label_id.clone()]),
            Some(vec!["INBOX".to_string()]),
        )
        .await
        .map_err(|err| map_gmail_error("apply snooze labels", err))?;

    let queue = JobQueue::new(dispatcher.db.clone());
    let payload = json!({
        "account_id": message.account_id,
        "message_id": message.id,
        "action_id": action.id,
        "snooze_label_id": snooze_label_id.clone(),
    });
    let idempotency_key = format!("unsnooze.gmail:{}:{}", message.account_id, action.id);
    let unsnooze_job_id = match queue
        .enqueue_scheduled(
            JOB_TYPE_UNSNOOZE_GMAIL,
            payload,
            Some(idempotency_key),
            0,
            snooze_until,
        )
        .await
    {
        Ok(id) => id,
        Err(QueueError::DuplicateIdempotency {
            existing_job_id: Some(existing),
            ..
        }) => existing,
        Err(err) => return Err(JobError::retryable(format!("schedule unsnooze job: {err}"))),
    };

    let inverse_parameters = json!({
        "add_labels": ["INBOX"],
        "remove_labels": [snooze_label_id.clone()],
        "cancel_unsnooze_job_id": unsnooze_job_id.clone(),
        "note": "Undo snooze by returning to inbox and removing snooze label",
    });

    let mut undo_hint =
        pre_image.build_undo_hint(ActionType::Snooze, ActionType::None, inverse_parameters);
    undo_hint["snooze_until"] = json!(snooze_until);
    undo_hint["snooze_label"] = json!(snooze_label_id);
    undo_hint["unsnooze_job_id"] = json!(unsnooze_job_id);

    Ok(ActionExecutionResult { undo_hint })
}

/// Execute the Gmail action mutation based on action type.
///
/// This function dispatches to the appropriate handler based on the action_type
/// and returns an ActionExecutionResult with the undo hint.
async fn execute_action(
    gmail_client: &GmailClient<NoopTokenStore>,
    provider_message_id: &str,
    action: &Action,
) -> Result<ActionExecutionResult, GmailClientError> {
    // For delete, we don't need pre-image since it's irreversible
    if action.action_type == "delete" {
        return execute_delete(gmail_client, provider_message_id).await;
    }

    // Capture pre-image for all other actions
    let pre_image = capture_pre_image(gmail_client, provider_message_id).await?;

    match action.action_type.as_str() {
        "archive" => execute_archive(gmail_client, provider_message_id, &pre_image).await,
        "apply_label" => {
            execute_apply_label(
                gmail_client,
                provider_message_id,
                &pre_image,
                &action.parameters_json,
            )
            .await
        }
        "remove_label" => {
            execute_remove_label(
                gmail_client,
                provider_message_id,
                &pre_image,
                &action.parameters_json,
            )
            .await
        }
        "mark_read" => execute_mark_read(gmail_client, provider_message_id, &pre_image).await,
        "mark_unread" => execute_mark_unread(gmail_client, provider_message_id, &pre_image).await,
        "star" => execute_star(gmail_client, provider_message_id, &pre_image).await,
        "unstar" => execute_unstar(gmail_client, provider_message_id, &pre_image).await,
        "trash" => execute_trash(gmail_client, provider_message_id, &pre_image).await,
        "restore" => execute_restore(gmail_client, provider_message_id, &pre_image).await,
        other => {
            // Unsupported action types should fail, not silently succeed
            Err(GmailClientError::UnsupportedAction(other.to_string()))
        }
    }
}

/// Execute a Gmail action.
///
/// This handler:
/// 1. Loads the action from the database
/// 2. Validates the action belongs to the specified account
/// 3. Marks the action as executing
/// 4. Fetches the Gmail message and captures pre-image state
/// 5. Executes the appropriate Gmail API mutation
/// 6. Stores the undo hint and marks the action as completed
///
/// On failure, the action is marked as failed with an error message.
pub async fn handle_action_gmail(dispatcher: &JobDispatcher, job: Job) -> Result<(), JobError> {
    let payload: ActionJobPayload = serde_json::from_value(job.payload.clone())
        .map_err(|err| JobError::Fatal(format!("invalid action.gmail payload: {err}")))?;

    let repo = ActionRepository::new(dispatcher.db.clone());
    let action = repo
        .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &payload.action_id)
        .await
        .map_err(|err| map_action_error("load action", err))?;

    if action.account_id != payload.account_id {
        return Err(JobError::Fatal(format!(
            "action {} does not belong to account {}",
            payload.action_id, payload.account_id
        )));
    }

    // Check if already processed or in a terminal state
    match action.status {
        ActionStatus::Completed
        | ActionStatus::Failed
        | ActionStatus::Canceled
        | ActionStatus::Rejected => {
            info!(
                account_id = %payload.account_id,
                action_id = %payload.action_id,
                status = ?action.status,
                "action already in terminal state, skipping"
            );
            return Ok(());
        }
        ActionStatus::ApprovedPending => {
            info!(
                account_id = %payload.account_id,
                action_id = %payload.action_id,
                "action awaiting approval, skipping"
            );
            return Ok(());
        }
        ActionStatus::Queued => {
            // Mark as executing before we start
            repo.mark_executing(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action.id)
                .await
                .map_err(|err| {
                    JobError::retryable(format!("failed to mark action executing: {err}"))
                })?;
        }
        ActionStatus::Executing => {
            // Already executing - this is a retry, continue with execution
        }
    }

    // Get the provider message ID from our internal message record
    let message = get_provider_message_id(dispatcher, &action.message_id).await?;
    let provider_message_id = &message.provider_message_id;

    // Create Gmail client
    let gmail_client = create_gmail_client(dispatcher, &payload.account_id).await?;

    // Execute the action and get the result
    let execution_result: Result<ActionExecutionResult, JobError> =
        if action.action_type == "snooze" {
            execute_snooze(dispatcher, &gmail_client, &message, &action).await
        } else {
            execute_action(&gmail_client, provider_message_id, &action)
                .await
                .map_err(|err| map_gmail_error("execute gmail action", err))
        };

    match execution_result {
        Ok(execution_result) => {
            // Mark completed with undo hint
            repo.mark_completed_with_undo_hint(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                &action.id,
                execution_result.undo_hint,
            )
            .await
            .map_err(|err| {
                JobError::retryable(format!("failed to mark action completed: {err}"))
            })?;

            info!(
                account_id = %payload.account_id,
                action_id = %payload.action_id,
                action_type = %action.action_type,
                "executed gmail action successfully"
            );

            Ok(())
        }
        Err(job_error) => {
            let attempts_exhausted = job.attempts >= job.max_attempts;

            // Mark the action as failed for fatal errors or when no retries remain; otherwise
            // keep the action in Executing so the worker can retry.
            if !job_error.is_retryable() || attempts_exhausted {
                let error_message = match &job_error {
                    JobError::Fatal(msg) => msg.clone(),
                    JobError::Retryable { message, .. } => message.clone(),
                };

                let _ = repo
                    .mark_failed(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action.id, error_message)
                    .await;
            }

            Err(job_error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ===== PreImageState unit tests =====

    #[test]
    fn pre_image_state_from_labels_detects_unread() {
        let labels = vec!["INBOX".to_string(), "UNREAD".to_string()];
        let state = PreImageState::from_labels(&labels);

        assert!(state.is_unread);
        assert!(state.is_in_inbox);
        assert!(!state.is_starred);
        assert!(!state.is_in_trash);
        assert_eq!(state.labels, labels);
    }

    #[test]
    fn pre_image_state_from_labels_detects_starred() {
        let labels = vec!["INBOX".to_string(), "STARRED".to_string()];
        let state = PreImageState::from_labels(&labels);

        assert!(!state.is_unread);
        assert!(state.is_in_inbox);
        assert!(state.is_starred);
        assert!(!state.is_in_trash);
    }

    #[test]
    fn pre_image_state_from_labels_detects_trash() {
        let labels = vec!["TRASH".to_string()];
        let state = PreImageState::from_labels(&labels);

        assert!(!state.is_unread);
        assert!(!state.is_in_inbox);
        assert!(!state.is_starred);
        assert!(state.is_in_trash);
    }

    #[test]
    fn pre_image_state_from_labels_handles_empty() {
        let labels: Vec<String> = vec![];
        let state = PreImageState::from_labels(&labels);

        assert!(!state.is_unread);
        assert!(!state.is_in_inbox);
        assert!(!state.is_starred);
        assert!(!state.is_in_trash);
        assert!(state.labels.is_empty());
    }

    #[test]
    fn pre_image_state_from_labels_handles_custom_labels() {
        let labels = vec![
            "INBOX".to_string(),
            "UNREAD".to_string(),
            "Label_123".to_string(),
            "Label_456".to_string(),
        ];
        let state = PreImageState::from_labels(&labels);

        assert!(state.is_unread);
        assert!(state.is_in_inbox);
        assert_eq!(state.labels.len(), 4);
        assert!(state.labels.contains(&"Label_123".to_string()));
    }

    #[test]
    fn pre_image_state_build_undo_hint_for_archive() {
        let labels = vec!["INBOX".to_string(), "UNREAD".to_string()];
        let state = PreImageState::from_labels(&labels);

        let undo_hint = state.build_undo_hint(
            ActionType::Archive,
            ActionType::ApplyLabel,
            json!({"label": "INBOX"}),
        );

        assert_eq!(undo_hint["action"], "archive");
        assert_eq!(undo_hint["inverse_action"], "apply_label");
        assert_eq!(undo_hint["inverse_parameters"]["label"], "INBOX");
        assert_eq!(undo_hint["pre_labels"], json!(["INBOX", "UNREAD"]));
        assert_eq!(undo_hint["pre_unread"], true);
        assert_eq!(undo_hint["pre_in_inbox"], true);
        assert_eq!(undo_hint["pre_starred"], false);
        assert_eq!(undo_hint["pre_in_trash"], false);
    }

    #[test]
    fn pre_image_state_build_undo_hint_for_mark_read() {
        let labels = vec!["INBOX".to_string(), "UNREAD".to_string()];
        let state = PreImageState::from_labels(&labels);

        let undo_hint =
            state.build_undo_hint(ActionType::MarkRead, ActionType::MarkUnread, json!({}));

        assert_eq!(undo_hint["action"], "mark_read");
        assert_eq!(undo_hint["inverse_action"], "mark_unread");
        assert_eq!(undo_hint["pre_unread"], true);
    }

    #[test]
    fn pre_image_state_build_undo_hint_for_star() {
        let labels = vec!["INBOX".to_string()];
        let state = PreImageState::from_labels(&labels);

        let undo_hint = state.build_undo_hint(ActionType::Star, ActionType::Unstar, json!({}));

        assert_eq!(undo_hint["action"], "star");
        assert_eq!(undo_hint["inverse_action"], "unstar");
        assert_eq!(undo_hint["pre_starred"], false);
    }

    #[test]
    fn pre_image_state_build_undo_hint_for_trash() {
        let labels = vec!["INBOX".to_string(), "IMPORTANT".to_string()];
        let state = PreImageState::from_labels(&labels);

        let undo_hint = state.build_undo_hint(ActionType::Trash, ActionType::Restore, json!({}));

        assert_eq!(undo_hint["action"], "trash");
        assert_eq!(undo_hint["inverse_action"], "restore");
        assert_eq!(undo_hint["pre_labels"], json!(["INBOX", "IMPORTANT"]));
        assert_eq!(undo_hint["pre_in_trash"], false);
    }

    #[test]
    fn pre_image_state_build_undo_hint_for_delete() {
        let labels = vec!["TRASH".to_string()];
        let state = PreImageState::from_labels(&labels);

        let undo_hint = state.build_undo_hint(
            ActionType::Delete,
            ActionType::None,
            json!({"note": "cannot undo delete"}),
        );

        assert_eq!(undo_hint["action"], "delete");
        assert_eq!(undo_hint["inverse_action"], "none");
        assert_eq!(
            undo_hint["inverse_parameters"]["note"],
            "cannot undo delete"
        );
        assert_eq!(undo_hint["pre_in_trash"], true);
    }

    // ===== Additional PreImageState edge case tests =====

    #[test]
    fn pre_image_state_from_labels_all_flags_true() {
        // Test a message that has all system labels
        let labels = vec![
            "INBOX".to_string(),
            "UNREAD".to_string(),
            "STARRED".to_string(),
            "TRASH".to_string(), // unlikely in practice but let's test
        ];
        let state = PreImageState::from_labels(&labels);

        assert!(state.is_unread);
        assert!(state.is_in_inbox);
        assert!(state.is_starred);
        assert!(state.is_in_trash);
        assert_eq!(state.labels.len(), 4);
    }

    #[test]
    fn pre_image_state_from_labels_case_sensitive() {
        // Gmail labels are case-sensitive, lowercase should not match
        let labels = vec![
            "inbox".to_string(),
            "unread".to_string(),
            "starred".to_string(),
            "trash".to_string(),
        ];
        let state = PreImageState::from_labels(&labels);

        assert!(!state.is_unread);
        assert!(!state.is_in_inbox);
        assert!(!state.is_starred);
        assert!(!state.is_in_trash);
        // Labels are still stored
        assert_eq!(state.labels.len(), 4);
    }

    #[test]
    fn pre_image_state_from_labels_with_category_labels() {
        // Gmail uses CATEGORY_* labels
        let labels = vec![
            "INBOX".to_string(),
            "CATEGORY_PROMOTIONS".to_string(),
            "CATEGORY_SOCIAL".to_string(),
        ];
        let state = PreImageState::from_labels(&labels);

        assert!(state.is_in_inbox);
        assert!(!state.is_unread);
        assert!(!state.is_starred);
        assert!(!state.is_in_trash);
        assert_eq!(state.labels.len(), 3);
    }

    #[test]
    fn pre_image_state_build_undo_hint_with_remove_label() {
        let labels = vec!["INBOX".to_string(), "Label_Important".to_string()];
        let state = PreImageState::from_labels(&labels);

        let undo_hint = state.build_undo_hint(
            ActionType::RemoveLabel,
            ActionType::ApplyLabel,
            json!({"label": "Label_Important"}),
        );

        assert_eq!(undo_hint["action"], "remove_label");
        assert_eq!(undo_hint["inverse_action"], "apply_label");
        assert_eq!(undo_hint["inverse_parameters"]["label"], "Label_Important");
        assert!(
            undo_hint["pre_labels"]
                .as_array()
                .unwrap()
                .contains(&json!("Label_Important"))
        );
    }

    #[test]
    fn pre_image_state_build_undo_hint_with_apply_label() {
        let labels = vec!["INBOX".to_string()];
        let state = PreImageState::from_labels(&labels);

        let undo_hint = state.build_undo_hint(
            ActionType::ApplyLabel,
            ActionType::RemoveLabel,
            json!({"label": "NewLabel"}),
        );

        assert_eq!(undo_hint["action"], "apply_label");
        assert_eq!(undo_hint["inverse_action"], "remove_label");
        assert_eq!(undo_hint["inverse_parameters"]["label"], "NewLabel");
    }

    #[test]
    fn pre_image_state_build_undo_hint_for_restore() {
        let labels = vec!["TRASH".to_string()];
        let state = PreImageState::from_labels(&labels);

        let undo_hint = state.build_undo_hint(ActionType::Restore, ActionType::Trash, json!({}));

        assert_eq!(undo_hint["action"], "restore");
        assert_eq!(undo_hint["inverse_action"], "trash");
        assert_eq!(undo_hint["pre_in_trash"], true);
    }

    #[test]
    fn pre_image_state_build_undo_hint_for_unstar() {
        let labels = vec!["INBOX".to_string(), "STARRED".to_string()];
        let state = PreImageState::from_labels(&labels);

        let undo_hint = state.build_undo_hint(ActionType::Unstar, ActionType::Star, json!({}));

        assert_eq!(undo_hint["action"], "unstar");
        assert_eq!(undo_hint["inverse_action"], "star");
        assert_eq!(undo_hint["pre_starred"], true);
    }

    #[test]
    fn pre_image_state_build_undo_hint_for_mark_unread() {
        // Message is already read (no UNREAD label)
        let labels = vec!["INBOX".to_string()];
        let state = PreImageState::from_labels(&labels);

        let undo_hint =
            state.build_undo_hint(ActionType::MarkUnread, ActionType::MarkRead, json!({}));

        assert_eq!(undo_hint["action"], "mark_unread");
        assert_eq!(undo_hint["inverse_action"], "mark_read");
        assert_eq!(undo_hint["pre_unread"], false);
    }

    // ===== Snooze parameter parsing =====

    #[test]
    fn parse_snooze_with_absolute_until() {
        let until = (Utc::now() + chrono::Duration::minutes(10))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let parsed = parse_snooze_until(&json!({"until": until})).expect("parse");
        let diff = (parsed - Utc::now()).num_minutes();
        assert!(diff >= 9 && diff <= 10);
    }

    #[test]
    fn parse_snooze_with_duration() {
        let parsed =
            parse_snooze_until(&json!({"amount": 2, "units": "hours"})).expect("parse duration");
        let diff = (parsed - Utc::now()).num_minutes();
        assert!(diff >= 119 && diff <= 121);
    }

    #[test]
    fn parse_snooze_rejects_past_until() {
        let until = (Utc::now() - chrono::Duration::minutes(1))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let err = parse_snooze_until(&json!({"until": until})).unwrap_err();
        match err {
            GmailClientError::InvalidParameter(msg) => {
                assert!(msg.contains("future"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn parse_snooze_rejects_missing_fields() {
        let err = parse_snooze_until(&json!({})).unwrap_err();
        assert!(matches!(err, GmailClientError::InvalidParameter(_)));
    }

    #[test]
    fn parse_snooze_rejects_overlong_duration() {
        let err = parse_snooze_until(&json!({"amount": 400, "units": "days"})).unwrap_err();
        match err {
            GmailClientError::InvalidParameter(msg) => {
                assert!(msg.contains("1 year"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn parse_snooze_rejects_non_positive_amount() {
        let err = parse_snooze_until(&json!({"amount": 0, "units": "hours"})).unwrap_err();
        match err {
            GmailClientError::InvalidParameter(msg) => {
                assert!(msg.contains("greater than"), "unexpected message: {msg}");
            }
            other => panic!("unexpected error: {other:?}"),
        }

        let err = parse_snooze_until(&json!({"amount": -3, "units": "hours"})).unwrap_err();
        match err {
            GmailClientError::InvalidParameter(msg) => {
                assert!(msg.contains("greater than"), "unexpected message: {msg}");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn parse_snooze_rejects_unknown_units() {
        let err = parse_snooze_until(&json!({"amount": 1, "units": "weeks"})).unwrap_err();
        match err {
            GmailClientError::InvalidParameter(msg) => {
                assert!(
                    msg.contains("unsupported units"),
                    "unexpected message: {msg}"
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn parse_snooze_rejects_mixed_formats() {
        let until = (Utc::now() + chrono::Duration::hours(1))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let err = parse_snooze_until(&json!({"until": until, "amount": 1, "units": "hours"}))
            .unwrap_err();

        match err {
            GmailClientError::InvalidParameter(msg) => {
                assert!(msg.contains("either 'until'"), "unexpected message: {msg}");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    // ===== Integration tests for capture_pre_image =====

    mod integration_tests {
        use super::*;
        use crate::accounts::{Account, AccountConfig, AccountRepository, PubsubConfig};
        use crate::config::PolicyConfig;
        use crate::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
        use crate::gmail::{NoopTokenStore, OAuthTokens};
        use crate::llm::MockLLMClient;
        use crate::messages::{Mailbox, MessageRepository, NewMessage};
        use crate::migrations::run_migrations;
        use crate::threads::ThreadRepository;
        use chrono::Utc;
        use tempfile::TempDir;
        use wiremock::matchers::{method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        async fn setup_db() -> (crate::Database, TempDir) {
            let dir = TempDir::new().expect("temp dir");
            let db_name = format!("db_{}.sqlite", uuid::Uuid::new_v4());
            let db_path = dir.path().join(db_name);
            let db = crate::Database::new(&db_path).await.expect("create db");
            run_migrations(&db).await.expect("migrations");
            (db, dir)
        }

        async fn setup_account(db: &crate::Database) -> (Account, String) {
            let repo = AccountRepository::new(db.clone());
            let config = AccountConfig {
                client_id: "client".into(),
                client_secret: "secret".into(),
                oauth: OAuthTokens {
                    access_token: "access_token".into(),
                    refresh_token: "refresh_token".into(),
                    expires_at: Utc::now() + chrono::Duration::hours(1),
                },
                pubsub: PubsubConfig::default(),
            };
            let account = repo
                .create(
                    DEFAULT_ORG_ID,
                    DEFAULT_USER_ID,
                    "user@example.com",
                    Some("User".into()),
                    config,
                )
                .await
                .expect("create account");

            (account.clone(), account.id)
        }

        async fn setup_message(
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
                    "thread-123",
                    Some("Test Subject".to_string()),
                    Some("Test snippet".to_string()),
                    Some(Utc::now()),
                    json!({}),
                )
                .await
                .expect("create thread");

            let msg_repo = MessageRepository::new(db.clone());
            let msg = msg_repo
                .upsert(NewMessage {
                    org_id: DEFAULT_ORG_ID,
                    user_id: DEFAULT_USER_ID,
                    account_id: account_id.to_string(),
                    thread_id: thread.id.clone(),
                    provider_message_id: provider_message_id.to_string(),
                    from_email: Some("sender@example.com".to_string()),
                    from_name: Some("Sender".to_string()),
                    to: vec![Mailbox {
                        email: "user@example.com".to_string(),
                        name: Some("User".to_string()),
                    }],
                    cc: vec![],
                    bcc: vec![],
                    subject: Some("Test Subject".to_string()),
                    received_at: Some(Utc::now()),
                    internal_date: Some(Utc::now()),
                    labels: vec!["INBOX".to_string()],
                    headers: vec![],
                    body_plain: Some("Test body".to_string()),
                    body_html: None,
                    snippet: Some("Test snippet".to_string()),
                    raw_json: json!({}),
                })
                .await
                .expect("create message");

            msg.id
        }

        #[tokio::test]
        async fn capture_pre_image_returns_labels() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .and(query_param("format", "minimal"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "id": "msg-123",
                    "threadId": "thread-123",
                    "labelIds": ["INBOX", "UNREAD", "IMPORTANT"],
                    "snippet": "Test",
                    "internalDate": "1730000000000"
                })))
                .expect(1)
                .mount(&server)
                .await;

            let tokens = OAuthTokens {
                access_token: "test_access".into(),
                refresh_token: "test_refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            };

            let client = crate::gmail::GmailClient::new(
                reqwest::Client::new(),
                "user@example.com".to_string(),
                "client_id".to_string(),
                "client_secret".to_string(),
                tokens,
                Arc::new(NoopTokenStore),
            )
            .with_api_base(api_base);

            let pre_image = capture_pre_image(&client, "msg-123")
                .await
                .expect("capture");

            assert!(pre_image.is_unread);
            assert!(pre_image.is_in_inbox);
            assert!(!pre_image.is_starred);
            assert!(!pre_image.is_in_trash);
            assert_eq!(pre_image.labels.len(), 3);
            assert!(pre_image.labels.contains(&"IMPORTANT".to_string()));
        }

        #[tokio::test]
        async fn capture_pre_image_handles_not_found() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            Mock::given(method("GET"))
                .and(path(
                    "/gmail/v1/users/user@example.com/messages/nonexistent",
                ))
                .and(query_param("format", "minimal"))
                .respond_with(ResponseTemplate::new(404).set_body_json(json!({
                    "error": {
                        "code": 404,
                        "message": "Not found"
                    }
                })))
                .expect(1)
                .mount(&server)
                .await;

            let tokens = OAuthTokens {
                access_token: "test_access".into(),
                refresh_token: "test_refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            };

            let client = crate::gmail::GmailClient::new(
                reqwest::Client::new(),
                "user@example.com".to_string(),
                "client_id".to_string(),
                "client_secret".to_string(),
                tokens,
                Arc::new(NoopTokenStore),
            )
            .with_api_base(api_base);

            let result = capture_pre_image(&client, "nonexistent").await;
            assert!(result.is_err());
            match result.err().unwrap() {
                crate::gmail::GmailClientError::Http(e) => {
                    assert_eq!(e.status().unwrap(), reqwest::StatusCode::NOT_FOUND);
                }
                other => panic!("Expected Http error, got {:?}", other),
            }
        }

        #[tokio::test]
        async fn get_provider_message_id_returns_message() {
            let (db, _dir) = setup_db().await;
            let (_, account_id) = setup_account(&db).await;
            let internal_message_id = setup_message(&db, &account_id, "provider-msg-456").await;

            let dispatcher = JobDispatcher::new(
                db.clone(),
                reqwest::Client::new(),
                Arc::new(MockLLMClient::new()),
                PolicyConfig::default(),
            );

            let message = get_provider_message_id(&dispatcher, &internal_message_id)
                .await
                .expect("get message");

            assert_eq!(message.id, internal_message_id);
            assert_eq!(message.provider_message_id, "provider-msg-456");
            assert_eq!(message.account_id, account_id);
        }

        #[tokio::test]
        async fn get_provider_message_id_returns_fatal_for_not_found() {
            let (db, _dir) = setup_db().await;

            let dispatcher = JobDispatcher::new(
                db.clone(),
                reqwest::Client::new(),
                Arc::new(MockLLMClient::new()),
                PolicyConfig::default(),
            );

            let result = get_provider_message_id(&dispatcher, "nonexistent-id").await;

            match result {
                Err(JobError::Fatal(msg)) => {
                    assert!(msg.contains("message not found"));
                    assert!(msg.contains("nonexistent-id"));
                }
                other => panic!("Expected Fatal error, got {:?}", other),
            }
        }

        #[tokio::test]
        async fn create_gmail_client_uses_dispatcher_api_base() {
            let server = MockServer::start().await;
            let custom_api_base = format!("{}/gmail/v1/users", &server.uri());

            // Set up mock to verify the custom API base is used
            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/profile"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "emailAddress": "user@example.com",
                    "historyId": "12345"
                })))
                .expect(1)
                .mount(&server)
                .await;

            let (db, _dir) = setup_db().await;
            let (_, account_id) = setup_account(&db).await;

            let dispatcher = JobDispatcher::new(
                db.clone(),
                reqwest::Client::new(),
                Arc::new(MockLLMClient::new()),
                PolicyConfig::default(),
            )
            .with_gmail_api_base(custom_api_base);

            let client = create_gmail_client(&dispatcher, &account_id)
                .await
                .expect("create client");

            // Verify the client was created successfully by making an API call
            // This also verifies the custom API base is being used since the mock
            // expects a request at the custom endpoint
            let profile = client.get_profile().await.expect("get profile");
            assert_eq!(profile.email_address, "user@example.com");
        }

        #[tokio::test]
        async fn create_gmail_client_uses_default_api_base_when_not_set() {
            let (db, _dir) = setup_db().await;
            let (_, account_id) = setup_account(&db).await;

            let dispatcher = JobDispatcher::new(
                db.clone(),
                reqwest::Client::new(),
                Arc::new(MockLLMClient::new()),
                PolicyConfig::default(),
            );

            // Verify client creation succeeds and returns the expected type
            let result = create_gmail_client(&dispatcher, &account_id).await;
            assert!(result.is_ok(), "create_gmail_client should succeed");
        }

        #[tokio::test]
        async fn create_gmail_client_uses_account_credentials() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            // Set up mock to verify the access token is used
            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/profile"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "emailAddress": "user@example.com",
                    "historyId": "12345"
                })))
                .expect(1)
                .mount(&server)
                .await;

            let (db, _dir) = setup_db().await;
            let (_, account_id) = setup_account(&db).await;

            let dispatcher = JobDispatcher::new(
                db.clone(),
                reqwest::Client::new(),
                Arc::new(MockLLMClient::new()),
                PolicyConfig::default(),
            )
            .with_gmail_api_base(api_base);

            let client = create_gmail_client(&dispatcher, &account_id)
                .await
                .expect("create client");

            // Make a request to verify the client works
            let profile = client.get_profile().await.expect("get profile");
            assert_eq!(profile.email_address, "user@example.com");
        }

        #[tokio::test]
        async fn create_gmail_client_returns_error_for_nonexistent_account() {
            let (db, _dir) = setup_db().await;

            let dispatcher = JobDispatcher::new(
                db.clone(),
                reqwest::Client::new(),
                Arc::new(MockLLMClient::new()),
                PolicyConfig::default(),
            );

            // Try to create client with non-existent account
            let result = create_gmail_client(&dispatcher, "nonexistent-account-id").await;

            match result {
                Err(JobError::Fatal(msg)) => {
                    assert!(
                        msg.contains("not found") || msg.contains("Not found"),
                        "Error message should indicate account not found: {}",
                        msg
                    );
                }
                Err(other) => panic!("Expected Fatal error, got different JobError: {:?}", other),
                Ok(_) => panic!("Expected Fatal error, but got Ok"),
            }
        }
    }

    // ===== Tests for action execution handlers =====

    mod action_handler_tests {
        use super::*;
        use crate::accounts::{Account, AccountConfig, AccountRepository, PubsubConfig};
        use crate::config::PolicyConfig;
        use crate::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
        use crate::decisions::{ActionRepository, ActionStatus, NewAction};
        use crate::gmail::{NoopTokenStore, OAuthTokens};
        use crate::labels::LabelRepository;
        use crate::llm::MockLLMClient;
        use crate::messages::{Mailbox, MessageRepository, NewMessage};
        use crate::migrations::run_migrations;
        use crate::queue::JobQueue;
        use crate::threads::ThreadRepository;
        use chrono::Utc;
        use tempfile::TempDir;
        use wiremock::matchers::{body_json, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        async fn setup_db() -> (crate::Database, TempDir) {
            let dir = TempDir::new().expect("temp dir");
            let db_name = format!("db_{}.sqlite", uuid::Uuid::new_v4());
            let db_path = dir.path().join(db_name);
            let db = crate::Database::new(&db_path).await.expect("create db");
            run_migrations(&db).await.expect("migrations");
            (db, dir)
        }

        async fn setup_account(db: &crate::Database) -> (Account, String) {
            let repo = AccountRepository::new(db.clone());
            let config = AccountConfig {
                client_id: "client".into(),
                client_secret: "secret".into(),
                oauth: OAuthTokens {
                    access_token: "access_token".into(),
                    refresh_token: "refresh_token".into(),
                    expires_at: Utc::now() + chrono::Duration::hours(1),
                },
                pubsub: PubsubConfig::default(),
            };
            let account = repo
                .create(
                    DEFAULT_ORG_ID,
                    DEFAULT_USER_ID,
                    "user@example.com",
                    Some("User".into()),
                    config,
                )
                .await
                .expect("create account");

            (account.clone(), account.id)
        }

        async fn setup_message(
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
                    "thread-123",
                    Some("Test Subject".to_string()),
                    Some("Test snippet".to_string()),
                    Some(Utc::now()),
                    json!({}),
                )
                .await
                .expect("create thread");

            let msg_repo = MessageRepository::new(db.clone());
            let msg = msg_repo
                .upsert(NewMessage {
                    org_id: DEFAULT_ORG_ID,
                    user_id: DEFAULT_USER_ID,
                    account_id: account_id.to_string(),
                    thread_id: thread.id.clone(),
                    provider_message_id: provider_message_id.to_string(),
                    from_email: Some("sender@example.com".to_string()),
                    from_name: Some("Sender".to_string()),
                    to: vec![Mailbox {
                        email: "user@example.com".to_string(),
                        name: Some("User".to_string()),
                    }],
                    cc: vec![],
                    bcc: vec![],
                    subject: Some("Test Subject".to_string()),
                    received_at: Some(Utc::now()),
                    internal_date: Some(Utc::now()),
                    labels: vec!["INBOX".to_string(), "UNREAD".to_string()],
                    headers: vec![],
                    body_plain: Some("Test body".to_string()),
                    body_html: None,
                    snippet: Some("Test snippet".to_string()),
                    raw_json: json!({}),
                })
                .await
                .expect("create message");

            msg.id
        }

        async fn setup_action(
            db: &crate::Database,
            account_id: &str,
            message_id: &str,
            action_type: &str,
            parameters: Value,
        ) -> String {
            let repo = ActionRepository::new(db.clone());
            let action = repo
                .create(NewAction {
                    org_id: DEFAULT_ORG_ID,
                    user_id: DEFAULT_USER_ID,
                    account_id: account_id.to_string(),
                    message_id: message_id.to_string(),
                    decision_id: None,
                    action_type: action_type.to_string(),
                    parameters_json: parameters,
                    status: ActionStatus::Queued,
                    error_message: None,
                    executed_at: None,
                    undo_hint_json: json!({}),
                    trace_id: None,
                })
                .await
                .expect("create action");

            action.id
        }

        fn build_gmail_message_response(message_id: &str, labels: Vec<&str>) -> serde_json::Value {
            json!({
                "id": message_id,
                "threadId": "thread-123",
                "labelIds": labels,
                "snippet": "Test snippet",
                "internalDate": "1730000000000"
            })
        }

        // ===== Tests for execute_archive =====

        #[tokio::test]
        async fn execute_archive_removes_inbox_label() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            // Mock for pre-image capture (GET message)
            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    build_gmail_message_response("msg-123", vec!["INBOX", "UNREAD"]),
                ))
                .expect(1)
                .mount(&server)
                .await;

            // Mock for modify_message (POST)
            Mock::given(method("POST"))
                .and(path(
                    "/gmail/v1/users/user@example.com/messages/msg-123/modify",
                ))
                .and(body_json(json!({
                    "removeLabelIds": ["INBOX"]
                })))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(build_gmail_message_response("msg-123", vec!["UNREAD"])),
                )
                .expect(1)
                .mount(&server)
                .await;

            let tokens = OAuthTokens {
                access_token: "test_access".into(),
                refresh_token: "test_refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            };

            let client = crate::gmail::GmailClient::new(
                reqwest::Client::new(),
                "user@example.com".to_string(),
                "client_id".to_string(),
                "client_secret".to_string(),
                tokens,
                Arc::new(NoopTokenStore),
            )
            .with_api_base(api_base);

            let pre_image = capture_pre_image(&client, "msg-123")
                .await
                .expect("pre-image");
            let result = execute_archive(&client, "msg-123", &pre_image)
                .await
                .expect("archive");

            assert_eq!(result.undo_hint["action"], "archive");
            assert_eq!(result.undo_hint["inverse_action"], "apply_label");
            assert_eq!(result.undo_hint["inverse_parameters"]["label"], "INBOX");
            assert_eq!(result.undo_hint["pre_in_inbox"], true);
        }

        // ===== Tests for execute_apply_label =====

        #[tokio::test]
        async fn execute_apply_label_adds_label() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(build_gmail_message_response("msg-123", vec!["INBOX"])),
                )
                .expect(1)
                .mount(&server)
                .await;

            Mock::given(method("POST"))
                .and(path(
                    "/gmail/v1/users/user@example.com/messages/msg-123/modify",
                ))
                .and(body_json(json!({
                    "addLabelIds": ["Label_Important"]
                })))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    build_gmail_message_response("msg-123", vec!["INBOX", "Label_Important"]),
                ))
                .expect(1)
                .mount(&server)
                .await;

            let tokens = OAuthTokens {
                access_token: "test_access".into(),
                refresh_token: "test_refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            };

            let client = crate::gmail::GmailClient::new(
                reqwest::Client::new(),
                "user@example.com".to_string(),
                "client_id".to_string(),
                "client_secret".to_string(),
                tokens,
                Arc::new(NoopTokenStore),
            )
            .with_api_base(api_base);

            let pre_image = capture_pre_image(&client, "msg-123")
                .await
                .expect("pre-image");
            let params = json!({"label": "Label_Important"});
            let result = execute_apply_label(&client, "msg-123", &pre_image, &params)
                .await
                .expect("apply_label");

            assert_eq!(result.undo_hint["action"], "apply_label");
            assert_eq!(result.undo_hint["inverse_action"], "remove_label");
            assert_eq!(
                result.undo_hint["inverse_parameters"]["label"],
                "Label_Important"
            );
        }

        // ===== Tests for execute_remove_label =====

        #[tokio::test]
        async fn execute_remove_label_removes_label() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    build_gmail_message_response("msg-123", vec!["INBOX", "Label_Important"]),
                ))
                .expect(1)
                .mount(&server)
                .await;

            Mock::given(method("POST"))
                .and(path(
                    "/gmail/v1/users/user@example.com/messages/msg-123/modify",
                ))
                .and(body_json(json!({
                    "removeLabelIds": ["Label_Important"]
                })))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(build_gmail_message_response("msg-123", vec!["INBOX"])),
                )
                .expect(1)
                .mount(&server)
                .await;

            let tokens = OAuthTokens {
                access_token: "test_access".into(),
                refresh_token: "test_refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            };

            let client = crate::gmail::GmailClient::new(
                reqwest::Client::new(),
                "user@example.com".to_string(),
                "client_id".to_string(),
                "client_secret".to_string(),
                tokens,
                Arc::new(NoopTokenStore),
            )
            .with_api_base(api_base);

            let pre_image = capture_pre_image(&client, "msg-123")
                .await
                .expect("pre-image");
            let params = json!({"label": "Label_Important"});
            let result = execute_remove_label(&client, "msg-123", &pre_image, &params)
                .await
                .expect("remove_label");

            assert_eq!(result.undo_hint["action"], "remove_label");
            assert_eq!(result.undo_hint["inverse_action"], "apply_label");
            assert_eq!(
                result.undo_hint["inverse_parameters"]["label"],
                "Label_Important"
            );
            // Verify the label was in pre-image
            assert!(
                result.undo_hint["pre_labels"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("Label_Important"))
            );
        }

        #[tokio::test]
        async fn execute_apply_label_fails_with_missing_label_parameter() {
            let tokens = OAuthTokens {
                access_token: "test_access".into(),
                refresh_token: "test_refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            };

            let client = crate::gmail::GmailClient::new(
                reqwest::Client::new(),
                "user@example.com".to_string(),
                "client_id".to_string(),
                "client_secret".to_string(),
                tokens,
                Arc::new(NoopTokenStore),
            );

            let pre_image = PreImageState::from_labels(&["INBOX".to_string()]);
            let params = json!({}); // Missing "label" parameter
            let result = execute_apply_label(&client, "msg-123", &pre_image, &params).await;

            assert!(result.is_err());
            match result.unwrap_err() {
                GmailClientError::InvalidParameter(msg) => {
                    assert!(msg.contains("apply_label"));
                    assert!(msg.contains("label"));
                }
                other => panic!("expected InvalidParameter error, got {:?}", other),
            }
        }

        #[tokio::test]
        async fn execute_apply_label_fails_with_empty_label_parameter() {
            let tokens = OAuthTokens {
                access_token: "test_access".into(),
                refresh_token: "test_refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            };

            let client = crate::gmail::GmailClient::new(
                reqwest::Client::new(),
                "user@example.com".to_string(),
                "client_id".to_string(),
                "client_secret".to_string(),
                tokens,
                Arc::new(NoopTokenStore),
            );

            let pre_image = PreImageState::from_labels(&["INBOX".to_string()]);
            let params = json!({"label": ""}); // Empty string label
            let result = execute_apply_label(&client, "msg-123", &pre_image, &params).await;

            assert!(result.is_err());
            match result.unwrap_err() {
                GmailClientError::InvalidParameter(msg) => {
                    assert!(msg.contains("apply_label"));
                    assert!(msg.contains("non-empty"));
                }
                other => panic!("expected InvalidParameter error, got {:?}", other),
            }
        }

        #[tokio::test]
        async fn execute_apply_label_fails_with_null_label_parameter() {
            let tokens = OAuthTokens {
                access_token: "test_access".into(),
                refresh_token: "test_refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            };

            let client = crate::gmail::GmailClient::new(
                reqwest::Client::new(),
                "user@example.com".to_string(),
                "client_id".to_string(),
                "client_secret".to_string(),
                tokens,
                Arc::new(NoopTokenStore),
            );

            let pre_image = PreImageState::from_labels(&["INBOX".to_string()]);
            let params = json!({"label": null}); // Null label
            let result = execute_apply_label(&client, "msg-123", &pre_image, &params).await;

            assert!(result.is_err());
            match result.unwrap_err() {
                GmailClientError::InvalidParameter(msg) => {
                    assert!(msg.contains("apply_label"));
                }
                other => panic!("expected InvalidParameter error, got {:?}", other),
            }
        }

        #[tokio::test]
        async fn execute_remove_label_fails_with_missing_label_parameter() {
            let tokens = OAuthTokens {
                access_token: "test_access".into(),
                refresh_token: "test_refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            };

            let client = crate::gmail::GmailClient::new(
                reqwest::Client::new(),
                "user@example.com".to_string(),
                "client_id".to_string(),
                "client_secret".to_string(),
                tokens,
                Arc::new(NoopTokenStore),
            );

            let pre_image =
                PreImageState::from_labels(&["INBOX".to_string(), "Label_Test".to_string()]);
            let params = json!({}); // Missing "label" parameter
            let result = execute_remove_label(&client, "msg-123", &pre_image, &params).await;

            assert!(result.is_err());
            match result.unwrap_err() {
                GmailClientError::InvalidParameter(msg) => {
                    assert!(msg.contains("remove_label"));
                    assert!(msg.contains("label"));
                }
                other => panic!("expected InvalidParameter error, got {:?}", other),
            }
        }

        #[tokio::test]
        async fn execute_remove_label_fails_with_empty_label_parameter() {
            let tokens = OAuthTokens {
                access_token: "test_access".into(),
                refresh_token: "test_refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            };

            let client = crate::gmail::GmailClient::new(
                reqwest::Client::new(),
                "user@example.com".to_string(),
                "client_id".to_string(),
                "client_secret".to_string(),
                tokens,
                Arc::new(NoopTokenStore),
            );

            let pre_image =
                PreImageState::from_labels(&["INBOX".to_string(), "Label_Test".to_string()]);
            let params = json!({"label": ""}); // Empty string label
            let result = execute_remove_label(&client, "msg-123", &pre_image, &params).await;

            assert!(result.is_err());
            match result.unwrap_err() {
                GmailClientError::InvalidParameter(msg) => {
                    assert!(msg.contains("remove_label"));
                    assert!(msg.contains("non-empty"));
                }
                other => panic!("expected InvalidParameter error, got {:?}", other),
            }
        }

        // ===== Tests for execute_mark_read and execute_mark_unread =====

        #[tokio::test]
        async fn execute_mark_read_removes_unread_label() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    build_gmail_message_response("msg-123", vec!["INBOX", "UNREAD"]),
                ))
                .expect(1)
                .mount(&server)
                .await;

            Mock::given(method("POST"))
                .and(path(
                    "/gmail/v1/users/user@example.com/messages/msg-123/modify",
                ))
                .and(body_json(json!({
                    "removeLabelIds": ["UNREAD"]
                })))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(build_gmail_message_response("msg-123", vec!["INBOX"])),
                )
                .expect(1)
                .mount(&server)
                .await;

            let tokens = OAuthTokens {
                access_token: "test_access".into(),
                refresh_token: "test_refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            };

            let client = crate::gmail::GmailClient::new(
                reqwest::Client::new(),
                "user@example.com".to_string(),
                "client_id".to_string(),
                "client_secret".to_string(),
                tokens,
                Arc::new(NoopTokenStore),
            )
            .with_api_base(api_base);

            let pre_image = capture_pre_image(&client, "msg-123")
                .await
                .expect("pre-image");
            assert!(pre_image.is_unread); // Message starts as unread

            let result = execute_mark_read(&client, "msg-123", &pre_image)
                .await
                .expect("mark_read");

            assert_eq!(result.undo_hint["action"], "mark_read");
            assert_eq!(result.undo_hint["inverse_action"], "mark_unread");
            assert_eq!(result.undo_hint["pre_unread"], true);
        }

        #[tokio::test]
        async fn execute_mark_unread_adds_unread_label() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(build_gmail_message_response("msg-123", vec!["INBOX"])), // Already read
                )
                .expect(1)
                .mount(&server)
                .await;

            Mock::given(method("POST"))
                .and(path(
                    "/gmail/v1/users/user@example.com/messages/msg-123/modify",
                ))
                .and(body_json(json!({
                    "addLabelIds": ["UNREAD"]
                })))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    build_gmail_message_response("msg-123", vec!["INBOX", "UNREAD"]),
                ))
                .expect(1)
                .mount(&server)
                .await;

            let tokens = OAuthTokens {
                access_token: "test_access".into(),
                refresh_token: "test_refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            };

            let client = crate::gmail::GmailClient::new(
                reqwest::Client::new(),
                "user@example.com".to_string(),
                "client_id".to_string(),
                "client_secret".to_string(),
                tokens,
                Arc::new(NoopTokenStore),
            )
            .with_api_base(api_base);

            let pre_image = capture_pre_image(&client, "msg-123")
                .await
                .expect("pre-image");
            assert!(!pre_image.is_unread); // Message starts as read

            let result = execute_mark_unread(&client, "msg-123", &pre_image)
                .await
                .expect("mark_unread");

            assert_eq!(result.undo_hint["action"], "mark_unread");
            assert_eq!(result.undo_hint["inverse_action"], "mark_read");
            assert_eq!(result.undo_hint["pre_unread"], false);
        }

        // ===== Tests for execute_star and execute_unstar =====

        #[tokio::test]
        async fn execute_star_adds_starred_label() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(build_gmail_message_response("msg-123", vec!["INBOX"])),
                )
                .expect(1)
                .mount(&server)
                .await;

            Mock::given(method("POST"))
                .and(path(
                    "/gmail/v1/users/user@example.com/messages/msg-123/modify",
                ))
                .and(body_json(json!({
                    "addLabelIds": ["STARRED"]
                })))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    build_gmail_message_response("msg-123", vec!["INBOX", "STARRED"]),
                ))
                .expect(1)
                .mount(&server)
                .await;

            let tokens = OAuthTokens {
                access_token: "test_access".into(),
                refresh_token: "test_refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            };

            let client = crate::gmail::GmailClient::new(
                reqwest::Client::new(),
                "user@example.com".to_string(),
                "client_id".to_string(),
                "client_secret".to_string(),
                tokens,
                Arc::new(NoopTokenStore),
            )
            .with_api_base(api_base);

            let pre_image = capture_pre_image(&client, "msg-123")
                .await
                .expect("pre-image");
            assert!(!pre_image.is_starred);

            let result = execute_star(&client, "msg-123", &pre_image)
                .await
                .expect("star");

            assert_eq!(result.undo_hint["action"], "star");
            assert_eq!(result.undo_hint["inverse_action"], "unstar");
            assert_eq!(result.undo_hint["pre_starred"], false);
        }

        #[tokio::test]
        async fn execute_unstar_removes_starred_label() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    build_gmail_message_response("msg-123", vec!["INBOX", "STARRED"]),
                ))
                .expect(1)
                .mount(&server)
                .await;

            Mock::given(method("POST"))
                .and(path(
                    "/gmail/v1/users/user@example.com/messages/msg-123/modify",
                ))
                .and(body_json(json!({
                    "removeLabelIds": ["STARRED"]
                })))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(build_gmail_message_response("msg-123", vec!["INBOX"])),
                )
                .expect(1)
                .mount(&server)
                .await;

            let tokens = OAuthTokens {
                access_token: "test_access".into(),
                refresh_token: "test_refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            };

            let client = crate::gmail::GmailClient::new(
                reqwest::Client::new(),
                "user@example.com".to_string(),
                "client_id".to_string(),
                "client_secret".to_string(),
                tokens,
                Arc::new(NoopTokenStore),
            )
            .with_api_base(api_base);

            let pre_image = capture_pre_image(&client, "msg-123")
                .await
                .expect("pre-image");
            assert!(pre_image.is_starred);

            let result = execute_unstar(&client, "msg-123", &pre_image)
                .await
                .expect("unstar");

            assert_eq!(result.undo_hint["action"], "unstar");
            assert_eq!(result.undo_hint["inverse_action"], "star");
            assert_eq!(result.undo_hint["pre_starred"], true);
        }

        // ===== Tests for execute_trash and execute_restore =====

        #[tokio::test]
        async fn execute_trash_moves_to_trash() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    build_gmail_message_response("msg-123", vec!["INBOX", "IMPORTANT"]),
                ))
                .expect(1)
                .mount(&server)
                .await;

            Mock::given(method("POST"))
                .and(path(
                    "/gmail/v1/users/user@example.com/messages/msg-123/trash",
                ))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(build_gmail_message_response("msg-123", vec!["TRASH"])),
                )
                .expect(1)
                .mount(&server)
                .await;

            let tokens = OAuthTokens {
                access_token: "test_access".into(),
                refresh_token: "test_refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            };

            let client = crate::gmail::GmailClient::new(
                reqwest::Client::new(),
                "user@example.com".to_string(),
                "client_id".to_string(),
                "client_secret".to_string(),
                tokens,
                Arc::new(NoopTokenStore),
            )
            .with_api_base(api_base);

            let pre_image = capture_pre_image(&client, "msg-123")
                .await
                .expect("pre-image");
            assert!(!pre_image.is_in_trash);

            let result = execute_trash(&client, "msg-123", &pre_image)
                .await
                .expect("trash");

            assert_eq!(result.undo_hint["action"], "trash");
            assert_eq!(result.undo_hint["inverse_action"], "restore");
            assert_eq!(result.undo_hint["pre_in_trash"], false);
            // Pre-image should have the original labels for potential undo
            assert_eq!(
                result.undo_hint["pre_labels"],
                json!(["INBOX", "IMPORTANT"])
            );
        }

        #[tokio::test]
        async fn execute_restore_removes_from_trash() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(build_gmail_message_response("msg-123", vec!["TRASH"])),
                )
                .expect(1)
                .mount(&server)
                .await;

            Mock::given(method("POST"))
                .and(path(
                    "/gmail/v1/users/user@example.com/messages/msg-123/untrash",
                ))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(build_gmail_message_response("msg-123", vec!["INBOX"])),
                )
                .expect(1)
                .mount(&server)
                .await;

            let tokens = OAuthTokens {
                access_token: "test_access".into(),
                refresh_token: "test_refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            };

            let client = crate::gmail::GmailClient::new(
                reqwest::Client::new(),
                "user@example.com".to_string(),
                "client_id".to_string(),
                "client_secret".to_string(),
                tokens,
                Arc::new(NoopTokenStore),
            )
            .with_api_base(api_base);

            let pre_image = capture_pre_image(&client, "msg-123")
                .await
                .expect("pre-image");
            assert!(pre_image.is_in_trash);

            let result = execute_restore(&client, "msg-123", &pre_image)
                .await
                .expect("restore");

            assert_eq!(result.undo_hint["action"], "restore");
            assert_eq!(result.undo_hint["inverse_action"], "trash");
            assert_eq!(result.undo_hint["pre_in_trash"], true);
        }

        // ===== Tests for execute_delete =====

        #[tokio::test]
        async fn execute_delete_permanently_deletes() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            // No pre-image capture for delete - it's irreversible
            Mock::given(method("DELETE"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .respond_with(ResponseTemplate::new(204)) // No content
                .expect(1)
                .mount(&server)
                .await;

            let tokens = OAuthTokens {
                access_token: "test_access".into(),
                refresh_token: "test_refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            };

            let client = crate::gmail::GmailClient::new(
                reqwest::Client::new(),
                "user@example.com".to_string(),
                "client_id".to_string(),
                "client_secret".to_string(),
                tokens,
                Arc::new(NoopTokenStore),
            )
            .with_api_base(api_base);

            let result = execute_delete(&client, "msg-123").await.expect("delete");

            assert_eq!(result.undo_hint["action"], "delete");
            assert_eq!(result.undo_hint["inverse_action"], "none");
            assert_eq!(result.undo_hint["irreversible"], true);
        }

        // ===== Tests for execute_action dispatcher =====

        #[tokio::test]
        async fn execute_action_dispatches_to_archive() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    build_gmail_message_response("msg-123", vec!["INBOX", "UNREAD"]),
                ))
                .mount(&server)
                .await;

            Mock::given(method("POST"))
                .and(path(
                    "/gmail/v1/users/user@example.com/messages/msg-123/modify",
                ))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(build_gmail_message_response("msg-123", vec!["UNREAD"])),
                )
                .mount(&server)
                .await;

            let (db, _dir) = setup_db().await;
            let (_, account_id) = setup_account(&db).await;
            let message_id = setup_message(&db, &account_id, "msg-123").await;
            let action_id = setup_action(&db, &account_id, &message_id, "archive", json!({})).await;

            let action_repo = ActionRepository::new(db.clone());
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");

            let tokens = OAuthTokens {
                access_token: "test_access".into(),
                refresh_token: "test_refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            };

            let client = crate::gmail::GmailClient::new(
                reqwest::Client::new(),
                "user@example.com".to_string(),
                "client_id".to_string(),
                "client_secret".to_string(),
                tokens,
                Arc::new(NoopTokenStore),
            )
            .with_api_base(api_base);

            let result = execute_action(&client, "msg-123", &action)
                .await
                .expect("execute_action");

            assert_eq!(result.undo_hint["action"], "archive");
        }

        #[tokio::test]
        async fn execute_action_dispatches_to_delete_without_pre_image() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            // Delete should NOT call GET first (no pre-image needed)
            Mock::given(method("DELETE"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .respond_with(ResponseTemplate::new(204))
                .expect(1)
                .mount(&server)
                .await;

            let (db, _dir) = setup_db().await;
            let (_, account_id) = setup_account(&db).await;
            let message_id = setup_message(&db, &account_id, "msg-123").await;
            let action_id = setup_action(&db, &account_id, &message_id, "delete", json!({})).await;

            let action_repo = ActionRepository::new(db.clone());
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");

            let tokens = OAuthTokens {
                access_token: "test_access".into(),
                refresh_token: "test_refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            };

            let client = crate::gmail::GmailClient::new(
                reqwest::Client::new(),
                "user@example.com".to_string(),
                "client_id".to_string(),
                "client_secret".to_string(),
                tokens,
                Arc::new(NoopTokenStore),
            )
            .with_api_base(api_base);

            let result = execute_action(&client, "msg-123", &action)
                .await
                .expect("execute_action");

            assert_eq!(result.undo_hint["action"], "delete");
            assert_eq!(result.undo_hint["irreversible"], true);
        }

        #[tokio::test]
        async fn execute_action_fails_for_unsupported_action_type() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            // Pre-image GET - still needed since unsupported action goes through pre-image capture
            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(build_gmail_message_response("msg-123", vec!["INBOX"])),
                )
                .mount(&server)
                .await;

            let (db, _dir) = setup_db().await;
            let (_, account_id) = setup_account(&db).await;
            let message_id = setup_message(&db, &account_id, "msg-123").await;
            let action_id =
                setup_action(&db, &account_id, &message_id, "unknown_action", json!({})).await;

            let action_repo = ActionRepository::new(db.clone());
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");

            let tokens = OAuthTokens {
                access_token: "test_access".into(),
                refresh_token: "test_refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            };

            let client = crate::gmail::GmailClient::new(
                reqwest::Client::new(),
                "user@example.com".to_string(),
                "client_id".to_string(),
                "client_secret".to_string(),
                tokens,
                Arc::new(NoopTokenStore),
            )
            .with_api_base(api_base);

            let result = execute_action(&client, "msg-123", &action).await;

            // Should fail with UnsupportedAction error
            assert!(result.is_err());
            let err = result.unwrap_err();
            match err {
                GmailClientError::UnsupportedAction(action_type) => {
                    assert_eq!(action_type, "unknown_action");
                }
                other => panic!("expected UnsupportedAction error, got {:?}", other),
            }
        }

        // ===== Tests for handle_action_gmail =====

        #[tokio::test]
        async fn handle_action_gmail_executes_archive_and_marks_completed() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    build_gmail_message_response("msg-123", vec!["INBOX", "UNREAD"]),
                ))
                .mount(&server)
                .await;

            Mock::given(method("POST"))
                .and(path(
                    "/gmail/v1/users/user@example.com/messages/msg-123/modify",
                ))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(build_gmail_message_response("msg-123", vec!["UNREAD"])),
                )
                .mount(&server)
                .await;

            let (db, _dir) = setup_db().await;
            let (_, account_id) = setup_account(&db).await;
            let message_id = setup_message(&db, &account_id, "msg-123").await;
            let action_id = setup_action(&db, &account_id, &message_id, "archive", json!({})).await;

            let queue = JobQueue::new(db.clone());
            let job_id = queue
                .enqueue(
                    JOB_TYPE,
                    json!({"account_id": account_id.clone(), "action_id": action_id.clone()}),
                    None,
                    1,
                )
                .await
                .expect("enqueue job");
            let job = queue.fetch_job(&job_id).await.expect("fetch job");

            let dispatcher = JobDispatcher::new(
                db.clone(),
                reqwest::Client::new(),
                Arc::new(MockLLMClient::new()),
                PolicyConfig::default(),
            )
            .with_gmail_api_base(api_base);

            handle_action_gmail(&dispatcher, job).await.expect("handle");

            // Verify action status
            let action_repo = ActionRepository::new(db.clone());
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");

            assert_eq!(action.status, ActionStatus::Completed);
            assert!(action.executed_at.is_some());
            assert_eq!(action.undo_hint_json["action"], "archive");
            assert_eq!(action.undo_hint_json["inverse_action"], "apply_label");
        }

        #[tokio::test]
        async fn handle_action_gmail_marks_failed_on_gmail_error() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            // Return 404 for the message - it doesn't exist
            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .respond_with(ResponseTemplate::new(404))
                .mount(&server)
                .await;

            let (db, _dir) = setup_db().await;
            let (_, account_id) = setup_account(&db).await;
            let message_id = setup_message(&db, &account_id, "msg-123").await;
            let action_id = setup_action(&db, &account_id, &message_id, "archive", json!({})).await;

            let queue = JobQueue::new(db.clone());
            let job_id = queue
                .enqueue(
                    JOB_TYPE,
                    json!({"account_id": account_id.clone(), "action_id": action_id.clone()}),
                    None,
                    1,
                )
                .await
                .expect("enqueue job");
            let job = queue.fetch_job(&job_id).await.expect("fetch job");

            let dispatcher = JobDispatcher::new(
                db.clone(),
                reqwest::Client::new(),
                Arc::new(MockLLMClient::new()),
                PolicyConfig::default(),
            )
            .with_gmail_api_base(api_base);

            // Should return a Fatal error for 404
            let result = handle_action_gmail(&dispatcher, job).await;
            assert!(matches!(result, Err(JobError::Fatal(_))));

            // Verify action was marked as failed
            let action_repo = ActionRepository::new(db.clone());
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");

            assert_eq!(action.status, ActionStatus::Failed);
            assert!(action.error_message.is_some());
        }

        #[tokio::test]
        async fn handle_action_gmail_skips_completed_action() {
            let (db, _dir) = setup_db().await;
            let (_, account_id) = setup_account(&db).await;
            let message_id = setup_message(&db, &account_id, "msg-123").await;
            let action_id = setup_action(&db, &account_id, &message_id, "archive", json!({})).await;

            // Mark action as completed
            let action_repo = ActionRepository::new(db.clone());
            action_repo
                .mark_executing(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("mark executing");
            action_repo
                .mark_completed(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("mark completed");

            let queue = JobQueue::new(db.clone());
            let job_id = queue
                .enqueue(
                    JOB_TYPE,
                    json!({"account_id": account_id.clone(), "action_id": action_id.clone()}),
                    None,
                    1,
                )
                .await
                .expect("enqueue job");
            let job = queue.fetch_job(&job_id).await.expect("fetch job");

            let dispatcher = JobDispatcher::new(
                db.clone(),
                reqwest::Client::new(),
                Arc::new(MockLLMClient::new()),
                PolicyConfig::default(),
            );

            // Should succeed immediately without making any API calls
            handle_action_gmail(&dispatcher, job).await.expect("handle");

            // Action should still be completed
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");
            assert_eq!(action.status, ActionStatus::Completed);
        }

        #[tokio::test]
        async fn handle_action_gmail_skips_approved_pending_action() {
            let (db, _dir) = setup_db().await;
            let (_, account_id) = setup_account(&db).await;
            let message_id = setup_message(&db, &account_id, "msg-123").await;

            // Create action in ApprovedPending status
            let repo = ActionRepository::new(db.clone());
            let action = repo
                .create(NewAction {
                    org_id: DEFAULT_ORG_ID,
                    user_id: DEFAULT_USER_ID,
                    account_id: account_id.to_string(),
                    message_id: message_id.to_string(),
                    decision_id: None,
                    action_type: "delete".to_string(),
                    parameters_json: json!({}),
                    status: ActionStatus::ApprovedPending,
                    error_message: None,
                    executed_at: None,
                    undo_hint_json: json!({}),
                    trace_id: None,
                })
                .await
                .expect("create action");

            let queue = JobQueue::new(db.clone());
            let job_id = queue
                .enqueue(
                    JOB_TYPE,
                    json!({"account_id": account_id.clone(), "action_id": action.id.clone()}),
                    None,
                    1,
                )
                .await
                .expect("enqueue job");
            let job = queue.fetch_job(&job_id).await.expect("fetch job");

            let dispatcher = JobDispatcher::new(
                db.clone(),
                reqwest::Client::new(),
                Arc::new(MockLLMClient::new()),
                PolicyConfig::default(),
            );

            // Should succeed immediately without executing
            handle_action_gmail(&dispatcher, job).await.expect("handle");

            // Action should still be ApprovedPending
            let action = repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action.id)
                .await
                .expect("get action");
            assert_eq!(action.status, ActionStatus::ApprovedPending);
        }

        #[tokio::test]
        async fn handle_action_gmail_rejects_mismatched_account() {
            let (db, _dir) = setup_db().await;
            let (_, account_id) = setup_account(&db).await;
            let message_id = setup_message(&db, &account_id, "msg-123").await;
            let action_id = setup_action(&db, &account_id, &message_id, "archive", json!({})).await;

            let queue = JobQueue::new(db.clone());
            let job_id = queue
                .enqueue(
                    JOB_TYPE,
                    json!({"account_id": "different-account", "action_id": action_id.clone()}),
                    None,
                    1,
                )
                .await
                .expect("enqueue job");
            let job = queue.fetch_job(&job_id).await.expect("fetch job");

            let dispatcher = JobDispatcher::new(
                db.clone(),
                reqwest::Client::new(),
                Arc::new(MockLLMClient::new()),
                PolicyConfig::default(),
            );

            let result = handle_action_gmail(&dispatcher, job).await;

            match result {
                Err(JobError::Fatal(msg)) => {
                    assert!(msg.contains("does not belong to account"));
                }
                other => panic!("Expected Fatal error, got {:?}", other),
            }
        }

        #[tokio::test]
        async fn handle_action_gmail_returns_retryable_on_rate_limit() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            // Return 429 rate limit
            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .respond_with(ResponseTemplate::new(429))
                .mount(&server)
                .await;

            let (db, _dir) = setup_db().await;
            let (_, account_id) = setup_account(&db).await;
            let message_id = setup_message(&db, &account_id, "msg-123").await;
            let action_id = setup_action(&db, &account_id, &message_id, "archive", json!({})).await;

            let queue = JobQueue::new(db.clone());
            let job_id = queue
                .enqueue(
                    JOB_TYPE,
                    json!({"account_id": account_id.clone(), "action_id": action_id.clone()}),
                    None,
                    1,
                )
                .await
                .expect("enqueue job");
            let job = queue.fetch_job(&job_id).await.expect("fetch job");

            let dispatcher = JobDispatcher::new(
                db.clone(),
                reqwest::Client::new(),
                Arc::new(MockLLMClient::new()),
                PolicyConfig::default(),
            )
            .with_gmail_api_base(api_base);

            let result = handle_action_gmail(&dispatcher, job).await;

            match result {
                Err(JobError::Retryable { .. }) => {}
                other => panic!("Expected Retryable error, got {:?}", other),
            }

            // The action should remain in Executing state so the worker can retry.
            let action_repo = ActionRepository::new(db.clone());
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");

            assert_eq!(action.status, ActionStatus::Executing);
            assert!(action.error_message.is_none());
        }

        #[tokio::test]
        async fn handle_action_gmail_marks_failed_when_retry_exhausted() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            // Return 429 rate limit
            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .respond_with(ResponseTemplate::new(429))
                .mount(&server)
                .await;

            let (db, _dir) = setup_db().await;
            let (_, account_id) = setup_account(&db).await;
            let message_id = setup_message(&db, &account_id, "msg-123").await;
            let action_id = setup_action(&db, &account_id, &message_id, "archive", json!({})).await;

            let queue = JobQueue::new(db.clone());
            let job_id = queue
                .enqueue(
                    JOB_TYPE,
                    json!({"account_id": account_id.clone(), "action_id": action_id.clone()}),
                    None,
                    1,
                )
                .await
                .expect("enqueue job");

            // Simulate the final allowed attempt (no more retries remain).
            let mut job = queue.fetch_job(&job_id).await.expect("fetch job");
            job.attempts = job.max_attempts;

            let dispatcher = JobDispatcher::new(
                db.clone(),
                reqwest::Client::new(),
                Arc::new(MockLLMClient::new()),
                PolicyConfig::default(),
            )
            .with_gmail_api_base(api_base);

            let result = handle_action_gmail(&dispatcher, job).await;

            match result {
                Err(JobError::Retryable { .. }) => {}
                other => panic!("Expected Retryable error, got {:?}", other),
            }

            // Even though the error is retryable, the action should move to Failed because no
            // more retries are available.
            let action_repo = ActionRepository::new(db.clone());
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");

            assert_eq!(action.status, ActionStatus::Failed);
            let error_message = action.error_message.expect("error message should be set");
            assert!(
                error_message.contains("rate limited") || error_message.contains("429"),
                "error message should describe rate limit"
            );
        }

        #[tokio::test]
        async fn handle_action_gmail_executes_all_action_types() {
            // Test that each action type is correctly dispatched
            let action_types = vec![
                ("archive", json!({})),
                ("apply_label", json!({"label": "Label_123"})),
                ("remove_label", json!({"label": "Label_123"})),
                ("mark_read", json!({})),
                ("mark_unread", json!({})),
                ("star", json!({})),
                ("unstar", json!({})),
                ("trash", json!({})),
                ("restore", json!({})),
            ];

            for (action_type, params) in action_types {
                let server = MockServer::start().await;
                let api_base = format!("{}/gmail/v1/users", &server.uri());

                // Mock pre-image GET
                Mock::given(method("GET"))
                    .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(
                        build_gmail_message_response(
                            "msg-123",
                            vec!["INBOX", "UNREAD", "STARRED", "Label_123"],
                        ),
                    ))
                    .mount(&server)
                    .await;

                // Mock modify
                Mock::given(method("POST"))
                    .and(path(
                        "/gmail/v1/users/user@example.com/messages/msg-123/modify",
                    ))
                    .respond_with(
                        ResponseTemplate::new(200)
                            .set_body_json(build_gmail_message_response("msg-123", vec!["INBOX"])),
                    )
                    .mount(&server)
                    .await;

                // Mock trash
                Mock::given(method("POST"))
                    .and(path(
                        "/gmail/v1/users/user@example.com/messages/msg-123/trash",
                    ))
                    .respond_with(
                        ResponseTemplate::new(200)
                            .set_body_json(build_gmail_message_response("msg-123", vec!["TRASH"])),
                    )
                    .mount(&server)
                    .await;

                // Mock untrash
                Mock::given(method("POST"))
                    .and(path(
                        "/gmail/v1/users/user@example.com/messages/msg-123/untrash",
                    ))
                    .respond_with(
                        ResponseTemplate::new(200)
                            .set_body_json(build_gmail_message_response("msg-123", vec!["INBOX"])),
                    )
                    .mount(&server)
                    .await;

                let (db, _dir) = setup_db().await;
                let (_, account_id) = setup_account(&db).await;
                let message_id = setup_message(&db, &account_id, "msg-123").await;
                let action_id =
                    setup_action(&db, &account_id, &message_id, action_type, params).await;

                let queue = JobQueue::new(db.clone());
                let job_id = queue
                    .enqueue(
                        JOB_TYPE,
                        json!({"account_id": account_id.clone(), "action_id": action_id.clone()}),
                        None,
                        1,
                    )
                    .await
                    .expect("enqueue job");
                let job = queue.fetch_job(&job_id).await.expect("fetch job");

                let dispatcher = JobDispatcher::new(
                    db.clone(),
                    reqwest::Client::new(),
                    Arc::new(MockLLMClient::new()),
                    PolicyConfig::default(),
                )
                .with_gmail_api_base(api_base);

                handle_action_gmail(&dispatcher, job)
                    .await
                    .unwrap_or_else(|e| {
                        panic!("Failed to execute action type '{}': {:?}", action_type, e)
                    });

                // Verify action was completed
                let action_repo = ActionRepository::new(db.clone());
                let action = action_repo
                    .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                    .await
                    .expect("get action");

                assert_eq!(
                    action.status,
                    ActionStatus::Completed,
                    "Action type '{}' should be completed",
                    action_type
                );
                assert_eq!(
                    action.undo_hint_json["action"], action_type,
                    "Undo hint should have correct action type for '{}'",
                    action_type
                );
            }
        }

        #[tokio::test]
        async fn handle_action_gmail_executes_delete() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            // Delete doesn't need pre-image
            Mock::given(method("DELETE"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .respond_with(ResponseTemplate::new(204))
                .expect(1)
                .mount(&server)
                .await;

            let (db, _dir) = setup_db().await;
            let (_, account_id) = setup_account(&db).await;
            let message_id = setup_message(&db, &account_id, "msg-123").await;
            let action_id = setup_action(&db, &account_id, &message_id, "delete", json!({})).await;

            let queue = JobQueue::new(db.clone());
            let job_id = queue
                .enqueue(
                    JOB_TYPE,
                    json!({"account_id": account_id.clone(), "action_id": action_id.clone()}),
                    None,
                    1,
                )
                .await
                .expect("enqueue job");
            let job = queue.fetch_job(&job_id).await.expect("fetch job");

            let dispatcher = JobDispatcher::new(
                db.clone(),
                reqwest::Client::new(),
                Arc::new(MockLLMClient::new()),
                PolicyConfig::default(),
            )
            .with_gmail_api_base(api_base);

            handle_action_gmail(&dispatcher, job).await.expect("handle");

            // Verify action was completed with irreversible undo hint
            let action_repo = ActionRepository::new(db.clone());
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");

            assert_eq!(action.status, ActionStatus::Completed);
            assert_eq!(action.undo_hint_json["action"], "delete");
            assert_eq!(action.undo_hint_json["irreversible"], true);
        }

        #[tokio::test]
        async fn handle_action_gmail_schedules_unsnooze_and_applies_label() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            let snooze_until = (Utc::now() + chrono::Duration::minutes(5))
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

            // Pre-image capture
            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    build_gmail_message_response("msg-123", vec!["INBOX", "UNREAD"]),
                ))
                .expect(1)
                .mount(&server)
                .await;

            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/labels"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({"labels": []})))
                .expect(1)
                .mount(&server)
                .await;

            // Create label
            Mock::given(method("POST"))
                .and(path("/gmail/v1/users/user@example.com/labels"))
                .and(body_json(json!({"name": "Ashford/Snoozed"})))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "id": "Label_Snoozed",
                    "name": "Ashford/Snoozed",
                    "type": "user"
                })))
                .expect(1)
                .mount(&server)
                .await;

            // Apply snooze: remove INBOX, add snooze label
            Mock::given(method("POST"))
                .and(path(
                    "/gmail/v1/users/user@example.com/messages/msg-123/modify",
                ))
                .and(body_json(json!({
                    "addLabelIds": ["Label_Snoozed"],
                    "removeLabelIds": ["INBOX"]
                })))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    build_gmail_message_response("msg-123", vec!["Label_Snoozed", "UNREAD"]),
                ))
                .expect(1)
                .mount(&server)
                .await;

            let (db, _dir) = setup_db().await;
            let (_, account_id) = setup_account(&db).await;
            let message_id = setup_message(&db, &account_id, "msg-123").await;
            let action_id = setup_action(
                &db,
                &account_id,
                &message_id,
                "snooze",
                json!({"until": snooze_until}),
            )
            .await;

            let queue = JobQueue::new(db.clone());
            let job_id = queue
                .enqueue(
                    JOB_TYPE,
                    json!({"account_id": account_id.clone(), "action_id": action_id.clone()}),
                    None,
                    1,
                )
                .await
                .expect("enqueue job");
            let job = queue.fetch_job(&job_id).await.expect("fetch job");

            let dispatcher = JobDispatcher::new(
                db.clone(),
                reqwest::Client::new(),
                Arc::new(MockLLMClient::new()),
                PolicyConfig::default(),
            )
            .with_gmail_api_base(api_base);

            handle_action_gmail(&dispatcher, job)
                .await
                .expect("snooze action should succeed");

            // Verify action completion and undo hint metadata
            let action_repo = ActionRepository::new(db.clone());
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");

            assert_eq!(action.status, ActionStatus::Completed);
            assert_eq!(action.undo_hint_json["action"], "snooze");
            assert_eq!(action.undo_hint_json["inverse_action"], "none");
            assert_eq!(action.undo_hint_json["snooze_label"], "Label_Snoozed");
            assert_eq!(
                action.undo_hint_json["inverse_parameters"]["add_labels"],
                json!(["INBOX"])
            );
            assert_eq!(
                action.undo_hint_json["inverse_parameters"]["remove_labels"],
                json!(["Label_Snoozed"])
            );

            let unsnooze_job_id = action.undo_hint_json["unsnooze_job_id"]
                .as_str()
                .expect("job id present")
                .to_string();

            assert_eq!(
                action.undo_hint_json["inverse_parameters"]["cancel_unsnooze_job_id"],
                json!(unsnooze_job_id)
            );

            let queue = JobQueue::new(db.clone());
            let unsnooze_job = queue
                .fetch_job(&unsnooze_job_id)
                .await
                .expect("unsnooze job exists");
            assert_eq!(unsnooze_job.job_type, JOB_TYPE_UNSNOOZE_GMAIL);
            assert_eq!(
                unsnooze_job.payload["snooze_label_id"],
                json!("Label_Snoozed")
            );

            let parsed_until = DateTime::parse_from_rfc3339(
                action.undo_hint_json["snooze_until"].as_str().unwrap(),
            )
            .unwrap()
            .with_timezone(&Utc);
            let diff = unsnooze_job
                .not_before
                .expect("not_before set")
                .signed_duration_since(parsed_until)
                .num_seconds()
                .abs();
            assert!(diff <= 1, "unsnooze job scheduled at expected time");

            // Snooze label should be stored locally
            let label_repo = LabelRepository::new(db.clone());
            let stored = label_repo
                .get_by_name(
                    DEFAULT_ORG_ID,
                    DEFAULT_USER_ID,
                    &account_id,
                    "Ashford/Snoozed",
                )
                .await
                .expect("label stored");
            assert_eq!(stored.provider_label_id, "Label_Snoozed");
        }

        #[tokio::test]
        async fn handle_action_gmail_recreates_missing_snooze_label_when_cached() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            let snooze_until = (Utc::now() + chrono::Duration::minutes(10))
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

            // Pre-image capture
            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(build_gmail_message_response("msg-123", vec!["INBOX"])),
                )
                .expect(1)
                .mount(&server)
                .await;

            // Gmail currently has a fresh snooze label with a new id, cached id is stale
            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/labels"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "labels": [
                        {"id": "Label_Fresh", "name": "Ashford/Snoozed", "type": "user"}
                    ]
                })))
                .expect(1)
                .mount(&server)
                .await;

            // Modify should use the fresh label id
            Mock::given(method("POST"))
                .and(path(
                    "/gmail/v1/users/user@example.com/messages/msg-123/modify",
                ))
                .and(body_json(json!({
                    "addLabelIds": ["Label_Fresh"],
                    "removeLabelIds": ["INBOX"]
                })))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(build_gmail_message_response(
                        "msg-123",
                        vec!["Label_Fresh"],
                    )),
                )
                .expect(1)
                .mount(&server)
                .await;

            let (db, _dir) = setup_db().await;
            let (_, account_id) = setup_account(&db).await;
            let message_id = setup_message(&db, &account_id, "msg-123").await;

            // Seed stale cached label
            let label_repo = LabelRepository::new(db.clone());
            label_repo
                .upsert(NewLabel {
                    org_id: DEFAULT_ORG_ID,
                    user_id: DEFAULT_USER_ID,
                    account_id: account_id.clone(),
                    provider_label_id: "Label_Stale".to_string(),
                    name: "Ashford/Snoozed".to_string(),
                    label_type: "user".to_string(),
                    description: None,
                    available_to_classifier: true,
                    message_list_visibility: None,
                    label_list_visibility: None,
                    background_color: None,
                    text_color: None,
                })
                .await
                .expect("seed label");

            let action_id = setup_action(
                &db,
                &account_id,
                &message_id,
                "snooze",
                json!({"until": snooze_until}),
            )
            .await;

            let queue = JobQueue::new(db.clone());
            let job_id = queue
                .enqueue(
                    JOB_TYPE,
                    json!({"account_id": account_id.clone(), "action_id": action_id.clone()}),
                    None,
                    1,
                )
                .await
                .expect("enqueue job");
            let job = queue.fetch_job(&job_id).await.expect("fetch job");

            let dispatcher = JobDispatcher::new(
                db.clone(),
                reqwest::Client::new(),
                Arc::new(MockLLMClient::new()),
                PolicyConfig::default(),
            )
            .with_gmail_api_base(api_base);

            handle_action_gmail(&dispatcher, job)
                .await
                .expect("snooze action should succeed");

            let stored = label_repo
                .get_by_name(
                    DEFAULT_ORG_ID,
                    DEFAULT_USER_ID,
                    &account_id,
                    "Ashford/Snoozed",
                )
                .await
                .expect("label stored");
            assert_eq!(stored.provider_label_id, "Label_Fresh");

            let action_repo = ActionRepository::new(db.clone());
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");

            let unsnooze_job_id = action.undo_hint_json["unsnooze_job_id"]
                .as_str()
                .expect("job id present");

            let queue = JobQueue::new(db.clone());
            let unsnooze_job = queue
                .fetch_job(unsnooze_job_id)
                .await
                .expect("unsnooze job exists");
            assert_eq!(
                unsnooze_job.payload["snooze_label_id"],
                json!("Label_Fresh")
            );
        }

        #[tokio::test]
        async fn handle_action_gmail_transitions_from_queued_to_executing() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(build_gmail_message_response("msg-123", vec!["INBOX"])),
                )
                .mount(&server)
                .await;

            Mock::given(method("POST"))
                .and(path(
                    "/gmail/v1/users/user@example.com/messages/msg-123/modify",
                ))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(build_gmail_message_response("msg-123", vec![])),
                )
                .mount(&server)
                .await;

            let (db, _dir) = setup_db().await;
            let (_, account_id) = setup_account(&db).await;
            let message_id = setup_message(&db, &account_id, "msg-123").await;
            let action_id = setup_action(&db, &account_id, &message_id, "archive", json!({})).await;

            // Verify action starts as Queued
            let action_repo = ActionRepository::new(db.clone());
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");
            assert_eq!(action.status, ActionStatus::Queued);

            let queue = JobQueue::new(db.clone());
            let job_id = queue
                .enqueue(
                    JOB_TYPE,
                    json!({"account_id": account_id.clone(), "action_id": action_id.clone()}),
                    None,
                    1,
                )
                .await
                .expect("enqueue job");
            let job = queue.fetch_job(&job_id).await.expect("fetch job");

            let dispatcher = JobDispatcher::new(
                db.clone(),
                reqwest::Client::new(),
                Arc::new(MockLLMClient::new()),
                PolicyConfig::default(),
            )
            .with_gmail_api_base(api_base);

            handle_action_gmail(&dispatcher, job).await.expect("handle");

            // Verify action is now Completed
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");
            assert_eq!(action.status, ActionStatus::Completed);
            assert!(action.executed_at.is_some());
        }

        #[tokio::test]
        async fn handle_action_gmail_continues_execution_if_already_executing() {
            let server = MockServer::start().await;
            let api_base = format!("{}/gmail/v1/users", &server.uri());

            Mock::given(method("GET"))
                .and(path("/gmail/v1/users/user@example.com/messages/msg-123"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(build_gmail_message_response("msg-123", vec!["INBOX"])),
                )
                .mount(&server)
                .await;

            Mock::given(method("POST"))
                .and(path(
                    "/gmail/v1/users/user@example.com/messages/msg-123/modify",
                ))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(build_gmail_message_response("msg-123", vec![])),
                )
                .mount(&server)
                .await;

            let (db, _dir) = setup_db().await;
            let (_, account_id) = setup_account(&db).await;
            let message_id = setup_message(&db, &account_id, "msg-123").await;
            let action_id = setup_action(&db, &account_id, &message_id, "archive", json!({})).await;

            // Pre-mark as executing (simulating a retry after crash)
            let action_repo = ActionRepository::new(db.clone());
            action_repo
                .mark_executing(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("mark executing");

            let queue = JobQueue::new(db.clone());
            let job_id = queue
                .enqueue(
                    JOB_TYPE,
                    json!({"account_id": account_id.clone(), "action_id": action_id.clone()}),
                    None,
                    1,
                )
                .await
                .expect("enqueue job");
            let job = queue.fetch_job(&job_id).await.expect("fetch job");

            let dispatcher = JobDispatcher::new(
                db.clone(),
                reqwest::Client::new(),
                Arc::new(MockLLMClient::new()),
                PolicyConfig::default(),
            )
            .with_gmail_api_base(api_base);

            // Should succeed even though already executing
            handle_action_gmail(&dispatcher, job).await.expect("handle");

            // Verify action completed
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");
            assert_eq!(action.status, ActionStatus::Completed);
        }

        #[tokio::test]
        async fn handle_action_gmail_returns_fatal_for_missing_action() {
            let (db, _dir) = setup_db().await;
            let (_, account_id) = setup_account(&db).await;

            let queue = JobQueue::new(db.clone());
            let job_id = queue
                .enqueue(
                    JOB_TYPE,
                    json!({"account_id": account_id.clone(), "action_id": "nonexistent-action"}),
                    None,
                    1,
                )
                .await
                .expect("enqueue job");
            let job = queue.fetch_job(&job_id).await.expect("fetch job");

            let dispatcher = JobDispatcher::new(
                db.clone(),
                reqwest::Client::new(),
                Arc::new(MockLLMClient::new()),
                PolicyConfig::default(),
            );

            let result = handle_action_gmail(&dispatcher, job).await;

            match result {
                Err(JobError::Fatal(msg)) => {
                    assert!(msg.contains("not found") || msg.contains("Not found"));
                }
                other => panic!("Expected Fatal error, got {:?}", other),
            }
        }

        #[tokio::test]
        async fn handle_action_gmail_returns_fatal_for_invalid_payload() {
            let (db, _dir) = setup_db().await;

            let queue = JobQueue::new(db.clone());
            let job_id = queue
                .enqueue(
                    JOB_TYPE,
                    json!({"invalid": "payload"}), // Missing required fields
                    None,
                    1,
                )
                .await
                .expect("enqueue job");
            let job = queue.fetch_job(&job_id).await.expect("fetch job");

            let dispatcher = JobDispatcher::new(
                db.clone(),
                reqwest::Client::new(),
                Arc::new(MockLLMClient::new()),
                PolicyConfig::default(),
            );

            let result = handle_action_gmail(&dispatcher, job).await;

            match result {
                Err(JobError::Fatal(msg)) => {
                    assert!(msg.contains("invalid") || msg.contains("payload"));
                }
                other => panic!("Expected Fatal error, got {:?}", other),
            }
        }
    }
}
