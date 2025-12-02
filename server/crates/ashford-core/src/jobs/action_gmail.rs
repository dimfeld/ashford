use serde::Deserialize;
use tracing::info;

use crate::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
use crate::decisions::{ActionRepository, ActionStatus};
use crate::{Job, JobError};

use super::{JobDispatcher, map_action_error};

pub const JOB_TYPE: &str = "action.gmail";

#[derive(Debug, Deserialize)]
struct ActionJobPayload {
    pub account_id: String,
    pub action_id: String,
}

/// Execute a Gmail action. This is currently a placeholder that marks the
/// action as executed so the pipeline can continue; provider-side mutations
/// will be implemented in a later phase.
pub async fn handle_action_gmail(
    dispatcher: &JobDispatcher,
    job: Job,
) -> Result<(), JobError> {
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

    match action.status {
        ActionStatus::Queued => {
            repo.mark_executing(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action.id)
                .await
                .map_err(|err| JobError::retryable(format!("failed to mark action executing: {err}")))?;
            repo.mark_completed(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action.id)
                .await
                .map_err(|err| JobError::retryable(format!("failed to mark action completed: {err}")))?;
        }
        ActionStatus::Executing => {
            repo.mark_completed(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action.id)
                .await
                .map_err(|err| JobError::retryable(format!("failed to mark action completed: {err}")))?;
        }
        _ => {
            // Already processed or awaiting approval; nothing to do.
            return Ok(());
        }
    }

    info!(
        account_id = %payload.account_id,
        action_id = %payload.action_id,
        "executed gmail action (stub)"
    );

    Ok(())
}
