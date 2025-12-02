use serde::Deserialize;
use tracing::info;

use crate::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
use crate::decisions::{ActionRepository, ActionStatus};
use crate::{Job, JobError};

use super::{JobDispatcher, map_action_error};

pub const JOB_TYPE: &str = "approval.notify";

#[derive(Debug, Deserialize)]
struct ApprovalPayload {
    pub account_id: String,
    pub action_id: String,
    pub message_id: String,
}

/// Notify approvers that an action requires approval.
/// This is a placeholder that currently just logs; notification fan-out will
/// be added when the approval surface is implemented.
pub async fn handle_approval_notify(
    dispatcher: &JobDispatcher,
    job: Job,
) -> Result<(), JobError> {
    let payload: ApprovalPayload = serde_json::from_value(job.payload.clone())
        .map_err(|err| JobError::Fatal(format!("invalid approval.notify payload: {err}")))?;

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

    if action.status != ActionStatus::ApprovedPending {
        // Already handled or not awaiting approval; nothing to notify.
        return Ok(());
    }

    info!(
        account_id = %payload.account_id,
        action_id = %payload.action_id,
        message_id = %payload.message_id,
        "approval requested (notification stub)"
    );

    Ok(())
}
