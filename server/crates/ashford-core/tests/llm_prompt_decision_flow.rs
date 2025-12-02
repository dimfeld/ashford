use ashford_core::gmail::types::Header;
use ashford_core::llm::{
    ActionType, ChatRole, DECISION_TOOL_NAME, DecisionDetails, DecisionOutput, DecisionParseError,
    Explanations, MessageRef, PromptBuilder, TelemetryPlaceholder, ToolCallResult, UndoHint,
    build_decision_tool,
};
use ashford_core::messages::{Mailbox, Message};
use ashford_core::rules::types::{Direction, LlmRule, RuleScope};
use chrono::Utc;

fn sample_message() -> Message {
    Message {
        id: "msg_1".into(),
        account_id: "acc_1".into(),
        thread_id: "thr_1".into(),
        provider_message_id: "prov_1".into(),
        from_email: Some("news@example.com".into()),
        from_name: Some("News".into()),
        to: vec![Mailbox {
            email: "user@example.com".into(),
            name: Some("User".into()),
        }],
        cc: vec![],
        bcc: vec![],
        subject: Some("Weekly updates and news".into()),
        snippet: Some("Here is your weekly digest".into()),
        received_at: Some(Utc::now()),
        internal_date: None,
        labels: vec!["INBOX".into()],
        headers: vec![Header {
            name: "List-Id".into(),
            value: "<news.list>".into(),
        }],
        body_plain: Some("Welcome to your weekly newsletter".into()),
        body_html: None,
        raw_json: serde_json::json!({}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        org_id: 1,
        user_id: 1,
    }
}

fn sample_decision() -> DecisionOutput {
    DecisionOutput {
        message_ref: MessageRef {
            provider: "gmail".into(),
            account_id: "acc_1".into(),
            thread_id: "thr_1".into(),
            message_id: "msg_1".into(),
        },
        decision: DecisionDetails {
            action: ActionType::Archive,
            parameters: serde_json::json!({"label": "Processed"}),
            confidence: 0.91,
            needs_approval: false,
            rationale: "Routine newsletter".into(),
        },
        explanations: Explanations {
            salient_features: vec!["newsletter".into()],
            matched_directions: vec!["direction-1".into()],
            considered_alternatives: vec![],
        },
        undo_hint: UndoHint {
            inverse_action: ActionType::Move,
            inverse_parameters: serde_json::json!({"destination": "INBOX"}),
        },
        telemetry: TelemetryPlaceholder::default(),
    }
}

#[test]
fn prompt_builder_includes_layers_and_task_directive() {
    let builder = PromptBuilder::new();
    let prompt = builder.build(
        &sample_message(),
        &[Direction {
            id: "d1".into(),
            org_id: 1,
            user_id: None,
            content: "Never delete newsletters".into(),
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }],
        &[LlmRule {
            id: "r1".into(),
            org_id: 1,
            user_id: None,
            name: "Newsletter rule".into(),
            description: Some("Handle newsletters safely".into()),
            scope: RuleScope::Global,
            scope_ref: None,
            rule_text: "Archive newsletters unless urgent".into(),
            enabled: true,
            metadata_json: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }],
        None,
    );

    assert_eq!(prompt.len(), 2);
    assert_eq!(prompt[0].role, ChatRole::System);
    assert_eq!(prompt[1].role, ChatRole::User);
    assert!(prompt[1].content.contains("DIRECTIONS:"));
    assert!(prompt[1].content.contains("LLM RULE: Newsletter rule"));
    assert!(prompt[1].content.contains("MESSAGE CONTEXT:"));
    assert!(prompt[1].content.contains("TASK:"));
    // Now uses tool calling instead of inline JSON schema
    assert!(prompt[1].content.contains("record_decision"));
    assert!(prompt[0].content.contains("record_decision"));
}

#[test]
fn build_decision_tool_creates_valid_tool() {
    let tool = build_decision_tool();
    assert_eq!(tool.name, DECISION_TOOL_NAME);
    assert!(tool.description.is_some());
    assert!(tool.schema.is_some());

    // Verify the schema contains expected fields
    let schema = tool.schema.as_ref().unwrap();
    assert!(schema.get("properties").is_some());
}

#[test]
fn decision_parse_round_trip_and_error_handling() {
    let decision = sample_decision();
    let json = serde_json::to_string_pretty(&decision).unwrap();
    let parsed = DecisionOutput::parse(&json).expect("should parse");
    assert_eq!(parsed, decision);

    let err = DecisionOutput::parse("not json").unwrap_err();
    assert!(matches!(err, DecisionParseError::NoJsonFound));
}

#[test]
fn decision_parse_from_tool_calls_round_trip() {
    let decision = sample_decision();
    let tool_calls = vec![ToolCallResult {
        call_id: "call_abc123".into(),
        fn_name: DECISION_TOOL_NAME.into(),
        fn_arguments: serde_json::to_value(&decision).unwrap(),
    }];

    let parsed = DecisionOutput::parse_from_tool_calls(&tool_calls, DECISION_TOOL_NAME)
        .expect("should parse from tool call");
    assert_eq!(parsed, decision);
}

#[test]
fn decision_parse_from_tool_calls_errors_on_wrong_tool() {
    let decision = sample_decision();
    let tool_calls = vec![ToolCallResult {
        call_id: "call_abc123".into(),
        fn_name: "wrong_tool".into(),
        fn_arguments: serde_json::to_value(&decision).unwrap(),
    }];

    let err = DecisionOutput::parse_from_tool_calls(&tool_calls, DECISION_TOOL_NAME).unwrap_err();
    assert!(matches!(err, DecisionParseError::WrongToolName { .. }));
}
