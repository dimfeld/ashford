use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::info;

use crate::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
use crate::decisions::{ActionRepository, ActionStatus};
use crate::gmail::{
    EmailAddress, MimeAttachment, MimeBuildError, MimeMessage, dedup_message_ids,
    normalize_message_id,
};
use crate::messages::{MessageError, MessageRepository};
use crate::threads::{ThreadError, ThreadRepository};
use crate::{Job, JobError};

use super::action_gmail::create_gmail_client_with_account;
use super::{JobDispatcher, map_action_error, map_gmail_error};

pub const JOB_TYPE: &str = "outbound.send";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OutboundMessageType {
    Forward,
    Reply,
}

#[derive(Debug, Deserialize)]
struct OutboundAttachmentPayload {
    filename: String,
    #[serde(default)]
    content_type: Option<String>,
    data_base64: String,
}

#[derive(Debug, Deserialize)]
struct OutboundSendPayload {
    account_id: String,
    action_id: String,
    message_type: OutboundMessageType,
    to: Vec<String>,
    #[serde(default)]
    cc: Vec<String>,
    #[serde(default)]
    bcc: Vec<String>,
    subject: Option<String>,
    body_plain: Option<String>,
    body_html: Option<String>,
    original_message_id: String,
    thread_id: Option<String>,
    #[serde(default)]
    references: Vec<String>,
    #[serde(default)]
    attachments: Vec<OutboundAttachmentPayload>,
}

#[derive(Debug, Clone)]
struct SentMetadata {
    message_id: String,
    thread_id: Option<String>,
}

pub async fn handle_outbound_send(dispatcher: &JobDispatcher, job: Job) -> Result<(), JobError> {
    let payload: OutboundSendPayload = serde_json::from_value(job.payload.clone())
        .map_err(|err| JobError::Fatal(format!("invalid outbound.send payload: {err}")))?;

    let action_repo = ActionRepository::new(dispatcher.db.clone());
    let action = action_repo
        .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &payload.action_id)
        .await
        .map_err(|err| map_action_error("load action", err))?;

    match action.status {
        ActionStatus::Completed
        | ActionStatus::Failed
        | ActionStatus::Canceled
        | ActionStatus::Rejected => {
            info!(
                account_id = %payload.account_id,
                action_id = %payload.action_id,
                status = ?action.status,
                "outbound action already terminal, skipping"
            );
            return Ok(());
        }
        ActionStatus::ApprovedPending => {
            info!(
                account_id = %payload.account_id,
                action_id = %payload.action_id,
                "outbound action awaiting approval, skipping"
            );
            return Ok(());
        }
        ActionStatus::Queued => {
            action_repo
                .mark_executing(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action.id)
                .await
                .map_err(|err| {
                    JobError::retryable(format!("failed to mark outbound action executing: {err}"))
                })?;
        }
        ActionStatus::Executing => {}
    }

    let existing_sent = sent_metadata_from_hint(&action.undo_hint_json);

    let result = async {
        if action.account_id != payload.account_id {
            return Err(JobError::Fatal(format!(
                "action {} does not belong to account {}",
                payload.action_id, payload.account_id
            )));
        }

        let expected_action_type = expected_action_type(&payload.message_type);
        if action.action_type != expected_action_type {
            return Err(JobError::Fatal(format!(
                "outbound.send payload expects action type '{expected_action_type}' but action {} is '{}'",
                action.id, action.action_type
            )));
        }

        if let Some(sent) = existing_sent.clone() {
            let undo_hint = build_send_undo_hint(expected_action_type, &sent);
            if action.undo_hint_json != undo_hint {
                persist_send_hint(&action_repo, &action.id, undo_hint.clone(), true).await?;
            }
            return Ok((sent, undo_hint));
        }

        let message_repo = MessageRepository::new(dispatcher.db.clone());
        let message = message_repo
            .get_by_id(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                &payload.original_message_id,
            )
            .await
            .map_err(|err| map_message_error("load message for outbound send", err))?;

        if message.account_id != payload.account_id {
            return Err(JobError::Fatal(format!(
                "message {} does not belong to account {}",
                payload.original_message_id, payload.account_id
            )));
        }

        let (account, gmail_client) =
            create_gmail_client_with_account(dispatcher, &payload.account_id).await?;

        let (in_reply_to, mut references) = build_thread_headers(&payload, &message.headers);
        if matches!(payload.message_type, OutboundMessageType::Reply) {
            references.extend(payload.references.clone());
        }
        let references = dedup_message_ids(references);

        let provider_thread_id = match payload.message_type {
            OutboundMessageType::Forward => None,
            OutboundMessageType::Reply => match payload.thread_id.clone() {
                Some(id) => Some(id),
                None => {
                    let thread_repo = ThreadRepository::new(dispatcher.db.clone());
                    match thread_repo
                        .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message.thread_id)
                        .await
                    {
                        Ok(thread) => Some(thread.provider_thread_id),
                        Err(err) => {
                            return Err(map_thread_error("load thread for outbound send", err));
                        }
                    }
                }
            },
        };

        let attachments = decode_attachments(&payload.attachments)?;
        let mime_message = MimeMessage {
            from: EmailAddress::new(account.display_name.clone(), account.email.clone()),
            to: payload.to.iter().cloned().map(EmailAddress::from).collect(),
            cc: payload.cc.iter().cloned().map(EmailAddress::from).collect(),
            bcc: payload
                .bcc
                .iter()
                .cloned()
                .map(EmailAddress::from)
                .collect(),
            subject: payload.subject.clone().or_else(|| message.subject.clone()),
            body_plain: payload.body_plain.clone(),
            body_html: payload.body_html.clone(),
            in_reply_to,
            references,
            attachments,
        };

        let raw_message = mime_message
            .to_base64_url()
            .map_err(|err| map_mime_error("build MIME message", err))?;

        let response = gmail_client
            .send_message(raw_message, provider_thread_id)
            .await
            .map_err(|err| map_gmail_error("send message", err))?;

        let sent = SentMetadata {
            message_id: response.id.clone(),
            thread_id: Some(response.thread_id.clone()),
        };
        let undo_hint = build_send_undo_hint(expected_action_type, &sent);

        persist_send_hint(&action_repo, &action.id, undo_hint.clone(), false).await?;

        Ok((sent, undo_hint))
    }
    .await;

    match result {
        Ok((sent_metadata, undo_hint)) => {
            action_repo
                .mark_completed_with_undo_hint(
                    DEFAULT_ORG_ID,
                    DEFAULT_USER_ID,
                    &action.id,
                    undo_hint,
                )
                .await
                .map_err(|err| {
                    JobError::retryable(format!("failed to mark outbound action completed: {err}"))
                })?;

            info!(
                account_id = %payload.account_id,
                action_id = %payload.action_id,
                sent_message_id = %sent_metadata.message_id,
                "sent outbound message"
            );

            Ok(())
        }
        Err(job_error) => {
            if !job_error.is_retryable() || job.attempts >= job.max_attempts {
                let _ = action_repo
                    .mark_failed(
                        DEFAULT_ORG_ID,
                        DEFAULT_USER_ID,
                        &action.id,
                        job_error.to_string(),
                    )
                    .await;
            }
            Err(job_error)
        }
    }
}

fn map_mime_error(context: &str, err: MimeBuildError) -> JobError {
    match err {
        MimeBuildError::MissingRecipients | MimeBuildError::MissingBody => {
            JobError::Fatal(format!("{context}: {err}"))
        }
        MimeBuildError::Io(err) => JobError::Fatal(format!("{context}: {err}")),
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

fn map_thread_error(context: &str, err: ThreadError) -> JobError {
    match err {
        ThreadError::NotFound(id) => JobError::Fatal(format!("{context}: thread not found {id}")),
        ThreadError::Database(err) => JobError::retryable(format!("{context}: db error {err}")),
        ThreadError::Sql(err) => JobError::retryable(format!("{context}: db error {err}")),
        ThreadError::Json(err) => JobError::Fatal(format!("{context}: decode error {err}")),
        ThreadError::DateTimeParse(err) => {
            JobError::Fatal(format!("{context}: decode error {err}"))
        }
    }
}

fn expected_action_type(message_type: &OutboundMessageType) -> &'static str {
    match message_type {
        OutboundMessageType::Forward => "forward",
        OutboundMessageType::Reply => "auto_reply",
    }
}

fn build_send_undo_hint(action_type: &str, sent: &SentMetadata) -> Value {
    json!({
        "action": action_type,
        "inverse_action": "none",
        "inverse_parameters": {"note": "cannot undo outbound send"},
        "irreversible": true,
        "sent_message_id": sent.message_id,
        "sent_thread_id": sent.thread_id
    })
}

fn sent_metadata_from_hint(hint: &Value) -> Option<SentMetadata> {
    let message_id = hint
        .get("sent_message_id")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string())?;
    let thread_id = hint
        .get("sent_thread_id")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string());

    Some(SentMetadata {
        message_id,
        thread_id,
    })
}

async fn persist_send_hint(
    action_repo: &ActionRepository,
    action_id: &str,
    undo_hint: Value,
    retryable_on_failure: bool,
) -> Result<(), JobError> {
    action_repo
        .update_undo_hint(DEFAULT_ORG_ID, DEFAULT_USER_ID, action_id, undo_hint)
        .await
        .map(|_| ())
        .map_err(|err| {
            if retryable_on_failure {
                map_action_error("persist outbound send result", err)
            } else {
                JobError::Fatal(format!("persist outbound send result: {err}"))
            }
        })
}

fn build_thread_headers(
    payload: &OutboundSendPayload,
    headers: &[crate::gmail::types::Header],
) -> (Option<String>, Vec<String>) {
    match payload.message_type {
        OutboundMessageType::Reply => {
            let message_id = header_value(headers, "Message-ID");

            let mut references = Vec::new();
            if let Some(refs) = header_value(headers, "References") {
                references.extend(split_references(&refs));
            }

            if let Some(id) = message_id.clone() {
                references.push(id);
            }

            (message_id, dedup_message_ids(references))
        }
        OutboundMessageType::Forward => (None, Vec::new()),
    }
}

fn header_value(headers: &[crate::gmail::types::Header], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case(name))
        .map(|h| h.value.clone())
}

fn split_references(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .filter_map(|part| normalize_message_id(part))
        .collect()
}

fn decode_attachments(
    payloads: &[OutboundAttachmentPayload],
) -> Result<Vec<MimeAttachment>, JobError> {
    let mut attachments = Vec::new();
    for attachment in payloads {
        let data = URL_SAFE_NO_PAD
            .decode(attachment.data_base64.as_bytes())
            .or_else(|_| STANDARD.decode(attachment.data_base64.as_bytes()))
            .map_err(|err| {
                JobError::Fatal(format!(
                    "invalid attachment data for {}: {err}",
                    attachment.filename
                ))
            })?;

        attachments.push(MimeAttachment {
            filename: attachment.filename.clone(),
            content_type: attachment
                .content_type
                .clone()
                .unwrap_or_else(|| "application/octet-stream".to_string()),
            data,
        });
    }
    Ok(attachments)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::{AccountConfig, AccountRepository, PubsubConfig};
    use crate::decisions::{ActionStatus, NewAction};
    use crate::gmail::OAuthTokens;
    use crate::messages::{Mailbox, NewMessage};
    use crate::migrations::run_migrations;
    use crate::queue::JobQueue;
    use crate::threads::ThreadRepository;
    use chrono::Utc;
    use serde_json::json;
    use tempfile::TempDir;
    use uuid::Uuid;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn setup_db() -> (crate::Database, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        let db_name = format!("db_{}.sqlite", Uuid::new_v4());
        let db_path = dir.path().join(db_name);
        let db = crate::Database::new(&db_path).await.expect("create db");
        run_migrations(&db).await.expect("migrations");
        (db, dir)
    }

    async fn setup_account(db: &crate::Database) -> String {
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

        account.id
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
                "gmail-thread-1",
                Some("Original subject".to_string()),
                Some("Snippet".to_string()),
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
                thread_id: thread.id.clone(),
                provider_message_id: provider_message_id.to_string(),
                from_email: Some("sender@external.com".into()),
                from_name: Some("External Sender".into()),
                to: vec![Mailbox {
                    email: "user@example.com".into(),
                    name: Some("User".into()),
                }],
                cc: vec![],
                bcc: vec![],
                subject: Some("Original subject".into()),
                snippet: Some("Snippet".into()),
                received_at: Some(Utc::now()),
                internal_date: Some(Utc::now()),
                labels: vec!["INBOX".into()],
                headers: vec![
                    crate::gmail::types::Header {
                        name: "Message-ID".into(),
                        value: "<orig@id>".into(),
                    },
                    crate::gmail::types::Header {
                        name: "References".into(),
                        value: "<ref1@id>".into(),
                    },
                ],
                body_plain: Some("Hello".into()),
                body_html: Some("<p>Hello</p>".into()),
                raw_json: json!({}),
            })
            .await
            .expect("create message");

        message.id
    }

    async fn setup_action(
        db: &crate::Database,
        account_id: &str,
        message_id: &str,
        action_type: &str,
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
                parameters_json: json!({}),
                status: ActionStatus::Executing,
                error_message: None,
                executed_at: None,
                undo_hint_json: json!({}),
                trace_id: None,
            })
            .await
            .expect("create action");

        action.id
    }

    fn dispatcher(db: crate::Database, api_base: String) -> JobDispatcher {
        JobDispatcher::new(
            db,
            reqwest::Client::new(),
            std::sync::Arc::new(crate::llm::MockLLMClient::new()),
            crate::config::PolicyConfig::default(),
        )
        .with_gmail_api_base(api_base)
    }

    #[tokio::test]
    async fn sends_reply_and_marks_action_completed() {
        let (db, _dir) = setup_db().await;
        let account_id = setup_account(&db).await;
        let message_id = setup_message(&db, &account_id, "provider-123").await;
        let action_id = setup_action(&db, &account_id, &message_id, "auto_reply").await;

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "action_id": action_id,
                    "message_type": "reply",
                    "to": ["someone@example.com"],
                    "cc": [],
                    "bcc": [],
                    "subject": "Re: Original subject",
                    "body_plain": "Auto-reply body",
                    "body_html": "<p>Auto-reply body</p>",
                    "original_message_id": message_id,
                    "thread_id": "gmail-thread-1"
                }),
                None,
                0,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());

        Mock::given(method("POST"))
            .and(path("/gmail/v1/users/user@example.com/messages/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "sent-123",
                "threadId": "gmail-thread-1",
                "labelIds": ["SENT"]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let dispatcher = dispatcher(db.clone(), api_base);
        handle_outbound_send(&dispatcher, job)
            .await
            .expect("outbound send succeeds");

        // Verify the request contained the Gmail thread ID and encoded MIME.
        let requests = server.received_requests().await.expect("requests");
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).expect("json body");
        let raw = body["raw"].as_str().expect("raw exists");
        let decoded = URL_SAFE_NO_PAD
            .decode(raw.as_bytes())
            .expect("decode base64");
        let raw_message = String::from_utf8(decoded).expect("utf8");
        assert!(
            raw_message.contains("someone@example.com"),
            "raw message should include recipient, got: {raw_message}"
        );
        assert!(raw_message.contains("In-Reply-To: <orig@id>"));
        assert!(
            raw_message.contains("<ref1@id>"),
            "raw message should include references, got: {raw_message}"
        );
        assert_eq!(body["threadId"], "gmail-thread-1");

        let repo = ActionRepository::new(db.clone());
        let action = repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
            .await
            .expect("load action");
        assert!(matches!(action.status, ActionStatus::Completed));
        assert_eq!(
            action
                .undo_hint_json
                .get("irreversible")
                .and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[tokio::test]
    async fn uses_provider_thread_id_when_payload_missing() {
        let (db, _dir) = setup_db().await;
        let account_id = setup_account(&db).await;
        let message_id = setup_message(&db, &account_id, "provider-456").await;
        let action_id = setup_action(&db, &account_id, &message_id, "auto_reply").await;

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "action_id": action_id,
                    "message_type": "reply",
                    "to": ["someone@example.com"],
                    "cc": [],
                    "bcc": [],
                    "subject": "Re: Original subject",
                    "body_plain": "Auto-reply body",
                    "body_html": "<p>Auto-reply body</p>",
                    "original_message_id": message_id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());

        Mock::given(method("POST"))
            .and(path("/gmail/v1/users/user@example.com/messages/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "sent-789",
                "threadId": "gmail-thread-1",
                "labelIds": ["SENT"]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let dispatcher = dispatcher(db.clone(), api_base);
        handle_outbound_send(&dispatcher, job)
            .await
            .expect("outbound send succeeds");

        let requests = server.received_requests().await.expect("requests");
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).expect("json");
        assert_eq!(body["threadId"], "gmail-thread-1");

        let raw = body["raw"].as_str().unwrap();
        let decoded = URL_SAFE_NO_PAD.decode(raw).unwrap();
        let raw_message = String::from_utf8(decoded).unwrap();
        assert!(raw_message.contains("In-Reply-To: <orig@id>"));
    }

    #[tokio::test]
    async fn forward_decodes_standard_base64_attachment_and_omits_in_reply_to() {
        let (db, _dir) = setup_db().await;
        let account_id = setup_account(&db).await;
        let message_id = setup_message(&db, &account_id, "provider-789").await;
        let action_id = setup_action(&db, &account_id, &message_id, "forward").await;

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "action_id": action_id,
                    "message_type": "forward",
                    "to": ["fwd@example.com"],
                    "cc": [],
                    "bcc": [],
                    "subject": "Fwd: Original subject",
                    "body_plain": "Forwarding",
                    "body_html": null,
                    "original_message_id": message_id,
                    "attachments": [{
                        "filename": "doc.txt",
                        "data_base64": "YWJjLw=="
                    }],
                    "references": ["<ref1@id>", "<ref2@id>"]
                }),
                None,
                0,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());

        Mock::given(method("POST"))
            .and(path("/gmail/v1/users/user@example.com/messages/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "sent-fwd",
                "threadId": "new-thread",
                "labelIds": ["SENT"]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let dispatcher = dispatcher(db.clone(), api_base);
        handle_outbound_send(&dispatcher, job)
            .await
            .expect("outbound send succeeds");

        let requests = server.received_requests().await.expect("requests");
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).expect("json body");
        assert!(
            !body
                .as_object()
                .expect("body object")
                .contains_key("threadId"),
            "forward should not include threadId in Gmail request: {body:?}"
        );
        let raw = body["raw"].as_str().unwrap();
        let decoded = URL_SAFE_NO_PAD.decode(raw).unwrap();
        let raw_message = String::from_utf8(decoded).unwrap();

        assert!(
            !raw_message.contains("In-Reply-To"),
            "forward should not set In-Reply-To, got: {raw_message}"
        );
        assert!(
            !raw_message.contains("References:"),
            "forward should not include References header, got: {raw_message}"
        );
        assert!(
            raw_message.contains("doc.txt"),
            "attachment filename should be present"
        );
        assert!(
            raw_message.contains("YWJjLw=="),
            "attachment content should be base64 encoded"
        );
    }

    #[tokio::test]
    async fn uses_existing_sent_metadata_instead_of_resending() {
        let (db, _dir) = setup_db().await;
        let account_id = setup_account(&db).await;
        let message_id = setup_message(&db, &account_id, "provider-999").await;
        let action_id = setup_action(&db, &account_id, &message_id, "auto_reply").await;

        let action_repo = ActionRepository::new(db.clone());
        let stored_hint = json!({
            "action": "auto_reply",
            "inverse_action": "none",
            "inverse_parameters": {"note": "cannot undo outbound send"},
            "irreversible": true,
            "sent_message_id": "already-sent",
            "sent_thread_id": "thread-abc"
        });
        action_repo
            .update_undo_hint(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                &action_id,
                stored_hint.clone(),
            )
            .await
            .expect("store undo hint");

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "action_id": action_id,
                    "message_type": "reply",
                    "to": ["someone@example.com"],
                    "cc": [],
                    "bcc": [],
                    "subject": "Re: Original subject",
                    "body_plain": "Auto-reply body",
                    "body_html": "<p>Auto-reply body</p>",
                    "original_message_id": message_id,
                    "thread_id": "gmail-thread-1"
                }),
                None,
                0,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());

        Mock::given(method("POST"))
            .and(path("/gmail/v1/users/user@example.com/messages/send"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;

        let dispatcher = dispatcher(db.clone(), api_base);
        handle_outbound_send(&dispatcher, job)
            .await
            .expect("outbound send succeeds");

        let action = action_repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
            .await
            .expect("action");
        assert!(matches!(action.status, ActionStatus::Completed));
        assert_eq!(action.undo_hint_json, stored_hint);
    }

    #[tokio::test]
    async fn uses_existing_sent_metadata_even_if_message_missing() {
        let (db, _dir) = setup_db().await;
        let account_id = setup_account(&db).await;
        let message_id = setup_message(&db, &account_id, "provider-missing").await;
        let action_id = setup_action(&db, &account_id, &message_id, "auto_reply").await;

        let action_repo = ActionRepository::new(db.clone());
        let stored_hint = json!({
            "action": "auto_reply",
            "inverse_action": "none",
            "inverse_parameters": {"note": "cannot undo outbound send"},
            "irreversible": true,
            "sent_message_id": "already-sent",
            "sent_thread_id": "thread-abc"
        });
        action_repo
            .update_undo_hint(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                &action_id,
                stored_hint.clone(),
            )
            .await
            .expect("store undo hint");

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "action_id": action_id,
                    "message_type": "reply",
                    "to": ["someone@example.com"],
                    "cc": [],
                    "bcc": [],
                    "subject": "Re: Original subject",
                    "body_plain": "Auto-reply body",
                    "body_html": "<p>Auto-reply body</p>",
                    "original_message_id": "missing-message-id",
                    "thread_id": "gmail-thread-1"
                }),
                None,
                0,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());

        Mock::given(method("POST"))
            .and(path("/gmail/v1/users/user@example.com/messages/send"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;

        let dispatcher = dispatcher(db.clone(), api_base);
        handle_outbound_send(&dispatcher, job)
            .await
            .expect("outbound send succeeds");

        let action = action_repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
            .await
            .expect("action");
        assert!(matches!(action.status, ActionStatus::Completed));
        assert_eq!(action.undo_hint_json, stored_hint);

        let received = server.received_requests().await.expect("requests");
        assert!(
            received.is_empty(),
            "no Gmail requests should be sent when sent metadata exists even if message missing"
        );
    }

    #[tokio::test]
    async fn invalid_attachment_returns_fatal_and_marks_action_failed() {
        let (db, _dir) = setup_db().await;
        let account_id = setup_account(&db).await;
        let message_id = setup_message(&db, &account_id, "provider-000").await;
        let action_id = setup_action(&db, &account_id, &message_id, "auto_reply").await;

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "action_id": action_id,
                    "message_type": "reply",
                    "to": ["bad@example.com"],
                    "cc": [],
                    "bcc": [],
                    "subject": "Re: Original subject",
                    "body_plain": "Body",
                    "body_html": null,
                    "original_message_id": message_id,
                    "attachments": [{
                        "filename": "bad.txt",
                        "data_base64": "not-base64!"
                    }]
                }),
                None,
                0,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let dispatcher = dispatcher(db.clone(), "https://gmail.invalid".to_string());
        let err = handle_outbound_send(&dispatcher, job)
            .await
            .expect_err("should fail on invalid attachment");

        match err {
            JobError::Fatal(msg) => assert!(msg.contains("invalid attachment data")),
            other => panic!("expected fatal error, got {other:?}"),
        }

        let repo = ActionRepository::new(db.clone());
        let action = repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
            .await
            .expect("action");
        assert!(matches!(action.status, ActionStatus::Failed));
    }

    #[tokio::test]
    async fn fails_when_action_type_mismatch_without_sending() {
        let (db, _dir) = setup_db().await;
        let account_id = setup_account(&db).await;
        let message_id = setup_message(&db, &account_id, "provider-mismatch").await;
        let action_id = setup_action(&db, &account_id, &message_id, "forward").await;

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "action_id": action_id,
                    "message_type": "reply",
                    "to": ["recipient@example.com"],
                    "cc": [],
                    "bcc": [],
                    "subject": "Re: Original subject",
                    "body_plain": "Auto-reply body",
                    "body_html": null,
                    "original_message_id": message_id,
                    "thread_id": "gmail-thread-1"
                }),
                None,
                0,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());

        Mock::given(method("POST"))
            .and(path("/gmail/v1/users/user@example.com/messages/send"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;

        let dispatcher = dispatcher(db.clone(), api_base);
        let err = handle_outbound_send(&dispatcher, job)
            .await
            .expect_err("mismatched action type should fail");

        match err {
            JobError::Fatal(msg) => {
                assert!(
                    msg.contains("expects action type 'auto_reply'"),
                    "unexpected error message: {msg}"
                )
            }
            other => panic!("expected fatal error, got {other:?}"),
        }

        let repo = ActionRepository::new(db.clone());
        let action = repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
            .await
            .expect("action");
        assert!(matches!(action.status, ActionStatus::Failed));

        let received = server.received_requests().await.expect("requests");
        assert!(
            received.is_empty(),
            "no Gmail requests should be sent on mismatch, got {received:?}"
        );
    }
}
