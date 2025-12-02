use std::sync::Arc;

use async_trait::async_trait;
use reqwest::StatusCode;

use crate::accounts::AccountError;
use crate::config::PolicyConfig;
use crate::decisions::ActionError;
use crate::gmail::GmailClientError;
use crate::gmail::oauth::OAuthError;
use crate::llm::{LLMClient, LLMError};
use crate::rules::ExecutorError;
use crate::worker::{JobError, JobExecutor};
use crate::{Database, Job, JobContext};

mod backfill_gmail;
mod action_gmail;
mod approval_notify;
mod classify;
mod history_sync_gmail;
mod ingest_gmail;

use backfill_gmail::handle_backfill_gmail;
use action_gmail::handle_action_gmail;
use approval_notify::handle_approval_notify;
use classify::handle_classify;
use history_sync_gmail::handle_history_sync_gmail;
use ingest_gmail::handle_ingest_gmail;

pub const JOB_TYPE_ACTION_GMAIL: &str = action_gmail::JOB_TYPE;
pub const JOB_TYPE_APPROVAL_NOTIFY: &str = approval_notify::JOB_TYPE;
pub const JOB_TYPE_BACKFILL_GMAIL: &str = backfill_gmail::JOB_TYPE;
pub const JOB_TYPE_CLASSIFY: &str = "classify";
pub const JOB_TYPE_INGEST_GMAIL: &str = "ingest.gmail";
pub const JOB_TYPE_HISTORY_SYNC_GMAIL: &str = "history.sync.gmail";

#[derive(Clone)]
pub struct JobDispatcher {
    pub db: Database,
    pub http: reqwest::Client,
    pub gmail_api_base: Option<String>,
    pub llm_client: Arc<dyn LLMClient>,
    pub policy_config: PolicyConfig,
}

impl JobDispatcher {
    pub fn new(
        db: Database,
        http: reqwest::Client,
        llm_client: Arc<dyn LLMClient>,
        policy_config: PolicyConfig,
    ) -> Self {
        Self {
            db,
            http,
            gmail_api_base: None,
            llm_client,
            policy_config,
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
            JOB_TYPE_ACTION_GMAIL => handle_action_gmail(self, job).await,
            JOB_TYPE_APPROVAL_NOTIFY => handle_approval_notify(self, job).await,
            JOB_TYPE_CLASSIFY => handle_classify(self, job).await,
            JOB_TYPE_INGEST_GMAIL => handle_ingest_gmail(self, job).await,
            JOB_TYPE_HISTORY_SYNC_GMAIL => handle_history_sync_gmail(self, job).await,
            other => Err(JobError::Fatal(format!("unknown job type: {other}"))),
        }
    }
}

pub(crate) fn map_gmail_error(context: &str, err: GmailClientError) -> JobError {
    match err {
        GmailClientError::Unauthorized => JobError::retryable(format!("{context}: unauthorized")),
        GmailClientError::Http(ref http_err) => {
            if let Some(status) = http_err.status() {
                match status {
                    StatusCode::NOT_FOUND => {
                        JobError::Fatal(format!("{context}: resource not found (404)"))
                    }
                    StatusCode::TOO_MANY_REQUESTS | StatusCode::FORBIDDEN => {
                        // 429 is explicit rate limit; 403 often indicates userRateLimitExceeded
                        JobError::retryable(format!("{context}: rate limited ({status})"))
                    }
                    StatusCode::UNAUTHORIZED => {
                        JobError::retryable(format!("{context}: unauthorized (401)"))
                    }
                    status if status.is_server_error() => {
                        JobError::retryable(format!("{context}: server error {status}"))
                    }
                    status => JobError::Fatal(format!("{context}: http status {status}")),
                }
            } else {
                JobError::retryable(format!("{context}: network error {http_err}"))
            }
        }
        GmailClientError::OAuth(err) => {
            JobError::retryable(format!("{context}: oauth error {err}"))
        }
        GmailClientError::TokenStore(err) => JobError::Fatal(format!("{context}: {err}")),
        GmailClientError::Decode(err) => JobError::Fatal(format!("{context}: {err}")),
    }
}

#[allow(dead_code)]
pub(crate) fn map_llm_error(context: &str, err: LLMError) -> JobError {
    match err {
        LLMError::RateLimited(info) => {
            let detail = info
                .retry_after_ms
                .map(|ms| format!(" (retry after {ms}ms)"))
                .unwrap_or_default();
            let message = format!("{context}: rate limited{detail}");
            if let Some(ms) = info.retry_after_ms {
                JobError::retryable_after(message, std::time::Duration::from_millis(ms))
            } else {
                JobError::retryable(message)
            }
        }
        LLMError::AuthenticationFailed => {
            JobError::Fatal(format!("{context}: authentication failed"))
        }
        LLMError::InvalidRequest(msg) => {
            JobError::Fatal(format!("{context}: invalid request {msg}"))
        }
        LLMError::ServerError(msg) => JobError::retryable(format!("{context}: server error {msg}")),
        LLMError::Timeout => JobError::retryable(format!("{context}: timeout")),
        LLMError::ParseError(msg) => JobError::Fatal(format!("{context}: parse error {msg}")),
        LLMError::ProviderError(msg) => {
            JobError::retryable(format!("{context}: provider error {msg}"))
        }
    }
}

pub(crate) fn map_account_error(context: &str, err: AccountError) -> JobError {
    match err {
        AccountError::NotFound(id) => JobError::Fatal(format!("{context}: account not found {id}")),
        AccountError::OAuth(OAuthError::MissingRefreshToken) => {
            JobError::Fatal(format!("{context}: missing refresh token for account"))
        }
        AccountError::OAuth(err) => JobError::retryable(format!("{context}: oauth error {err}")),
        AccountError::Conflict(_) => JobError::retryable(format!("{context}: optimistic conflict")),
        AccountError::Database(err) => JobError::retryable(format!("{context}: db error {err}")),
        AccountError::Sql(err) => JobError::retryable(format!("{context}: db error {err}")),
        AccountError::Json(err) => JobError::Fatal(format!("{context}: decode error {err}")),
        AccountError::DateTimeParse(err) => {
            JobError::Fatal(format!("{context}: decode error {err}"))
        }
    }
}

pub(crate) fn map_action_error(context: &str, err: ActionError) -> JobError {
    match err {
        ActionError::NotFound(id) => JobError::Fatal(format!("{context}: action not found {id}")),
        ActionError::InvalidStatus(status) => {
            JobError::Fatal(format!("{context}: invalid status {status}"))
        }
        ActionError::InvalidInitialStatus(status) => JobError::Fatal(format!(
            "{context}: invalid initial status {status:?}"
        )),
        ActionError::InvalidStatusTransition { from, to } => JobError::Fatal(format!(
            "{context}: invalid status transition {from:?}->{to:?}"
        )),
        ActionError::Json(err) => JobError::Fatal(format!("{context}: decode error {err}")),
        ActionError::DateTimeParse(err) => JobError::Fatal(format!("{context}: decode error {err}")),
        ActionError::Database(err) => JobError::retryable(format!("{context}: db error {err}")),
        ActionError::Sql(err) => JobError::retryable(format!("{context}: db error {err}")),
    }
}

#[allow(dead_code)]
pub(crate) fn map_executor_error(context: &str, err: ExecutorError) -> JobError {
    match err {
        ExecutorError::RuleLoader(loader_err) => {
            // RuleLoaderError wraps DeterministicRuleError which has Database/Sql variants
            JobError::retryable(format!("{context}: rule loading failed {loader_err}"))
        }
        ExecutorError::Condition(condition_err) => {
            // Condition evaluation errors (invalid regex, missing field, etc.) are data issues
            JobError::Fatal(format!(
                "{context}: condition evaluation failed {condition_err}"
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MockLLMClient;
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

    fn test_dispatcher(db: Database) -> JobDispatcher {
        JobDispatcher::new(
            db,
            reqwest::Client::new(),
            Arc::new(MockLLMClient::new()),
            PolicyConfig::default(),
        )
    }

    #[tokio::test]
    async fn unknown_job_type_is_fatal() {
        let (db, queue, _dir) = setup_queue().await;
        let job_id = queue
            .enqueue("unknown.job", json!({}), None, 0)
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let dispatcher = test_dispatcher(db);
        let ctx = JobContext::new(queue.clone(), job.clone());
        let result = dispatcher.execute(job, ctx).await;

        match result {
            Err(JobError::Fatal(msg)) => assert!(msg.contains("unknown job type")),
            other => panic!("expected fatal error, got {other:?}"),
        }
    }

    #[test]
    fn map_llm_error_marks_retryable_cases() {
        let context = "llm call";
        let retryable = vec![
            (
                LLMError::RateLimited(crate::llm::error::RateLimitInfo::new(Some(1500))),
                "rate limited (retry after 1500ms)",
                Some(std::time::Duration::from_millis(1500)),
            ),
            (LLMError::ServerError("500".into()), "server error", None),
            (LLMError::Timeout, "timeout", None),
            (
                LLMError::ProviderError("transient".into()),
                "provider error",
                None,
            ),
        ];

        for (err, expected, expected_retry_after) in retryable {
            match map_llm_error(context, err) {
                JobError::Retryable {
                    message,
                    retry_after,
                } => {
                    assert!(
                        message.contains(expected),
                        "expected retryable message to contain {expected}, got {message}"
                    );
                    assert_eq!(retry_after, expected_retry_after);
                }
                other => panic!("expected retryable, got {other:?}"),
            }
        }
    }

    #[test]
    fn map_llm_error_marks_fatal_cases() {
        let context = "llm call";
        let fatal = vec![
            (LLMError::AuthenticationFailed, "authentication failed"),
            (LLMError::InvalidRequest("bad".into()), "invalid request"),
            (LLMError::ParseError("json".into()), "parse error"),
        ];

        for (err, expected) in fatal {
            match map_llm_error(context, err) {
                JobError::Fatal(msg) => assert!(
                    msg.contains(expected),
                    "expected fatal message to contain {expected}, got {msg}"
                ),
                other => panic!("expected fatal, got {other:?}"),
            }
        }
    }

    // Note: 403 is tested via history_sync_gmail::tests::history_sync_retries_on_403_rate_limit
    // which uses a mock server to return actual 403 responses. Testing map_gmail_error directly
    // is difficult because reqwest::Error cannot be easily constructed with a status code.

    #[test]
    fn map_executor_error_rule_loader_is_retryable() {
        use crate::rules::deterministic::{ExecutorError, RuleLoaderError};
        use crate::rules::repositories::DeterministicRuleError;

        let context = "rule evaluation";
        let err = ExecutorError::RuleLoader(RuleLoaderError::Repository(
            DeterministicRuleError::Database(crate::db::DbError::Connect(
                libsql::Error::ConnectionFailed("connection lost".into()),
            )),
        ));

        match map_executor_error(context, err) {
            JobError::Retryable { message, .. } => {
                assert!(
                    message.contains("rule loading failed"),
                    "expected message to contain 'rule loading failed', got {message}"
                );
            }
            other => panic!("expected retryable, got {other:?}"),
        }
    }

    #[test]
    fn map_executor_error_condition_is_fatal() {
        use crate::rules::conditions::ConditionError;
        use crate::rules::deterministic::ExecutorError;

        let context = "rule evaluation";
        let err = ExecutorError::Condition(ConditionError::InvalidRegex {
            pattern: "(".into(),
            source: regex::Regex::new("(").unwrap_err(),
        });

        match map_executor_error(context, err) {
            JobError::Fatal(msg) => {
                assert!(
                    msg.contains("condition evaluation failed"),
                    "expected message to contain 'condition evaluation failed', got {msg}"
                );
            }
            other => panic!("expected fatal, got {other:?}"),
        }
    }
}
