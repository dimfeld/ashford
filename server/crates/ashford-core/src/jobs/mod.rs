use async_trait::async_trait;
use reqwest::StatusCode;

use crate::accounts::AccountError;
use crate::gmail::GmailClientError;
use crate::gmail::oauth::OAuthError;
use crate::worker::{JobError, JobExecutor};
use crate::{Database, Job, JobContext};

mod backfill_gmail;
mod history_sync_gmail;
mod ingest_gmail;

use backfill_gmail::handle_backfill_gmail;
use history_sync_gmail::handle_history_sync_gmail;
use ingest_gmail::handle_ingest_gmail;

pub const JOB_TYPE_BACKFILL_GMAIL: &str = backfill_gmail::JOB_TYPE;
pub const JOB_TYPE_INGEST_GMAIL: &str = "ingest.gmail";
pub const JOB_TYPE_HISTORY_SYNC_GMAIL: &str = "history.sync.gmail";

#[derive(Clone)]
pub struct JobDispatcher {
    pub db: Database,
    pub http: reqwest::Client,
    pub gmail_api_base: Option<String>,
}

impl JobDispatcher {
    pub fn new(db: Database, http: reqwest::Client) -> Self {
        Self {
            db,
            http,
            gmail_api_base: None,
        }
    }

    pub fn with_gmail_api_base(mut self, base: impl Into<String>) -> Self {
        self.gmail_api_base = Some(base.into());
        self
    }
}

#[async_trait]
impl JobExecutor for JobDispatcher {
    async fn execute(&self, job: Job, _ctx: JobContext) -> Result<(), JobError> {
        match job.job_type.as_str() {
            JOB_TYPE_BACKFILL_GMAIL => handle_backfill_gmail(self, job).await,
            JOB_TYPE_INGEST_GMAIL => handle_ingest_gmail(self, job).await,
            JOB_TYPE_HISTORY_SYNC_GMAIL => handle_history_sync_gmail(self, job).await,
            other => Err(JobError::Fatal(format!("unknown job type: {other}"))),
        }
    }
}

pub(crate) fn map_gmail_error(context: &str, err: GmailClientError) -> JobError {
    match err {
        GmailClientError::Unauthorized => JobError::Retryable(format!("{context}: unauthorized")),
        GmailClientError::Http(ref http_err) => {
            if let Some(status) = http_err.status() {
                match status {
                    StatusCode::NOT_FOUND => {
                        JobError::Fatal(format!("{context}: resource not found (404)"))
                    }
                    StatusCode::TOO_MANY_REQUESTS | StatusCode::FORBIDDEN => {
                        // 429 is explicit rate limit; 403 often indicates userRateLimitExceeded
                        JobError::Retryable(format!("{context}: rate limited ({status})"))
                    }
                    StatusCode::UNAUTHORIZED => {
                        JobError::Retryable(format!("{context}: unauthorized (401)"))
                    }
                    status if status.is_server_error() => {
                        JobError::Retryable(format!("{context}: server error {status}"))
                    }
                    status => JobError::Fatal(format!("{context}: http status {status}")),
                }
            } else {
                JobError::Retryable(format!("{context}: network error {http_err}"))
            }
        }
        GmailClientError::OAuth(err) => {
            JobError::Retryable(format!("{context}: oauth error {err}"))
        }
        GmailClientError::TokenStore(err) => JobError::Fatal(format!("{context}: {err}")),
        GmailClientError::Decode(err) => JobError::Fatal(format!("{context}: {err}")),
    }
}

pub(crate) fn map_account_error(context: &str, err: AccountError) -> JobError {
    match err {
        AccountError::NotFound(id) => JobError::Fatal(format!("{context}: account not found {id}")),
        AccountError::OAuth(OAuthError::MissingRefreshToken) => {
            JobError::Fatal(format!("{context}: missing refresh token for account"))
        }
        AccountError::OAuth(err) => JobError::Retryable(format!("{context}: oauth error {err}")),
        AccountError::Conflict(_) => JobError::Retryable(format!("{context}: optimistic conflict")),
        AccountError::Database(err) => JobError::Retryable(format!("{context}: db error {err}")),
        AccountError::Sql(err) => JobError::Retryable(format!("{context}: db error {err}")),
        AccountError::Json(err) => JobError::Fatal(format!("{context}: decode error {err}")),
        AccountError::DateTimeParse(err) => {
            JobError::Fatal(format!("{context}: decode error {err}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::run_migrations;
    use crate::queue::JobQueue;
    use serde_json::json;
    use tempfile::TempDir;

    async fn setup_queue() -> (Database, JobQueue, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        // Use a unique database filename to avoid any potential conflicts
        let db_name = format!("db_{}.sqlite", uuid::Uuid::new_v4());
        let db_path = dir.path().join(db_name);
        let db = Database::new(db_path.as_path()).await.expect("db");
        run_migrations(&db).await.expect("migrations");
        (db.clone(), JobQueue::new(db), dir)
    }

    #[tokio::test]
    async fn unknown_job_type_is_fatal() {
        let (db, queue, _dir) = setup_queue().await;
        let job_id = queue
            .enqueue("unknown.job", json!({}), None, 0)
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let dispatcher = JobDispatcher::new(db, reqwest::Client::new());
        let ctx = JobContext::new(queue.clone(), job.clone());
        let result = dispatcher.execute(job, ctx).await;

        match result {
            Err(JobError::Fatal(msg)) => assert!(msg.contains("unknown job type")),
            other => panic!("expected fatal error, got {other:?}"),
        }
    }

    // Note: 403 is tested via history_sync_gmail::tests::history_sync_retries_on_403_rate_limit
    // which uses a mock server to return actual 403 responses. Testing map_gmail_error directly
    // is difficult because reqwest::Error cannot be easily constructed with a status code.
}
