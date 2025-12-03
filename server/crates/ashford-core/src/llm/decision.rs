use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;
use thiserror::Error;

use super::types::ToolCallResult;
use crate::decisions::policy::ActionDangerLevel;

/// Supported actions that the LLM may return.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    ApplyLabel,
    RemoveLabel,
    MarkRead,
    MarkUnread,
    Archive,
    Delete,
    Trash,
    Restore,
    Move,
    Star,
    Unstar,
    Forward,
    AutoReply,
    CreateTask,
    Snooze,
    AddNote,
    Escalate,
    None,
}

impl ActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionType::ApplyLabel => "apply_label",
            ActionType::RemoveLabel => "remove_label",
            ActionType::MarkRead => "mark_read",
            ActionType::MarkUnread => "mark_unread",
            ActionType::Archive => "archive",
            ActionType::Delete => "delete",
            ActionType::Trash => "trash",
            ActionType::Restore => "restore",
            ActionType::Move => "move",
            ActionType::Star => "star",
            ActionType::Unstar => "unstar",
            ActionType::Forward => "forward",
            ActionType::AutoReply => "auto_reply",
            ActionType::CreateTask => "create_task",
            ActionType::Snooze => "snooze",
            ActionType::AddNote => "add_note",
            ActionType::Escalate => "escalate",
            ActionType::None => "none",
        }
    }

    /// Returns the danger level classification for this action type.
    ///
    /// - Safe: ApplyLabel, RemoveLabel, MarkRead, MarkUnread, Archive, Move, Trash, Restore, None
    /// - Reversible: Star, Unstar, Snooze, AddNote, CreateTask
    /// - Dangerous: Delete, Forward, AutoReply, Escalate
    pub fn danger_level(&self) -> ActionDangerLevel {
        match self {
            // Safe actions - unlikely to cause harm
            ActionType::ApplyLabel
            | ActionType::RemoveLabel
            | ActionType::MarkRead
            | ActionType::MarkUnread
            | ActionType::Archive
            | ActionType::Trash
            | ActionType::Restore
            | ActionType::Move
            | ActionType::None => ActionDangerLevel::Safe,

            // Reversible actions - can be easily undone
            ActionType::Star
            | ActionType::Unstar
            | ActionType::Snooze
            | ActionType::AddNote
            | ActionType::CreateTask => ActionDangerLevel::Reversible,

            // Dangerous actions - difficult or impossible to undo
            ActionType::Delete
            | ActionType::Forward
            | ActionType::AutoReply
            | ActionType::Escalate => ActionDangerLevel::Dangerous,
        }
    }
}

impl FromStr for ActionType {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "apply_label" => Ok(Self::ApplyLabel),
            "remove_label" => Ok(Self::RemoveLabel),
            "mark_read" => Ok(Self::MarkRead),
            "mark_unread" => Ok(Self::MarkUnread),
            "archive" => Ok(Self::Archive),
            "delete" => Ok(Self::Delete),
            "trash" => Ok(Self::Trash),
            "restore" => Ok(Self::Restore),
            "move" => Ok(Self::Move),
            "star" => Ok(Self::Star),
            "unstar" => Ok(Self::Unstar),
            "forward" => Ok(Self::Forward),
            "auto_reply" => Ok(Self::AutoReply),
            "create_task" => Ok(Self::CreateTask),
            "snooze" => Ok(Self::Snooze),
            "add_note" => Ok(Self::AddNote),
            "escalate" => Ok(Self::Escalate),
            "none" => Ok(Self::None),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MessageRef {
    pub provider: String,
    pub account_id: String,
    pub thread_id: String,
    pub message_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DecisionDetails {
    pub action: ActionType,
    pub parameters: Value,
    pub confidence: f64,
    pub needs_approval: bool,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ConsideredAlternative {
    pub action: ActionType,
    pub confidence: f64,
    pub why_not: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Explanations {
    pub salient_features: Vec<String>,
    pub matched_directions: Vec<String>,
    pub considered_alternatives: Vec<ConsideredAlternative>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct UndoHint {
    pub inverse_action: ActionType,
    pub inverse_parameters: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, JsonSchema)]
pub struct TelemetryPlaceholder {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DecisionOutput {
    pub message_ref: MessageRef,
    pub decision: DecisionDetails,
    pub explanations: Explanations,
    pub undo_hint: UndoHint,
    pub telemetry: TelemetryPlaceholder,
}

/// Validation errors for a parsed decision.
#[derive(Debug, Error, PartialEq)]
pub enum DecisionValidationError {
    #[error("{0} cannot be empty")]
    EmptyField(&'static str),
    #[error("confidence {0} is out of range [0.0, 1.0]")]
    InvalidConfidence(f64),
    #[error("considered_alternatives[{index}] confidence {confidence} is out of range [0.0, 1.0]")]
    InvalidAlternativeConfidence { index: usize, confidence: f64 },
}

impl DecisionOutput {
    pub fn validate(&self) -> Result<(), DecisionValidationError> {
        fn ensure_non_empty(
            value: &str,
            field: &'static str,
        ) -> Result<(), DecisionValidationError> {
            if value.trim().is_empty() {
                Err(DecisionValidationError::EmptyField(field))
            } else {
                Ok(())
            }
        }

        ensure_non_empty(&self.message_ref.provider, "message_ref.provider")?;
        ensure_non_empty(&self.message_ref.account_id, "message_ref.account_id")?;
        ensure_non_empty(&self.message_ref.thread_id, "message_ref.thread_id")?;
        ensure_non_empty(&self.message_ref.message_id, "message_ref.message_id")?;
        ensure_non_empty(&self.decision.rationale, "decision.rationale")?;

        if !(0.0..=1.0).contains(&self.decision.confidence) {
            return Err(DecisionValidationError::InvalidConfidence(
                self.decision.confidence,
            ));
        }

        for (idx, alt) in self.explanations.considered_alternatives.iter().enumerate() {
            if !(0.0..=1.0).contains(&alt.confidence) {
                return Err(DecisionValidationError::InvalidAlternativeConfidence {
                    index: idx,
                    confidence: alt.confidence,
                });
            }
            ensure_non_empty(&alt.why_not, "considered_alternatives.why_not")?;
        }

        Ok(())
    }
}

/// Errors that can occur while parsing an LLM decision response.
#[derive(Debug, Error)]
pub enum DecisionParseError {
    #[error("no JSON object found in response")]
    NoJsonFound,
    #[error("malformed JSON block in response")]
    MalformedJson,
    #[error("json parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("decision validation failed: {0}")]
    Validation(#[from] DecisionValidationError),
    #[error("no tool call found in response")]
    NoToolCall,
    #[error("wrong tool called: expected '{expected}', got '{actual}'")]
    WrongToolName { expected: String, actual: String },
}

impl PartialEq for DecisionParseError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (DecisionParseError::NoJsonFound, DecisionParseError::NoJsonFound) => true,
            (DecisionParseError::MalformedJson, DecisionParseError::MalformedJson) => true,
            (DecisionParseError::NoToolCall, DecisionParseError::NoToolCall) => true,
            (DecisionParseError::Validation(a), DecisionParseError::Validation(b)) => a == b,
            (DecisionParseError::Json(a), DecisionParseError::Json(b)) => {
                a.to_string() == b.to_string()
            }
            (
                DecisionParseError::WrongToolName {
                    expected: e1,
                    actual: a1,
                },
                DecisionParseError::WrongToolName {
                    expected: e2,
                    actual: a2,
                },
            ) => e1 == e2 && a1 == a2,
            _ => false,
        }
    }
}

impl DecisionOutput {
    /// Parse a decision from a text response containing JSON.
    /// This is the legacy parsing method that extracts JSON from code fences or raw text.
    pub fn parse(response: &str) -> Result<Self, DecisionParseError> {
        let json_str = extract_json_from_response(response)?;
        let parsed: DecisionOutput = serde_json::from_str(json_str)?;
        parsed.validate()?;
        Ok(parsed)
    }

    /// Parse a decision from tool call results.
    /// This is the preferred method when using structured output via tool calling.
    ///
    /// # Arguments
    /// * `tool_calls` - The list of tool calls from the LLM response
    /// * `expected_tool_name` - The expected tool name (usually DECISION_TOOL_NAME)
    ///
    /// # Returns
    /// The parsed and validated DecisionOutput, or an error if parsing fails.
    pub fn parse_from_tool_calls(
        tool_calls: &[ToolCallResult],
        expected_tool_name: &str,
    ) -> Result<Self, DecisionParseError> {
        let tool_call = tool_calls.first().ok_or(DecisionParseError::NoToolCall)?;

        if tool_call.fn_name != expected_tool_name {
            return Err(DecisionParseError::WrongToolName {
                expected: expected_tool_name.to_string(),
                actual: tool_call.fn_name.clone(),
            });
        }

        let parsed: DecisionOutput = serde_json::from_value(tool_call.fn_arguments.clone())?;
        parsed.validate()?;
        Ok(parsed)
    }
}

/// Extracts the JSON slice from an LLM response that may contain extra text or code fences.
pub fn extract_json_from_response(response: &str) -> Result<&str, DecisionParseError> {
    if let Some(slice) = json_in_code_fence(response) {
        return Ok(slice);
    }

    let Some(start_idx) = response.find('{') else {
        return Err(DecisionParseError::NoJsonFound);
    };

    match balanced_brace_slice(response, start_idx) {
        Some((start, end)) => Ok(&response[start..end]),
        None => Err(DecisionParseError::MalformedJson),
    }
}

fn json_in_code_fence<'a>(response: &'a str) -> Option<&'a str> {
    let fence_start = response.find("```")?;
    let content_start = fence_start + 3;
    let rest = &response[content_start..];
    let fence_end_rel = rest.find("```")?;
    let mut content = &response[content_start..content_start + fence_end_rel];
    if let Some(stripped) = content.strip_prefix("json") {
        content = stripped.trim_start();
    }
    Some(content)
}

fn balanced_brace_slice(text: &str, start_idx: usize) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escape = false;
    let mut end_idx = None;

    for (idx, ch) in text.char_indices().skip_while(|(i, _)| *i < start_idx) {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            match ch {
                '\\' => escape = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
                if depth == 0 {
                    end_idx = Some(idx + ch.len_utf8());
                    break;
                }
            }
            _ => {}
        }
    }

    end_idx.map(|end| (start_idx, end))
}

#[cfg(test)]
mod tests {
    use super::*;

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
                confidence: 0.82,
                needs_approval: false,
                rationale: "Routine newsletter".into(),
            },
            explanations: Explanations {
                salient_features: vec!["newsletter".into()],
                matched_directions: vec!["direction-1".into()],
                considered_alternatives: vec![ConsideredAlternative {
                    action: ActionType::ApplyLabel,
                    confidence: 0.3,
                    why_not: "Less confident".into(),
                }],
            },
            undo_hint: UndoHint {
                inverse_action: ActionType::Move,
                inverse_parameters: serde_json::json!({"destination": "INBOX"}),
            },
            telemetry: TelemetryPlaceholder::default(),
        }
    }

    #[test]
    fn action_type_round_trips() {
        for action in [
            ActionType::ApplyLabel,
            ActionType::RemoveLabel,
            ActionType::MarkRead,
            ActionType::MarkUnread,
            ActionType::Archive,
            ActionType::Delete,
            ActionType::Trash,
            ActionType::Restore,
            ActionType::Move,
            ActionType::Star,
            ActionType::Unstar,
            ActionType::Forward,
            ActionType::AutoReply,
            ActionType::CreateTask,
            ActionType::Snooze,
            ActionType::AddNote,
            ActionType::Escalate,
            ActionType::None,
        ] {
            let serialized = serde_json::to_string(&action).expect("serialize");
            let round_tripped: ActionType = serde_json::from_str(&serialized).expect("deserialize");
            assert_eq!(round_tripped, action);
            assert_eq!(action.as_str(), serialized.trim_matches('"'));
            assert_eq!(ActionType::from_str(action.as_str()).unwrap(), action);
        }
        assert!(ActionType::from_str("unknown").is_err());
    }

    #[test]
    fn validate_accepts_valid_decision() {
        let decision = sample_decision();
        assert!(decision.validate().is_ok());
    }

    #[test]
    fn validate_rejects_out_of_range_confidence() {
        let mut decision = sample_decision();
        decision.decision.confidence = 1.5;
        let err = decision.validate().unwrap_err();
        assert!(matches!(err, DecisionValidationError::InvalidConfidence(_)));
    }

    #[test]
    fn validate_accepts_boundary_confidence_values() {
        let mut decision = sample_decision();

        for value in [0.0, 1.0] {
            decision.decision.confidence = value;
            decision.explanations.considered_alternatives[0].confidence = value;
            decision
                .validate()
                .expect("boundary values should be valid");
        }
    }

    #[test]
    fn validate_rejects_empty_fields() {
        let mut decision = sample_decision();
        decision.message_ref.provider.clear();
        let err = decision.validate().unwrap_err();
        assert_eq!(
            err,
            DecisionValidationError::EmptyField("message_ref.provider")
        );
    }

    #[test]
    fn validate_rejects_invalid_alternative_confidence() {
        let mut decision = sample_decision();
        decision.explanations.considered_alternatives[0].confidence = -0.1;
        let err = decision.validate().unwrap_err();
        assert!(matches!(
            err,
            DecisionValidationError::InvalidAlternativeConfidence { .. }
        ));
    }

    #[test]
    fn validate_rejects_empty_rationale() {
        let mut decision = sample_decision();
        decision.decision.rationale.clear();
        let err = decision.validate().unwrap_err();
        assert_eq!(
            err,
            DecisionValidationError::EmptyField("decision.rationale")
        );
    }

    #[test]
    fn validate_rejects_missing_alternative_reason() {
        let mut decision = sample_decision();
        decision.explanations.considered_alternatives[0]
            .why_not
            .clear();
        let err = decision.validate().unwrap_err();
        assert_eq!(
            err,
            DecisionValidationError::EmptyField("considered_alternatives.why_not"),
        );
    }

    #[test]
    fn extract_handles_plain_json() {
        let json = r#"{ "a": 1 }"#;
        let extracted = extract_json_from_response(json).unwrap();
        assert_eq!(extracted.trim(), json);
    }

    #[test]
    fn extract_handles_wrapped_text() {
        let response = "prefix text\n{ \"key\": \"value\" }\nsuffix";
        let extracted = extract_json_from_response(response).unwrap();
        assert_eq!(extracted.trim(), "{ \"key\": \"value\" }");
    }

    #[test]
    fn extract_handles_code_fence() {
        let response = "Sure, here you go:\n```json\n{ \"ok\": true }\n```";
        let extracted = extract_json_from_response(response).unwrap();
        assert_eq!(extracted.trim(), "{ \"ok\": true }");
    }

    #[test]
    fn extract_handles_code_fence_without_language_hint() {
        let response = "```\n{\n  \"ok\": true\n}\n```";
        let extracted = extract_json_from_response(response).unwrap();
        assert_eq!(extracted.trim(), "{\n  \"ok\": true\n}");
    }

    #[test]
    fn extract_handles_braces_in_strings() {
        let response = "text {\"msg\": \"value with } brace\"} trailing";
        let extracted = extract_json_from_response(response).unwrap();
        assert_eq!(extracted.trim(), "{\"msg\": \"value with } brace\"}");
    }

    #[test]
    fn extract_errors_without_json() {
        let err = extract_json_from_response("no braces here").unwrap_err();
        assert_eq!(err, DecisionParseError::NoJsonFound);
    }

    #[test]
    fn extract_errors_on_unbalanced() {
        let err = extract_json_from_response("start { \"a\": 1 ").unwrap_err();
        assert_eq!(err, DecisionParseError::MalformedJson);
    }

    #[test]
    fn parse_valid_response() {
        let decision = sample_decision();
        let json = serde_json::to_string(&decision).unwrap();
        let wrapped = format!("```json\n{json}\n```");
        let parsed = DecisionOutput::parse(&wrapped).expect("parsed");
        assert_eq!(parsed, decision);
    }

    #[test]
    fn parse_detects_validation_error() {
        let mut decision = sample_decision();
        decision.decision.confidence = 1.2;
        let json = serde_json::to_string(&decision).unwrap();
        let err = DecisionOutput::parse(&json).unwrap_err();
        assert!(matches!(err, DecisionParseError::Validation(_)));
    }

    #[test]
    fn parse_propagates_json_error() {
        let err = DecisionOutput::parse("{\"a\": }").unwrap_err();
        assert!(matches!(err, DecisionParseError::Json(_)));
    }

    #[test]
    fn parse_from_tool_calls_succeeds() {
        let decision = sample_decision();
        let tool_calls = vec![ToolCallResult {
            call_id: "call_123".into(),
            fn_name: "record_decision".into(),
            fn_arguments: serde_json::to_value(&decision).unwrap(),
        }];

        let parsed = DecisionOutput::parse_from_tool_calls(&tool_calls, "record_decision")
            .expect("should parse");
        assert_eq!(parsed, decision);
    }

    #[test]
    fn parse_from_tool_calls_errors_on_empty() {
        let tool_calls: Vec<ToolCallResult> = vec![];
        let err =
            DecisionOutput::parse_from_tool_calls(&tool_calls, "record_decision").unwrap_err();
        assert_eq!(err, DecisionParseError::NoToolCall);
    }

    #[test]
    fn parse_from_tool_calls_errors_on_wrong_name() {
        let decision = sample_decision();
        let tool_calls = vec![ToolCallResult {
            call_id: "call_123".into(),
            fn_name: "wrong_tool".into(),
            fn_arguments: serde_json::to_value(&decision).unwrap(),
        }];

        let err =
            DecisionOutput::parse_from_tool_calls(&tool_calls, "record_decision").unwrap_err();
        assert_eq!(
            err,
            DecisionParseError::WrongToolName {
                expected: "record_decision".into(),
                actual: "wrong_tool".into(),
            }
        );
    }

    #[test]
    fn parse_from_tool_calls_validates_decision() {
        let mut decision = sample_decision();
        decision.decision.confidence = 1.5; // Invalid
        let tool_calls = vec![ToolCallResult {
            call_id: "call_123".into(),
            fn_name: "record_decision".into(),
            fn_arguments: serde_json::to_value(&decision).unwrap(),
        }];

        let err =
            DecisionOutput::parse_from_tool_calls(&tool_calls, "record_decision").unwrap_err();
        assert!(matches!(err, DecisionParseError::Validation(_)));
    }

    #[test]
    fn parse_from_tool_calls_handles_invalid_json() {
        let tool_calls = vec![ToolCallResult {
            call_id: "call_123".into(),
            fn_name: "record_decision".into(),
            fn_arguments: serde_json::json!({"invalid": "structure"}),
        }];

        let err =
            DecisionOutput::parse_from_tool_calls(&tool_calls, "record_decision").unwrap_err();
        assert!(matches!(err, DecisionParseError::Json(_)));
    }

    #[test]
    fn action_type_danger_level_classifications() {
        // Safe actions
        for action in [
            ActionType::ApplyLabel,
            ActionType::RemoveLabel,
            ActionType::MarkRead,
            ActionType::MarkUnread,
            ActionType::Archive,
            ActionType::Trash,
            ActionType::Restore,
            ActionType::Move,
            ActionType::None,
        ] {
            assert_eq!(
                action.danger_level(),
                ActionDangerLevel::Safe,
                "{:?} should be Safe",
                action
            );
        }

        // Reversible actions
        for action in [
            ActionType::Star,
            ActionType::Unstar,
            ActionType::Snooze,
            ActionType::AddNote,
            ActionType::CreateTask,
        ] {
            assert_eq!(
                action.danger_level(),
                ActionDangerLevel::Reversible,
                "{:?} should be Reversible",
                action
            );
        }

        // Dangerous actions
        for action in [
            ActionType::Delete,
            ActionType::Forward,
            ActionType::AutoReply,
            ActionType::Escalate,
        ] {
            assert_eq!(
                action.danger_level(),
                ActionDangerLevel::Dangerous,
                "{:?} should be Dangerous",
                action
            );
        }
    }

    #[test]
    fn all_action_types_have_danger_level() {
        // Ensure every ActionType variant has a danger level (compilation check)
        // This test will fail to compile if a new variant is added without updating danger_level()
        let all_actions = [
            ActionType::ApplyLabel,
            ActionType::RemoveLabel,
            ActionType::MarkRead,
            ActionType::MarkUnread,
            ActionType::Archive,
            ActionType::Delete,
            ActionType::Trash,
            ActionType::Restore,
            ActionType::Move,
            ActionType::Star,
            ActionType::Unstar,
            ActionType::Forward,
            ActionType::AutoReply,
            ActionType::CreateTask,
            ActionType::Snooze,
            ActionType::AddNote,
            ActionType::Escalate,
            ActionType::None,
        ];

        // All 18 action types should have a danger level
        assert_eq!(all_actions.len(), 18);
        for action in all_actions {
            // This should not panic - just confirm we get a valid danger level
            let _ = action.danger_level();
        }
    }
}
