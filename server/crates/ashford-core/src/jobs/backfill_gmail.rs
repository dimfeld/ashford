use serde::Deserialize;
use tracing::warn;

use crate::worker::JobError;
use crate::Job;

use super::JobDispatcher;

pub const JOB_TYPE: &str = "backfill.gmail";

#[derive(Debug, Deserialize)]
struct BackfillPayload {
    account_id: String,
    query: String,
}

pub async fn handle_backfill_gmail(_dispatcher: &JobDispatcher, job: Job) -> Result<(), JobError> {
    let payload: BackfillPayload = serde_json::from_value(job.payload.clone())
        .map_err(|err| JobError::Fatal(format!("invalid backfill.gmail payload: {err}")))?;

    // TODO: Implement actual backfill logic
    // 1. List messages using query (with pagination)
    // 2. Enqueue ingest.gmail jobs for each message
    // 3. After final page, get fresh historyId from profile
    // 4. Update account state to Normal with new historyId
    warn!(
        account_id = %payload.account_id,
        query = %payload.query,
        "backfill.gmail handler not yet implemented - job completing without action"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::run_migrations;
    use crate::queue::JobQueue;
    use crate::Database;
    use serde_json::json;
    use tempfile::TempDir;

    async fn setup_queue() -> (Database, JobQueue, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        let db_name = format!("db_{}.sqlite", uuid::Uuid::new_v4());
        let db_path = dir.path().join(db_name);
        let db = Database::new(db_path.as_path()).await.expect("db");
        run_migrations(&db).await.expect("migrations");
        (db.clone(), JobQueue::new(db), dir)
    }

    #[tokio::test]
    async fn stub_handler_completes_successfully() {
        let (db, queue, _dir) = setup_queue().await;
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": "test-account",
                    "query": "newer_than:7d"
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let dispatcher = JobDispatcher::new(db, reqwest::Client::new());
        let result = handle_backfill_gmail(&dispatcher, job).await;

        assert!(result.is_ok(), "stub handler should complete successfully");
    }

    #[tokio::test]
    async fn invalid_payload_returns_fatal_error() {
        let (db, queue, _dir) = setup_queue().await;
        let job_id = queue
            .enqueue(JOB_TYPE, json!({"invalid": "payload"}), None, 0)
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let dispatcher = JobDispatcher::new(db, reqwest::Client::new());
        let result = handle_backfill_gmail(&dispatcher, job).await;

        match result {
            Err(JobError::Fatal(msg)) => {
                assert!(msg.contains("invalid backfill.gmail payload"));
            }
            other => panic!("expected fatal error, got {other:?}"),
        }
    }
}
