//! Classify job handler for email classification.
//!
//! This module handles the classification of ingested email messages through
//! a two-phase approach:
//! 1. Fast path: Evaluate deterministic rules for immediate matches
//! 2. Slow path: Use LLM to classify messages that don't match deterministic rules

use serde::Deserialize;
use serde_json::json;
use tracing::{debug, info};

use crate::accounts::AccountRepository;
use crate::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
use crate::decisions::{
    ActionRepository, ActionStatus, DecisionRepository, DecisionSource, NewAction, NewDecision,
    SafetyEnforcer, SafetyResult,
};
use crate::llm::decision::{
    ActionType, DecisionDetails, DecisionOutput, Explanations, MessageRef, TelemetryPlaceholder,
    UndoHint,
};
use crate::llm::prompt::{DECISION_TOOL_NAME, PromptBuilder, build_decision_tool};
use crate::llm::types::CompletionRequest;
use crate::messages::{Message, MessageRepository};
use crate::queue::{JobQueue, QueueError};
use crate::rules::conditions::extract_domain;
use crate::rules::deterministic::{RuleExecutor, RuleMatch};
use crate::rules::repositories::{DirectionsRepository, LlmRuleRepository};
use crate::rules::types::{LlmRule, RuleScope, SafeMode};
use crate::{Job, JobError};

use super::{JobDispatcher, JOB_TYPE_ACTION_GMAIL, JOB_TYPE_APPROVAL_NOTIFY, map_account_error, map_executor_error, map_llm_error};

/// Payload for the classify job.
#[derive(Debug, Deserialize)]
pub struct ClassifyPayload {
    /// The account ID for the message.
    pub account_id: String,
    /// The internal message UUID (not provider_message_id).
    pub message_id: String,
}

/// Handle the classify job.
///
/// This function orchestrates the full decision pipeline:
/// 1. Parse payload and load message/account
/// 2. Try deterministic rules (fast path)
/// 3. If no match, use LLM to classify (slow path)
/// 4. Apply safety enforcement
/// 5. Persist decision and action records
pub async fn handle_classify(dispatcher: &JobDispatcher, job: Job) -> Result<(), JobError> {
    let payload: ClassifyPayload = serde_json::from_value(job.payload.clone())
        .map_err(|err| JobError::Fatal(format!("invalid classify payload: {err}")))?;

    // Load message
    let msg_repo = MessageRepository::new(dispatcher.db.clone());
    let message = msg_repo
        .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &payload.message_id)
        .await
        .map_err(|err| match err {
            crate::messages::MessageError::NotFound(_) => {
                JobError::Fatal(format!("message not found: {}", payload.message_id))
            }
            _ => JobError::retryable(format!("failed to load message: {err}")),
        })?;

    if message.account_id != payload.account_id {
        return Err(JobError::Fatal(format!(
            "message {} does not belong to account {}",
            payload.message_id, payload.account_id
        )));
    }

    // Load account
    let account_repo = AccountRepository::new(dispatcher.db.clone());
    let _account = account_repo
        .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &payload.account_id)
        .await
        .map_err(|err| map_account_error("load account", err))?;

    // Try deterministic rules first (fast path)
    let rule_repo =
        crate::rules::repositories::DeterministicRuleRepository::new(dispatcher.db.clone());
    let rule_executor = RuleExecutor::new(rule_repo);

    let rule_match = rule_executor
        .evaluate(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message)
        .await
        .map_err(|err| map_executor_error("evaluate deterministic rules", err))?;

    // Determine if we should skip safety enforcement (for explicit SafeMode overrides)
    let skip_safety_enforcement = rule_match.as_ref().is_some_and(|m| {
        matches!(m.safe_mode, SafeMode::DangerousOverride | SafeMode::AlwaysSafe)
    });

    let (mut decision_output, source) = if let Some(matched) = rule_match {
        // Fast path: deterministic rule matched
        let decision = rule_match_to_decision_output(&message, &matched);
        (decision, DecisionSource::Deterministic)
    } else {
        // Slow path: use LLM
        let decision = run_llm_classification(dispatcher, &message, &payload.account_id).await?;
        (decision, DecisionSource::Llm)
    };

    // Apply safety enforcement unless the deterministic rule has an explicit SafeMode override.
    // DangerousOverride and AlwaysSafe modes indicate the rule author has explicitly
    // configured the safety behavior, so we should respect their choice.
    let safety_result = if skip_safety_enforcement {
        // Use the needs_approval value already set by rule_match_to_decision_output
        SafetyResult {
            overrides_applied: vec![],
            requires_approval: decision_output.decision.needs_approval,
        }
    } else {
        let safety_enforcer = SafetyEnforcer::new(dispatcher.policy_config.clone());
        let result = safety_enforcer.enforce(&decision_output);
        if decision_output.decision.needs_approval != result.requires_approval {
            // Persist the final, safety-adjusted approval flag so decision_json is consistent
            decision_output.decision.needs_approval = result.requires_approval;
        }
        result
    };

    // Persist decision
    let decision_repo = DecisionRepository::new(dispatcher.db.clone());
    let decision_json = serde_json::to_value(&decision_output)
        .map_err(|err| JobError::Fatal(format!("failed to serialize decision: {err}")))?;

    let mut telemetry = safety_result.to_telemetry_json();
    if let Some(obj) = telemetry.as_object_mut() {
        obj.insert("source".to_string(), json!(source.as_str()));
    }

    let new_decision = NewDecision {
        org_id: DEFAULT_ORG_ID,
        user_id: DEFAULT_USER_ID,
        account_id: payload.account_id.clone(),
        message_id: payload.message_id.clone(),
        source,
        decision_json,
        action_type: Some(decision_output.decision.action.as_str().to_string()),
        confidence: Some(decision_output.decision.confidence),
        needs_approval: safety_result.requires_approval,
        rationale: Some(decision_output.decision.rationale.clone()),
        telemetry_json: telemetry,
    };

    let decision = decision_repo
        .create(new_decision)
        .await
        .map_err(|err| JobError::retryable(format!("failed to persist decision: {err}")))?;

    // Create action record
    let action_repo = ActionRepository::new(dispatcher.db.clone());
    let action_status = if safety_result.requires_approval {
        ActionStatus::ApprovedPending
    } else {
        ActionStatus::Queued
    };

    let undo_hint_json = serde_json::to_value(&decision_output.undo_hint)
        .map_err(|err| JobError::Fatal(format!("failed to serialize undo_hint: {err}")))?;

    let new_action = NewAction {
        org_id: DEFAULT_ORG_ID,
        user_id: DEFAULT_USER_ID,
        account_id: payload.account_id.clone(),
        message_id: payload.message_id.clone(),
        decision_id: Some(decision.id.clone()),
        action_type: decision_output.decision.action.as_str().to_string(),
        parameters_json: decision_output.decision.parameters.clone(),
        status: action_status.clone(),
        error_message: None,
        executed_at: None,
        undo_hint_json,
        trace_id: None,
    };

    let action = action_repo
        .create(new_action)
        .await
        .map_err(|err| JobError::retryable(format!("failed to persist action: {err}")))?;

    enqueue_follow_up_job(
        dispatcher,
        safety_result.requires_approval,
        &payload.account_id,
        &payload.message_id,
        &action.id,
    )
    .await?;

    info!(
        account_id = %payload.account_id,
        message_id = %payload.message_id,
        decision_id = %decision.id,
        source = ?decision.source,
        action = %decision_output.decision.action.as_str(),
        needs_approval = %safety_result.requires_approval,
        "classified email message"
    );

    Ok(())
}

async fn enqueue_follow_up_job(
    dispatcher: &JobDispatcher,
    requires_approval: bool,
    account_id: &str,
    message_id: &str,
    action_id: &str,
) -> Result<(), JobError> {
    let queue = JobQueue::new(dispatcher.db.clone());

    if requires_approval {
        let payload = json!({
            "account_id": account_id,
            "message_id": message_id,
            "action_id": action_id,
        });
        let idempotency_key =
            format!("{JOB_TYPE_APPROVAL_NOTIFY}:{account_id}:{message_id}:{action_id}");

        match queue
            .enqueue(
                JOB_TYPE_APPROVAL_NOTIFY,
                payload,
                Some(idempotency_key),
                0,
            )
            .await
        {
            Ok(_) => {}
            Err(QueueError::DuplicateIdempotency { .. }) => {
                debug!(account_id, action_id, "approval notify job already enqueued");
            }
            Err(err) => {
                return Err(JobError::retryable(format!(
                    "failed to enqueue approval notify job: {err}"
                )))
            }
        }
    } else {
        let payload = json!({
            "account_id": account_id,
            "message_id": message_id,
            "action_id": action_id,
        });
        let idempotency_key =
            format!("{JOB_TYPE_ACTION_GMAIL}:{account_id}:{message_id}:{action_id}");

        match queue
            .enqueue(JOB_TYPE_ACTION_GMAIL, payload, Some(idempotency_key), 0)
            .await
        {
            Ok(_) => {}
            Err(QueueError::DuplicateIdempotency { .. }) => {
                debug!(account_id, action_id, "action job already enqueued");
            }
            Err(err) => {
                return Err(JobError::retryable(format!(
                    "failed to enqueue action job: {err}"
                )))
            }
        }
    }

    Ok(())
}

/// Load LLM rules for all applicable scopes based on a message.
///
/// This function loads rules from:
/// 1. Global scope (scope_ref = None)
/// 2. Account scope (scope_ref = account_id)
/// 3. Domain scope (scope_ref = sender_domain)
/// 4. Sender scope (scope_ref = sender_email)
///
/// Results are merged and deduped by rule ID.
pub async fn load_llm_rules_for_message(
    repo: &LlmRuleRepository,
    org_id: i64,
    user_id: i64,
    account_id: &str,
    sender_email: Option<&str>,
) -> Result<Vec<LlmRule>, crate::rules::repositories::LlmRuleError> {
    let mut rules = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    // Load global rules
    let global_rules = repo
        .list_enabled_by_scope(org_id, user_id, RuleScope::Global, None)
        .await?;
    for rule in global_rules {
        if seen_ids.insert(rule.id.clone()) {
            rules.push(rule);
        }
    }

    // Load account rules
    let account_rules = repo
        .list_enabled_by_scope(org_id, user_id, RuleScope::Account, Some(account_id))
        .await?;
    for rule in account_rules {
        if seen_ids.insert(rule.id.clone()) {
            rules.push(rule);
        }
    }

    // Load domain and sender rules if we have a sender email
    if let Some(email) = sender_email {
        // Extract domain and load domain rules
        if let Some(domain) = extract_domain(email) {
            let domain_lower = domain.to_lowercase();
            let domain_rules = repo
                .list_enabled_by_scope(org_id, user_id, RuleScope::Domain, Some(&domain_lower))
                .await?;
            for rule in domain_rules {
                if seen_ids.insert(rule.id.clone()) {
                    rules.push(rule);
                }
            }
        }

        // Load sender rules
        let email_lower = email.to_lowercase();
        let sender_rules = repo
            .list_enabled_by_scope(org_id, user_id, RuleScope::Sender, Some(&email_lower))
            .await?;
        for rule in sender_rules {
            if seen_ids.insert(rule.id.clone()) {
                rules.push(rule);
            }
        }
    }

    Ok(rules)
}

/// Convert a deterministic RuleMatch to a DecisionOutput for consistent handling.
///
/// This creates a DecisionOutput with:
/// - Confidence set to 1.0 (deterministic match)
/// - needs_approval based on safe_mode
/// - Rationale describing the matched rule
pub fn rule_match_to_decision_output(message: &Message, rule_match: &RuleMatch) -> DecisionOutput {
    // Parse action type from the rule
    let action = rule_match
        .action_type
        .parse::<ActionType>()
        .unwrap_or(ActionType::None);

    // Determine needs_approval based on safe_mode
    let needs_approval = match rule_match.safe_mode {
        SafeMode::DangerousOverride => false,
        SafeMode::AlwaysSafe => false,
        SafeMode::Default => action.danger_level().requires_approval(),
    };

    // Generate undo hint based on action type
    let (inverse_action, inverse_parameters) =
        generate_undo_hint(action, &rule_match.action_parameters);

    DecisionOutput {
        message_ref: MessageRef {
            provider: "gmail".into(),
            account_id: message.account_id.clone(),
            thread_id: message.thread_id.clone(),
            message_id: message.id.clone(),
        },
        decision: DecisionDetails {
            action,
            parameters: rule_match.action_parameters.clone(),
            confidence: 1.0,
            needs_approval,
            rationale: format!(
                "Matched deterministic rule '{}' (priority {})",
                rule_match.rule.name, rule_match.rule.priority
            ),
        },
        explanations: Explanations {
            salient_features: vec![],
            matched_directions: vec![],
            considered_alternatives: vec![],
        },
        undo_hint: UndoHint {
            inverse_action,
            inverse_parameters,
        },
        telemetry: TelemetryPlaceholder::default(),
    }
}

/// Generate an undo hint for a given action type.
fn generate_undo_hint(
    action: ActionType,
    _parameters: &serde_json::Value,
) -> (ActionType, serde_json::Value) {
    match action {
        ActionType::Archive => (ActionType::Move, json!({"destination": "INBOX"})),
        ActionType::Delete => (ActionType::None, json!({"note": "cannot undo delete"})),
        ActionType::MarkRead => (ActionType::MarkUnread, json!({})),
        ActionType::MarkUnread => (ActionType::MarkRead, json!({})),
        ActionType::Star => (ActionType::Unstar, json!({})),
        ActionType::Unstar => (ActionType::Star, json!({})),
        ActionType::ApplyLabel => (ActionType::None, json!({"note": "remove applied label"})),
        ActionType::Move => (ActionType::Move, json!({"destination": "INBOX"})),
        ActionType::Forward => (ActionType::None, json!({"note": "cannot undo forward"})),
        ActionType::AutoReply => (ActionType::None, json!({"note": "cannot undo auto_reply"})),
        ActionType::CreateTask => (ActionType::None, json!({"note": "delete created task"})),
        ActionType::Snooze => (ActionType::None, json!({"note": "unsnooze message"})),
        ActionType::AddNote => (ActionType::None, json!({"note": "remove added note"})),
        ActionType::Escalate => (ActionType::None, json!({"note": "cannot undo escalate"})),
        ActionType::None => (ActionType::None, json!({})),
    }
}

/// Run LLM classification for a message.
async fn run_llm_classification(
    dispatcher: &JobDispatcher,
    message: &Message,
    account_id: &str,
) -> Result<DecisionOutput, JobError> {
    // Load directions
    let directions_repo = DirectionsRepository::new(dispatcher.db.clone());
    let directions = directions_repo
        .list_enabled(DEFAULT_ORG_ID, DEFAULT_USER_ID)
        .await
        .map_err(|err| JobError::retryable(format!("failed to load directions: {err}")))?;

    // Load LLM rules for all applicable scopes
    let llm_rules_repo = LlmRuleRepository::new(dispatcher.db.clone());
    let llm_rules = load_llm_rules_for_message(
        &llm_rules_repo,
        DEFAULT_ORG_ID,
        DEFAULT_USER_ID,
        account_id,
        message.from_email.as_deref(),
    )
    .await
    .map_err(|err| JobError::retryable(format!("failed to load LLM rules: {err}")))?;

    // Build prompt
    let prompt_builder = PromptBuilder::new();
    let messages = prompt_builder.build(message, &directions, &llm_rules, None);

    // Build decision tool
    let decision_tool = build_decision_tool();

    // Create completion request
    let request = CompletionRequest {
        messages,
        temperature: 0.2,
        max_tokens: 2048,
        json_mode: false,
        tools: vec![decision_tool],
    };

    // Call LLM
    let context = crate::llm::LlmCallContext {
        feature: "classify".into(),
        org_id: Some(DEFAULT_ORG_ID),
        user_id: Some(DEFAULT_USER_ID),
        account_id: Some(account_id.to_string()),
        message_id: Some(message.id.clone()),
        thread_id: Some(message.thread_id.clone()),
        rule_name: None,
        rule_id: None,
    };

    let response = dispatcher
        .llm_client
        .complete(request, context)
        .await
        .map_err(|err| map_llm_error("LLM classification", err))?;

    // Parse decision from tool calls
    let decision = DecisionOutput::parse_from_tool_calls(&response.tool_calls, DECISION_TOOL_NAME)
        .map_err(|err| JobError::Fatal(format!("failed to parse LLM decision: {err}")))?;

    Ok(decision)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::{AccountConfig, AccountRepository, PubsubConfig};
    use crate::config::PolicyConfig;
    use crate::gmail::OAuthTokens;
    use crate::gmail::types::Header;
    use crate::llm::MockLLMClient;
    use crate::messages::{Mailbox, NewMessage};
    use crate::migrations::run_migrations;
    use crate::queue::JobQueue;
    use crate::rules::types::DeterministicRule;
    use crate::threads::ThreadRepository;
    use chrono::Utc;
    use libsql::params;
    use serde_json::json;
    use std::sync::Arc;
    use tempfile::TempDir;

    async fn setup_db() -> (crate::Database, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        let db_name = format!("db_{}.sqlite", uuid::Uuid::new_v4());
        let db_path = dir.path().join(db_name);
        let db = crate::Database::new(&db_path).await.expect("create db");
        run_migrations(&db).await.expect("migrations");
        (db, dir)
    }

    async fn seed_account(db: &crate::Database) -> String {
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
        repo.create(
            DEFAULT_ORG_ID,
            DEFAULT_USER_ID,
            "user@example.com",
            Some("User".into()),
            config,
        )
        .await
        .expect("create account")
        .id
    }

    async fn seed_thread(
        db: &crate::Database,
        account_id: &str,
        provider_thread_id: &str,
    ) -> String {
        let repo = ThreadRepository::new(db.clone());
        repo.upsert(
            DEFAULT_ORG_ID,
            DEFAULT_USER_ID,
            account_id,
            provider_thread_id,
            Some("Subject".into()),
            Some("Snippet".into()),
            Some(Utc::now()),
            json!({"raw": true}),
        )
        .await
        .expect("create thread")
        .id
    }

    async fn seed_message(
        db: &crate::Database,
        account_id: &str,
        thread_id: &str,
        provider_message_id: &str,
    ) -> String {
        let repo = MessageRepository::new(db.clone());
        let msg = NewMessage {
            org_id: DEFAULT_ORG_ID,
            user_id: DEFAULT_USER_ID,
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
            subject: Some("Your package has shipped".into()),
            snippet: Some("Snippet".into()),
            received_at: Some(Utc::now()),
            internal_date: Some(Utc::now()),
            labels: vec!["INBOX".into()],
            headers: vec![Header {
                name: "X-Custom".into(),
                value: "value".into(),
            }],
            body_plain: Some("Hi there".into()),
            body_html: Some("<p>Hi there</p>".into()),
            raw_json: json!({"raw": true}),
        };
        repo.upsert(msg).await.expect("create message").id
    }

    fn sample_message(account_id: &str, thread_id: &str, message_id: &str) -> Message {
        Message {
            id: message_id.to_string(),
            account_id: account_id.to_string(),
            thread_id: thread_id.to_string(),
            provider_message_id: "provider-msg-1".into(),
            from_email: Some("alice@example.com".into()),
            from_name: Some("Alice".into()),
            to: vec![Mailbox {
                email: "bob@example.com".into(),
                name: Some("Bob".into()),
            }],
            cc: vec![],
            bcc: vec![],
            subject: Some("Your package has shipped".into()),
            snippet: Some("Snippet".into()),
            received_at: Some(Utc::now()),
            internal_date: Some(Utc::now()),
            labels: vec!["INBOX".into()],
            headers: vec![],
            body_plain: Some("Hello".into()),
            body_html: None,
            raw_json: json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            org_id: DEFAULT_ORG_ID,
            user_id: DEFAULT_USER_ID,
        }
    }

    #[test]
    fn rule_match_to_decision_output_sets_confidence_to_1() {
        let message = sample_message("acct1", "thread1", "msg1");
        let rule = DeterministicRule {
            id: "rule1".into(),
            org_id: DEFAULT_ORG_ID,
            user_id: Some(DEFAULT_USER_ID),
            name: "Test Rule".into(),
            description: None,
            scope: RuleScope::Global,
            scope_ref: None,
            priority: 10,
            enabled: true,
            conditions_json: json!({}),
            action_type: "archive".into(),
            action_parameters_json: json!({}),
            safe_mode: SafeMode::Default,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let rule_match = RuleMatch {
            rule: rule.clone(),
            action_type: "archive".into(),
            action_parameters: json!({}),
            safe_mode: SafeMode::Default,
        };

        let output = rule_match_to_decision_output(&message, &rule_match);

        assert_eq!(output.decision.confidence, 1.0);
        assert_eq!(output.decision.action, ActionType::Archive);
        assert!(!output.decision.needs_approval);
        assert!(output.decision.rationale.contains("Test Rule"));
    }

    #[test]
    fn rule_match_to_decision_output_respects_safe_mode_dangerous_override() {
        let message = sample_message("acct1", "thread1", "msg1");
        let rule = DeterministicRule {
            id: "rule1".into(),
            org_id: DEFAULT_ORG_ID,
            user_id: Some(DEFAULT_USER_ID),
            name: "Test Rule".into(),
            description: None,
            scope: RuleScope::Global,
            scope_ref: None,
            priority: 10,
            enabled: true,
            conditions_json: json!({}),
            action_type: "delete".into(),
            action_parameters_json: json!({}),
            safe_mode: SafeMode::DangerousOverride,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let rule_match = RuleMatch {
            rule: rule.clone(),
            action_type: "delete".into(),
            action_parameters: json!({}),
            safe_mode: SafeMode::DangerousOverride,
        };

        let output = rule_match_to_decision_output(&message, &rule_match);

        // DangerousOverride should not require approval even for dangerous actions
        assert!(!output.decision.needs_approval);
        assert_eq!(output.decision.action, ActionType::Delete);
    }

    #[test]
    fn rule_match_to_decision_output_default_safe_mode_checks_danger_level() {
        let message = sample_message("acct1", "thread1", "msg1");
        let rule = DeterministicRule {
            id: "rule1".into(),
            org_id: DEFAULT_ORG_ID,
            user_id: Some(DEFAULT_USER_ID),
            name: "Test Rule".into(),
            description: None,
            scope: RuleScope::Global,
            scope_ref: None,
            priority: 10,
            enabled: true,
            conditions_json: json!({}),
            action_type: "delete".into(),
            action_parameters_json: json!({}),
            safe_mode: SafeMode::Default,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let rule_match = RuleMatch {
            rule: rule.clone(),
            action_type: "delete".into(),
            action_parameters: json!({}),
            safe_mode: SafeMode::Default,
        };

        let output = rule_match_to_decision_output(&message, &rule_match);

        // Default safe_mode with dangerous action should require approval
        assert!(output.decision.needs_approval);
        assert_eq!(output.decision.action, ActionType::Delete);
    }

    #[test]
    fn generate_undo_hint_archive() {
        let (action, params) = generate_undo_hint(ActionType::Archive, &json!({}));
        assert_eq!(action, ActionType::Move);
        assert_eq!(params["destination"], "INBOX");
    }

    #[test]
    fn generate_undo_hint_mark_read() {
        let (action, params) = generate_undo_hint(ActionType::MarkRead, &json!({}));
        assert_eq!(action, ActionType::MarkUnread);
        assert_eq!(params, json!({}));
    }

    #[tokio::test]
    async fn classify_invalid_payload_returns_fatal() {
        let (db, _dir) = setup_db().await;
        let queue = JobQueue::new(db.clone());
        let dispatcher = JobDispatcher::new(
            db.clone(),
            reqwest::Client::new(),
            Arc::new(MockLLMClient::new()),
            PolicyConfig::default(),
        );

        let job_id = queue
            .enqueue("classify", json!({"invalid": "payload"}), None, 0)
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch");

        let err = handle_classify(&dispatcher, job)
            .await
            .expect_err("should fail");

        match err {
            JobError::Fatal(msg) => assert!(msg.contains("invalid classify payload")),
            other => panic!("expected Fatal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn classify_message_not_found_returns_fatal() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let queue = JobQueue::new(db.clone());
        let dispatcher = JobDispatcher::new(
            db.clone(),
            reqwest::Client::new(),
            Arc::new(MockLLMClient::new()),
            PolicyConfig::default(),
        );

        let job_id = queue
            .enqueue(
                "classify",
                json!({
                    "account_id": account_id,
                    "message_id": "nonexistent-message"
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch");

        let err = handle_classify(&dispatcher, job)
            .await
            .expect_err("should fail");

        match err {
            JobError::Fatal(msg) => assert!(msg.contains("message not found")),
            other => panic!("expected Fatal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn classify_account_not_found_returns_fatal() {
        let (db, _dir) = setup_db().await;
        // Create a message but with a non-existent account
        let real_account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &real_account_id, "thread1").await;
        let message_id = seed_message(&db, &real_account_id, &thread_id, "msg1").await;

        let queue = JobQueue::new(db.clone());
        let dispatcher = JobDispatcher::new(
            db.clone(),
            reqwest::Client::new(),
            Arc::new(MockLLMClient::new()),
            PolicyConfig::default(),
        );

        let job_id = queue
            .enqueue(
                "classify",
                json!({
                    "account_id": "nonexistent-account",
                    "message_id": message_id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch");

        let err = handle_classify(&dispatcher, job)
            .await
            .expect_err("should fail");

        match err {
            JobError::Fatal(msg) => assert!(msg.contains("not found") || msg.contains("account")),
            other => panic!("expected Fatal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn classify_rejects_message_account_mismatch() {
        let (db, _dir) = setup_db().await;
        let account_one = seed_account(&db).await;
        // Create a second account with a different email to avoid uniqueness conflicts
        let account_repo = AccountRepository::new(db.clone());
        let account_two = account_repo
            .create(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                "second@example.com",
                Some("Second".into()),
                AccountConfig {
                    client_id: "client".into(),
                    client_secret: "secret".into(),
                    oauth: OAuthTokens {
                        access_token: "access".into(),
                        refresh_token: "refresh".into(),
                        expires_at: Utc::now() + chrono::Duration::hours(1),
                    },
                    pubsub: PubsubConfig::default(),
                },
            )
            .await
            .expect("create second account")
            .id;
        let thread_id = seed_thread(&db, &account_one, "thread1").await;
        let message_id = seed_message(&db, &account_one, &thread_id, "msg1").await;

        let queue = JobQueue::new(db.clone());
        let dispatcher = JobDispatcher::new(
            db.clone(),
            reqwest::Client::new(),
            Arc::new(MockLLMClient::new()),
            PolicyConfig::default(),
        );

        // Send a payload that references a different account than the message belongs to
        let job_id = queue
            .enqueue(
                "classify",
                json!({
                    "account_id": account_two,
                    "message_id": message_id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch");

        let err = handle_classify(&dispatcher, job)
            .await
            .expect_err("should fail due to account mismatch");

        match err {
            JobError::Fatal(msg) => {
                assert!(msg.contains("does not belong"));
            }
            other => panic!("expected Fatal for account mismatch, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn load_llm_rules_for_message_loads_all_scopes() {
        let (db, _dir) = setup_db().await;
        let llm_rules_repo = LlmRuleRepository::new(db.clone());

        // Create rules in different scopes
        llm_rules_repo
            .create(crate::rules::types::NewLlmRule {
                org_id: DEFAULT_ORG_ID,
                user_id: Some(DEFAULT_USER_ID),
                name: "Global rule".into(),
                description: None,
                scope: RuleScope::Global,
                scope_ref: None,
                rule_text: "Global guidance".into(),
                enabled: true,
                metadata_json: json!({}),
            })
            .await
            .expect("create global rule");

        llm_rules_repo
            .create(crate::rules::types::NewLlmRule {
                org_id: DEFAULT_ORG_ID,
                user_id: Some(DEFAULT_USER_ID),
                name: "Account rule".into(),
                description: None,
                scope: RuleScope::Account,
                scope_ref: Some("acct1".into()),
                rule_text: "Account guidance".into(),
                enabled: true,
                metadata_json: json!({}),
            })
            .await
            .expect("create account rule");

        llm_rules_repo
            .create(crate::rules::types::NewLlmRule {
                org_id: DEFAULT_ORG_ID,
                user_id: Some(DEFAULT_USER_ID),
                name: "Domain rule".into(),
                description: None,
                scope: RuleScope::Domain,
                scope_ref: Some("example.com".into()),
                rule_text: "Domain guidance".into(),
                enabled: true,
                metadata_json: json!({}),
            })
            .await
            .expect("create domain rule");

        llm_rules_repo
            .create(crate::rules::types::NewLlmRule {
                org_id: DEFAULT_ORG_ID,
                user_id: Some(DEFAULT_USER_ID),
                name: "Sender rule".into(),
                description: None,
                scope: RuleScope::Sender,
                scope_ref: Some("alice@example.com".into()),
                rule_text: "Sender guidance".into(),
                enabled: true,
                metadata_json: json!({}),
            })
            .await
            .expect("create sender rule");

        // Load rules for a message from alice@example.com
        let rules = load_llm_rules_for_message(
            &llm_rules_repo,
            DEFAULT_ORG_ID,
            DEFAULT_USER_ID,
            "acct1",
            Some("alice@example.com"),
        )
        .await
        .expect("load rules");

        assert_eq!(rules.len(), 4);
        let names: Vec<&str> = rules.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"Global rule"));
        assert!(names.contains(&"Account rule"));
        assert!(names.contains(&"Domain rule"));
        assert!(names.contains(&"Sender rule"));
    }

    #[tokio::test]
    async fn load_llm_rules_for_message_handles_no_sender() {
        let (db, _dir) = setup_db().await;
        let llm_rules_repo = LlmRuleRepository::new(db.clone());

        llm_rules_repo
            .create(crate::rules::types::NewLlmRule {
                org_id: DEFAULT_ORG_ID,
                user_id: Some(DEFAULT_USER_ID),
                name: "Global rule".into(),
                description: None,
                scope: RuleScope::Global,
                scope_ref: None,
                rule_text: "Global guidance".into(),
                enabled: true,
                metadata_json: json!({}),
            })
            .await
            .expect("create global rule");

        // Load rules without sender email
        let rules = load_llm_rules_for_message(
            &llm_rules_repo,
            DEFAULT_ORG_ID,
            DEFAULT_USER_ID,
            "acct1",
            None,
        )
        .await
        .expect("load rules");

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].name, "Global rule");
    }

    #[tokio::test]
    async fn load_llm_rules_for_message_dedupes_by_id() {
        let (db, _dir) = setup_db().await;
        let llm_rules_repo = LlmRuleRepository::new(db.clone());

        // Create the same rule - it should only appear once
        llm_rules_repo
            .create(crate::rules::types::NewLlmRule {
                org_id: DEFAULT_ORG_ID,
                user_id: Some(DEFAULT_USER_ID),
                name: "Global rule".into(),
                description: None,
                scope: RuleScope::Global,
                scope_ref: None,
                rule_text: "Global guidance".into(),
                enabled: true,
                metadata_json: json!({}),
            })
            .await
            .expect("create global rule");

        let rules = load_llm_rules_for_message(
            &llm_rules_repo,
            DEFAULT_ORG_ID,
            DEFAULT_USER_ID,
            "acct1",
            Some("alice@example.com"),
        )
        .await
        .expect("load rules");

        // Should only have the global rule once
        assert_eq!(rules.len(), 1);
    }

    // =========================================================================
    // Integration Tests for Classify Job Handler
    // =========================================================================

    use crate::decisions::{ActionRepository, ActionStatus, DecisionRepository, DecisionSource};
    use crate::llm::types::ToolCallResult;
    use crate::rules::repositories::DeterministicRuleRepository;
    use crate::rules::types::NewDeterministicRule;

    /// Create a deterministic rule that matches messages from a specific sender
    fn create_sender_rule(
        sender_email: &str,
        action_type: &str,
        safe_mode: SafeMode,
    ) -> NewDeterministicRule {
        NewDeterministicRule {
            org_id: DEFAULT_ORG_ID,
            user_id: Some(DEFAULT_USER_ID),
            name: format!("Rule for {}", sender_email),
            description: Some("Test deterministic rule".into()),
            scope: RuleScope::Sender,
            scope_ref: Some(sender_email.to_lowercase()),
            priority: 10,
            enabled: true,
            // LeafCondition uses tag = "type", rename_all = "snake_case"
            // SenderEmail variant becomes {"type": "sender_email", "value": "..."}
            conditions_json: json!({
                "type": "sender_email",
                "value": sender_email
            }),
            action_type: action_type.into(),
            action_parameters_json: json!({}),
            safe_mode,
        }
    }

    /// Build a valid DecisionOutput for testing LLM responses
    fn build_test_decision_output(
        account_id: &str,
        thread_id: &str,
        message_id: &str,
        action: &str,
        confidence: f64,
        needs_approval: bool,
    ) -> crate::llm::decision::DecisionOutput {
        use crate::llm::decision::{
            ActionType, ConsideredAlternative, DecisionDetails, DecisionOutput, Explanations,
            MessageRef, TelemetryPlaceholder, UndoHint,
        };

        let action_type = action.parse::<ActionType>().unwrap_or(ActionType::None);

        DecisionOutput {
            message_ref: MessageRef {
                provider: "gmail".into(),
                account_id: account_id.to_string(),
                thread_id: thread_id.to_string(),
                message_id: message_id.to_string(),
            },
            decision: DecisionDetails {
                action: action_type,
                parameters: json!({}),
                confidence,
                needs_approval,
                rationale: "LLM determined this action".into(),
            },
            explanations: Explanations {
                salient_features: vec!["test feature".into()],
                matched_directions: vec!["test direction".into()],
                considered_alternatives: vec![ConsideredAlternative {
                    action: ActionType::None,
                    confidence: 0.1,
                    why_not: "Less suitable".into(),
                }],
            },
            undo_hint: UndoHint {
                inverse_action: ActionType::None,
                inverse_parameters: json!({}),
            },
            telemetry: TelemetryPlaceholder::default(),
        }
    }

    // Task 10: Integration test for deterministic rule path
    #[tokio::test]
    async fn classify_deterministic_rule_match_creates_decision_and_action() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "thread1").await;
        let message_id = seed_message(&db, &account_id, &thread_id, "msg1").await;

        // Create a deterministic rule that matches alice@example.com
        let rule_repo = DeterministicRuleRepository::new(db.clone());
        rule_repo
            .create(create_sender_rule(
                "alice@example.com",
                "archive",
                SafeMode::Default,
            ))
            .await
            .expect("create rule");

        // Create mock LLM client - should NOT be called
        let mock_llm = Arc::new(MockLLMClient::new());
        let queue = JobQueue::new(db.clone());
        let dispatcher = JobDispatcher::new(
            db.clone(),
            reqwest::Client::new(),
            mock_llm.clone(),
            PolicyConfig::default(),
        );

        // Create and run classify job
        let job_id = queue
            .enqueue(
                "classify",
                json!({
                    "account_id": account_id,
                    "message_id": message_id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch");

        handle_classify(&dispatcher, job)
            .await
            .expect("classify should succeed");

        // Verify LLM was NOT called (deterministic short-circuit)
        assert_eq!(
            mock_llm.call_count(),
            0,
            "LLM should not be called when deterministic rule matches"
        );

        // Verify Decision was created with source=Deterministic
        let decision_repo = DecisionRepository::new(db.clone());
        let decision = decision_repo
            .get_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("decision should exist");

        assert_eq!(decision.source, DecisionSource::Deterministic);
        assert_eq!(decision.action_type.as_deref(), Some("archive"));
        assert_eq!(decision.confidence, Some(1.0));
        assert!(!decision.needs_approval, "archive is a safe action");

        // Verify Action was created with correct status
        let action_repo = ActionRepository::new(db.clone());
        let actions = action_repo
            .list_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("actions");

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, "archive");
        assert_eq!(
            actions[0].status,
            ActionStatus::Queued,
            "safe action should be Queued, not ApprovedPending"
        );
        assert_eq!(
            actions[0].decision_id.as_deref(),
            Some(decision.id.as_str())
        );
    }

    #[tokio::test]
    async fn classify_enqueues_action_job_for_auto_execute() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "thread1").await;
        let message_id = seed_message(&db, &account_id, &thread_id, "msg1").await;

        // Deterministic rule that does not require approval
        let rule_repo = DeterministicRuleRepository::new(db.clone());
        rule_repo
            .create(create_sender_rule(
                "alice@example.com",
                "archive",
                SafeMode::Default,
            ))
            .await
            .expect("create rule");

        let mock_llm = Arc::new(MockLLMClient::new());
        let queue = JobQueue::new(db.clone());
        let dispatcher = JobDispatcher::new(
            db.clone(),
            reqwest::Client::new(),
            mock_llm,
            PolicyConfig::default(),
        );

        let job_id = queue
            .enqueue(
                "classify",
                json!({
                    "account_id": account_id,
                    "message_id": message_id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch");

        handle_classify(&dispatcher, job)
            .await
            .expect("classify should succeed");

        // There should be exactly one action.gmail job queued
        let conn = db.connection().await.expect("conn");
        let mut rows = conn
            .query(
                "SELECT payload_json FROM jobs WHERE type = ?1",
                params![crate::jobs::JOB_TYPE_ACTION_GMAIL],
            )
            .await
            .expect("query jobs");

        let mut found = 0;
        while let Some(row) = rows.next().await.expect("row") {
            let payload_json: String = row.get(0).expect("payload");
            let payload: serde_json::Value =
                serde_json::from_str(&payload_json).expect("parse payload");
            assert_eq!(payload["account_id"], account_id);
            assert_eq!(payload["message_id"], message_id);
            assert!(payload.get("action_id").is_some());
            found += 1;
        }

        assert_eq!(found, 1, "expected one action job enqueued");
    }

    // Task 10: Test deterministic rule with dangerous action requires approval
    #[tokio::test]
    async fn classify_deterministic_dangerous_action_requires_approval() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "thread1").await;
        let message_id = seed_message(&db, &account_id, &thread_id, "msg1").await;

        // Create a deterministic rule for delete action (dangerous)
        let rule_repo = DeterministicRuleRepository::new(db.clone());
        rule_repo
            .create(create_sender_rule(
                "alice@example.com",
                "delete",
                SafeMode::Default, // Default mode means dangerous actions require approval
            ))
            .await
            .expect("create rule");

        let mock_llm = Arc::new(MockLLMClient::new());
        let queue = JobQueue::new(db.clone());
        let dispatcher = JobDispatcher::new(
            db.clone(),
            reqwest::Client::new(),
            mock_llm.clone(),
            PolicyConfig::default(),
        );

        let job_id = queue
            .enqueue(
                "classify",
                json!({
                    "account_id": account_id,
                    "message_id": message_id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch");

        handle_classify(&dispatcher, job).await.expect("classify");

        // Verify LLM was NOT called
        assert_eq!(mock_llm.call_count(), 0);

        // Verify Decision has needs_approval=true due to dangerous action
        let decision_repo = DecisionRepository::new(db.clone());
        let decision = decision_repo
            .get_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("decision");

        assert!(
            decision.needs_approval,
            "delete is a dangerous action and should require approval"
        );

        // Verify Action status is ApprovedPending
        let action_repo = ActionRepository::new(db.clone());
        let actions = action_repo
            .list_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("actions");

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].status, ActionStatus::ApprovedPending);
    }

    // Test that DangerousOverride actually bypasses approval for dangerous actions
    #[tokio::test]
    async fn classify_dangerous_override_bypasses_safety_enforcement() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "thread1").await;
        let message_id = seed_message(&db, &account_id, &thread_id, "msg1").await;

        // Create a deterministic rule for delete action with DangerousOverride
        let rule_repo = DeterministicRuleRepository::new(db.clone());
        rule_repo
            .create(create_sender_rule(
                "alice@example.com",
                "delete",
                SafeMode::DangerousOverride, // Explicit override to bypass approval
            ))
            .await
            .expect("create rule");

        let mock_llm = Arc::new(MockLLMClient::new());
        let queue = JobQueue::new(db.clone());
        let dispatcher = JobDispatcher::new(
            db.clone(),
            reqwest::Client::new(),
            mock_llm.clone(),
            PolicyConfig::default(),
        );

        let job_id = queue
            .enqueue(
                "classify",
                json!({
                    "account_id": account_id,
                    "message_id": message_id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch");

        handle_classify(&dispatcher, job).await.expect("classify");

        // Verify LLM was NOT called
        assert_eq!(mock_llm.call_count(), 0);

        // Verify Decision has needs_approval=false despite being a dangerous action
        let decision_repo = DecisionRepository::new(db.clone());
        let decision = decision_repo
            .get_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("decision");

        assert!(
            !decision.needs_approval,
            "DangerousOverride should bypass approval even for dangerous actions"
        );

        // Verify Action status is Queued (not ApprovedPending)
        let action_repo = ActionRepository::new(db.clone());
        let actions = action_repo
            .list_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("actions");

        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0].status,
            ActionStatus::Queued,
            "DangerousOverride should result in Queued status"
        );

        // Verify no safety overrides were applied
        let telemetry = &decision.telemetry_json;
        let overrides = telemetry
            .get("safety_overrides")
            .and_then(|v| v.as_array());
        assert!(
            overrides.map(|arr| arr.is_empty()).unwrap_or(true),
            "No safety overrides should be applied for DangerousOverride"
        );
    }

    // Test that AlwaysSafe bypasses approval for actions in the approval_always list
    #[tokio::test]
    async fn classify_always_safe_bypasses_safety_enforcement() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "thread1").await;
        let message_id = seed_message(&db, &account_id, &thread_id, "msg1").await;

        // Create a deterministic rule for forward action with AlwaysSafe
        // (forward is in the default approval_always list)
        let rule_repo = DeterministicRuleRepository::new(db.clone());
        rule_repo
            .create(create_sender_rule(
                "alice@example.com",
                "forward",
                SafeMode::AlwaysSafe, // Explicit override to bypass approval
            ))
            .await
            .expect("create rule");

        let mock_llm = Arc::new(MockLLMClient::new());
        let queue = JobQueue::new(db.clone());
        let dispatcher = JobDispatcher::new(
            db.clone(),
            reqwest::Client::new(),
            mock_llm.clone(),
            PolicyConfig::default(), // forward is in approval_always by default
        );

        let job_id = queue
            .enqueue(
                "classify",
                json!({
                    "account_id": account_id,
                    "message_id": message_id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch");

        handle_classify(&dispatcher, job).await.expect("classify");

        // Verify Decision has needs_approval=false despite forward being in approval_always
        let decision_repo = DecisionRepository::new(db.clone());
        let decision = decision_repo
            .get_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("decision");

        assert!(
            !decision.needs_approval,
            "AlwaysSafe should bypass approval even for actions in approval_always list"
        );

        // Verify Action status is Queued (not ApprovedPending)
        let action_repo = ActionRepository::new(db.clone());
        let actions = action_repo
            .list_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("actions");

        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0].status,
            ActionStatus::Queued,
            "AlwaysSafe should result in Queued status"
        );
    }

    #[tokio::test]
    async fn classify_enqueues_approval_job_when_required() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "thread1").await;
        let message_id = seed_message(&db, &account_id, &thread_id, "msg1").await;

        // Deterministic rule that requires approval (delete)
        let rule_repo = DeterministicRuleRepository::new(db.clone());
        rule_repo
            .create(create_sender_rule(
                "alice@example.com",
                "delete",
                SafeMode::Default,
            ))
            .await
            .expect("create rule");

        let queue = JobQueue::new(db.clone());
        let dispatcher = JobDispatcher::new(
            db.clone(),
            reqwest::Client::new(),
            Arc::new(MockLLMClient::new()),
            PolicyConfig::default(),
        );

        let job_id = queue
            .enqueue(
                "classify",
                json!({
                    "account_id": account_id,
                    "message_id": message_id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch");

        handle_classify(&dispatcher, job)
            .await
            .expect("classify should succeed");

        // Verify approval.notify job enqueued
        let conn = db.connection().await.expect("conn");
        let mut rows = conn
            .query(
                "SELECT payload_json FROM jobs WHERE type = ?1",
                params![crate::jobs::JOB_TYPE_APPROVAL_NOTIFY],
            )
            .await
            .expect("query jobs");

        let mut found = 0;
        while let Some(row) = rows.next().await.expect("row") {
            let payload_json: String = row.get(0).expect("payload");
            let payload: serde_json::Value =
                serde_json::from_str(&payload_json).expect("parse payload");
            assert_eq!(payload["account_id"], account_id);
            assert_eq!(payload["message_id"], message_id);
            assert!(payload.get("action_id").is_some());
            found += 1;
        }

        assert_eq!(found, 1, "expected one approval job enqueued");
    }

    // Task 11: Integration test for LLM decision path
    #[tokio::test]
    async fn classify_llm_path_creates_decision_and_action() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "thread1").await;
        let message_id = seed_message(&db, &account_id, &thread_id, "msg1").await;

        // No deterministic rules - should fall through to LLM

        // Create mock LLM client with valid response
        let mock_llm = Arc::new(MockLLMClient::new());
        let decision_output = build_test_decision_output(
            &account_id,
            &thread_id,
            &message_id,
            "archive",
            0.85,
            false,
        );
        let tool_call_result = ToolCallResult {
            call_id: "call_test_123".into(),
            fn_name: "record_decision".into(),
            fn_arguments: serde_json::to_value(&decision_output).expect("serialize"),
        };
        mock_llm.enqueue_response(Ok(crate::llm::types::CompletionResponse {
            content: String::new(),
            model: "test-model".into(),
            input_tokens: 100,
            output_tokens: 50,
            latency_ms: 500,
            tool_calls: vec![tool_call_result],
        }));

        let queue = JobQueue::new(db.clone());
        let dispatcher = JobDispatcher::new(
            db.clone(),
            reqwest::Client::new(),
            mock_llm.clone(),
            PolicyConfig::default(),
        );

        let job_id = queue
            .enqueue(
                "classify",
                json!({
                    "account_id": account_id,
                    "message_id": message_id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch");

        handle_classify(&dispatcher, job).await.expect("classify");

        // Verify LLM WAS called
        assert_eq!(
            mock_llm.call_count(),
            1,
            "LLM should be called when no deterministic rule matches"
        );

        // Verify Decision was created with source=Llm
        let decision_repo = DecisionRepository::new(db.clone());
        let decision = decision_repo
            .get_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("decision");

        assert_eq!(decision.source, DecisionSource::Llm);
        assert_eq!(decision.action_type.as_deref(), Some("archive"));
        assert_eq!(decision.confidence, Some(0.85));
        assert!(!decision.needs_approval);

        // Verify Action was created
        let action_repo = ActionRepository::new(db.clone());
        let actions = action_repo
            .list_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("actions");

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, "archive");
        assert_eq!(actions[0].status, ActionStatus::Queued);
    }

    // Task 12: Integration test for safety enforcement
    #[tokio::test]
    async fn classify_safety_enforcement_overrides_to_require_approval() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "thread1").await;
        let message_id = seed_message(&db, &account_id, &thread_id, "msg1").await;

        // Create mock LLM client with dangerous action (Delete) and low confidence
        let mock_llm = Arc::new(MockLLMClient::new());
        let decision_output = build_test_decision_output(
            &account_id,
            &thread_id,
            &message_id,
            "delete",
            0.5,   // Below default threshold of 0.7
            false, // LLM says no approval needed, but safety should override
        );
        let tool_call_result = ToolCallResult {
            call_id: "call_test_456".into(),
            fn_name: "record_decision".into(),
            fn_arguments: serde_json::to_value(&decision_output).expect("serialize"),
        };
        mock_llm.enqueue_response(Ok(crate::llm::types::CompletionResponse {
            content: String::new(),
            model: "test-model".into(),
            input_tokens: 100,
            output_tokens: 50,
            latency_ms: 500,
            tool_calls: vec![tool_call_result],
        }));

        let queue = JobQueue::new(db.clone());
        let dispatcher = JobDispatcher::new(
            db.clone(),
            reqwest::Client::new(),
            mock_llm.clone(),
            PolicyConfig::default(), // Uses default confidence_default=0.7
        );

        let job_id = queue
            .enqueue(
                "classify",
                json!({
                    "account_id": account_id,
                    "message_id": message_id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch");

        handle_classify(&dispatcher, job).await.expect("classify");

        // Verify Decision has needs_approval=true due to safety enforcement
        let decision_repo = DecisionRepository::new(db.clone());
        let decision = decision_repo
            .get_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("decision");

        assert!(
            decision.needs_approval,
            "SafetyEnforcer should override to require approval"
        );

        let stored_flag = decision.decision_json["decision"]["needs_approval"]
            .as_bool()
            .unwrap();
        assert!(stored_flag, "decision_json should reflect enforced approval flag");

        // Verify telemetry_json contains safety_overrides
        let telemetry = &decision.telemetry_json;
        assert!(
            telemetry.get("safety_overrides").is_some(),
            "telemetry should contain safety_overrides"
        );
        let overrides = telemetry["safety_overrides"].as_array().expect("array");
        assert!(
            !overrides.is_empty(),
            "should have at least one safety override"
        );

        // Should have both DangerousAction and LowConfidence overrides
        let override_strings: Vec<&str> = overrides.iter().filter_map(|v| v.as_str()).collect();
        assert!(
            override_strings.iter().any(|s| s.contains("dangerous")),
            "should have dangerous action override"
        );
        assert!(
            override_strings.iter().any(|s| s.contains("confidence")),
            "should have low confidence override"
        );

        // Verify Action status is ApprovedPending
        let action_repo = ActionRepository::new(db.clone());
        let actions = action_repo
            .list_by_message_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message_id)
            .await
            .expect("actions");

        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0].status,
            ActionStatus::ApprovedPending,
            "action should require approval"
        );
    }

    // Task 13: Integration tests for LLM error handling

    #[tokio::test]
    async fn classify_llm_rate_limited_returns_retryable() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "thread1").await;
        let message_id = seed_message(&db, &account_id, &thread_id, "msg1").await;

        // Mock LLM returns rate limit error
        let mock_llm = Arc::new(MockLLMClient::new());
        mock_llm.enqueue_response(Err(crate::llm::LLMError::RateLimited(
            crate::llm::RateLimitInfo::new(Some(5000)),
        )));

        let queue = JobQueue::new(db.clone());
        let dispatcher = JobDispatcher::new(
            db.clone(),
            reqwest::Client::new(),
            mock_llm.clone(),
            PolicyConfig::default(),
        );

        let job_id = queue
            .enqueue(
                "classify",
                json!({
                    "account_id": account_id,
                    "message_id": message_id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch");

        let err = handle_classify(&dispatcher, job)
            .await
            .expect_err("should fail");

        match err {
            JobError::Retryable {
                message,
                retry_after,
            } => {
                assert!(message.contains("rate limited"));
                assert_eq!(retry_after, Some(std::time::Duration::from_millis(5000)));
            }
            other => panic!("expected Retryable with retry_after, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn classify_llm_authentication_failed_returns_fatal() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "thread1").await;
        let message_id = seed_message(&db, &account_id, &thread_id, "msg1").await;

        // Mock LLM returns auth error
        let mock_llm = Arc::new(MockLLMClient::new());
        mock_llm.enqueue_response(Err(crate::llm::LLMError::AuthenticationFailed));

        let queue = JobQueue::new(db.clone());
        let dispatcher = JobDispatcher::new(
            db.clone(),
            reqwest::Client::new(),
            mock_llm.clone(),
            PolicyConfig::default(),
        );

        let job_id = queue
            .enqueue(
                "classify",
                json!({
                    "account_id": account_id,
                    "message_id": message_id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch");

        let err = handle_classify(&dispatcher, job)
            .await
            .expect_err("should fail");

        match err {
            JobError::Fatal(msg) => {
                assert!(msg.contains("authentication failed"));
            }
            other => panic!("expected Fatal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn classify_llm_server_error_returns_retryable() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "thread1").await;
        let message_id = seed_message(&db, &account_id, &thread_id, "msg1").await;

        // Mock LLM returns server error
        let mock_llm = Arc::new(MockLLMClient::new());
        mock_llm.enqueue_response(Err(crate::llm::LLMError::ServerError(
            "500 Internal Server Error".into(),
        )));

        let queue = JobQueue::new(db.clone());
        let dispatcher = JobDispatcher::new(
            db.clone(),
            reqwest::Client::new(),
            mock_llm.clone(),
            PolicyConfig::default(),
        );

        let job_id = queue
            .enqueue(
                "classify",
                json!({
                    "account_id": account_id,
                    "message_id": message_id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch");

        let err = handle_classify(&dispatcher, job)
            .await
            .expect_err("should fail");

        match err {
            JobError::Retryable { message, .. } => {
                assert!(message.contains("server error"));
            }
            other => panic!("expected Retryable, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn classify_llm_no_tool_call_returns_fatal() {
        let (db, _dir) = setup_db().await;
        let account_id = seed_account(&db).await;
        let thread_id = seed_thread(&db, &account_id, "thread1").await;
        let message_id = seed_message(&db, &account_id, &thread_id, "msg1").await;

        // Mock LLM returns response with NO tool calls (decision parse error)
        let mock_llm = Arc::new(MockLLMClient::new());
        mock_llm.enqueue_response(Ok(crate::llm::types::CompletionResponse {
            content: "I would archive this email.".into(), // Text response, no tool call
            model: "test-model".into(),
            input_tokens: 100,
            output_tokens: 50,
            latency_ms: 500,
            tool_calls: vec![], // Empty - no tool calls
        }));

        let queue = JobQueue::new(db.clone());
        let dispatcher = JobDispatcher::new(
            db.clone(),
            reqwest::Client::new(),
            mock_llm.clone(),
            PolicyConfig::default(),
        );

        let job_id = queue
            .enqueue(
                "classify",
                json!({
                    "account_id": account_id,
                    "message_id": message_id
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch");

        let err = handle_classify(&dispatcher, job)
            .await
            .expect_err("should fail");

        match err {
            JobError::Fatal(msg) => {
                assert!(
                    msg.contains("parse") || msg.contains("tool"),
                    "error should mention parse/tool issue, got: {}",
                    msg
                );
            }
            other => panic!("expected Fatal for NoToolCall, got {:?}", other),
        }
    }
}
