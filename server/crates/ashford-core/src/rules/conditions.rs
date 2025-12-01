use std::collections::HashMap;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::messages::Message;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogicalOperator {
    #[serde(alias = "AND")]
    And,
    #[serde(alias = "OR")]
    Or,
    #[serde(alias = "NOT")]
    Not,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogicalCondition {
    pub op: LogicalOperator,
    pub children: Vec<Condition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LeafCondition {
    SenderEmail { value: String },
    SenderDomain { value: String },
    SubjectContains { value: String },
    SubjectRegex { value: String },
    HeaderMatch { header: String, pattern: String },
    LabelPresent { value: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Condition {
    Logical(LogicalCondition),
    Leaf(LeafCondition),
}

#[derive(Debug, Error)]
pub enum ConditionError {
    #[error("invalid condition json: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("invalid regex pattern '{pattern}': {source}")]
    InvalidRegex {
        pattern: String,
        source: regex::Error,
    },
    #[error("NOT condition requires exactly 1 child, got {0}")]
    InvalidNotChildCount(usize),
    #[error("empty condition tree")]
    EmptyTree,
}

pub fn parse_condition(value: &Value) -> Result<Condition, ConditionError> {
    if value.is_null() {
        return Err(ConditionError::EmptyTree);
    }
    let condition: Condition = serde_json::from_value(value.clone())?;
    validate_condition(&condition)?;
    Ok(condition)
}

fn validate_condition(condition: &Condition) -> Result<(), ConditionError> {
    match condition {
        Condition::Leaf(_) => Ok(()),
        Condition::Logical(logical) => match logical.op {
            LogicalOperator::And | LogicalOperator::Or => {
                if logical.children.is_empty() {
                    return Err(ConditionError::EmptyTree);
                }
                for child in &logical.children {
                    validate_condition(child)?;
                }
                Ok(())
            }
            LogicalOperator::Not => {
                if logical.children.len() != 1 {
                    return Err(ConditionError::InvalidNotChildCount(logical.children.len()));
                }
                validate_condition(&logical.children[0])
            }
        },
    }
}

#[derive(Debug, Default)]
pub struct EvaluationContext {
    regex_cache: HashMap<String, Regex>,
}

impl EvaluationContext {
    pub fn new() -> Self {
        Self {
            regex_cache: HashMap::new(),
        }
    }

    pub fn get_or_compile_regex(&mut self, pattern: &str) -> Result<&Regex, ConditionError> {
        if !self.regex_cache.contains_key(pattern) {
            let compiled = Regex::new(pattern).map_err(|source| ConditionError::InvalidRegex {
                pattern: pattern.to_string(),
                source,
            })?;
            self.regex_cache.insert(pattern.to_string(), compiled);
        }

        Ok(self
            .regex_cache
            .get(pattern)
            .expect("regex should be present after insertion"))
    }
}

pub fn evaluate(
    condition: &Condition,
    message: &Message,
    ctx: &mut EvaluationContext,
) -> Result<bool, ConditionError> {
    match condition {
        Condition::Leaf(leaf) => evaluate_leaf(leaf, message, ctx),
        Condition::Logical(logical) => match logical.op {
            LogicalOperator::And => {
                if logical.children.is_empty() {
                    return Err(ConditionError::EmptyTree);
                }
                for child in &logical.children {
                    if !evaluate(child, message, ctx)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            LogicalOperator::Or => {
                if logical.children.is_empty() {
                    return Err(ConditionError::EmptyTree);
                }
                for child in &logical.children {
                    if evaluate(child, message, ctx)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            LogicalOperator::Not => {
                if logical.children.len() != 1 {
                    return Err(ConditionError::InvalidNotChildCount(logical.children.len()));
                }
                Ok(!evaluate(&logical.children[0], message, ctx)?)
            }
        },
    }
}

fn evaluate_leaf(
    condition: &LeafCondition,
    message: &Message,
    ctx: &mut EvaluationContext,
) -> Result<bool, ConditionError> {
    match condition {
        LeafCondition::SenderEmail { value } => {
            if let Some(from) = message.from_email.as_deref() {
                Ok(matches_sender_email(value, from))
            } else {
                Ok(false)
            }
        }
        LeafCondition::SenderDomain { value } => {
            if let Some(domain) = message
                .from_email
                .as_deref()
                .and_then(|email| extract_domain(email))
            {
                Ok(domain.eq_ignore_ascii_case(value))
            } else {
                Ok(false)
            }
        }
        LeafCondition::SubjectContains { value } => {
            if let Some(subject) = message.subject.as_deref() {
                Ok(subject.to_lowercase().contains(&value.to_lowercase()))
            } else {
                Ok(false)
            }
        }
        LeafCondition::SubjectRegex { value } => {
            if let Some(subject) = message.subject.as_deref() {
                Ok(ctx.get_or_compile_regex(value)?.is_match(subject))
            } else {
                Ok(false)
            }
        }
        LeafCondition::HeaderMatch { header, pattern } => {
            let regex = ctx.get_or_compile_regex(pattern)?;
            for h in &message.headers {
                if h.name.eq_ignore_ascii_case(header) && regex.is_match(&h.value) {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        LeafCondition::LabelPresent { value } => {
            Ok(message.labels.iter().any(|label| label == value))
        }
    }
}

fn matches_sender_email(pattern: &str, email: &str) -> bool {
    if let Some(domain) = pattern.strip_prefix("*@") {
        return match extract_domain(email) {
            Some(email_domain) => email_domain.eq_ignore_ascii_case(domain),
            None => false,
        };
    }

    pattern.eq_ignore_ascii_case(email)
}

pub(crate) fn extract_domain(email: &str) -> Option<&str> {
    let at_index = email.rfind('@')?;
    let domain = &email[at_index + 1..];
    if domain.is_empty() {
        None
    } else {
        Some(domain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gmail::types::Header;
    use crate::messages::Mailbox;
    use chrono::Utc;

    fn sample_message() -> Message {
        Message {
            id: "msg1".into(),
            account_id: "acct1".into(),
            thread_id: "thread1".into(),
            provider_message_id: "provider1".into(),
            from_email: Some("alice@amazon.com".into()),
            from_name: Some("Alice".into()),
            to: vec![Mailbox {
                email: "bob@example.com".into(),
                name: Some("Bob".into()),
            }],
            cc: vec![],
            bcc: vec![],
            subject: Some("Your package has shipped".into()),
            snippet: None,
            received_at: Some(Utc::now()),
            internal_date: Some(Utc::now()),
            labels: vec!["INBOX".into(), "IMPORTANT".into()],
            headers: vec![
                Header {
                    name: "Subject".into(),
                    value: "Your package has shipped".into(),
                },
                Header {
                    name: "X-Custom".into(),
                    value: "Value".into(),
                },
            ],
            body_plain: None,
            body_html: None,
            raw_json: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            org_id: 1,
            user_id: 1,
        }
    }

    fn evaluate_simple(condition: Condition, message: &Message) -> Result<bool, ConditionError> {
        let mut ctx = EvaluationContext::new();
        evaluate(&condition, message, &mut ctx)
    }

    #[test]
    fn sender_email_exact_match() {
        let mut msg = sample_message();
        let condition = Condition::Leaf(LeafCondition::SenderEmail {
            value: "alice@amazon.com".into(),
        });
        assert!(evaluate_simple(condition, &msg).unwrap());

        msg.from_email = Some("ALICE@AMAZON.COM".into());
        let condition = Condition::Leaf(LeafCondition::SenderEmail {
            value: "alice@amazon.com".into(),
        });
        assert!(evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn sender_email_wildcard() {
        let msg = sample_message();
        let condition = Condition::Leaf(LeafCondition::SenderEmail {
            value: "*@amazon.com".into(),
        });
        assert!(evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn sender_email_wildcard_case_insensitive_domain() {
        let mut msg = sample_message();
        msg.from_email = Some("alice@AMAZON.com".into());
        let condition = Condition::Leaf(LeafCondition::SenderEmail {
            value: "*@amazon.COM".into(),
        });
        assert!(evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn sender_email_no_match() {
        let msg = sample_message();
        let condition = Condition::Leaf(LeafCondition::SenderEmail {
            value: "*@example.com".into(),
        });
        assert!(!evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn sender_email_missing() {
        let mut msg = sample_message();
        msg.from_email = None;
        let condition = Condition::Leaf(LeafCondition::SenderEmail {
            value: "alice@amazon.com".into(),
        });
        assert!(!evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn sender_domain_matches() {
        let msg = sample_message();
        let condition = Condition::Leaf(LeafCondition::SenderDomain {
            value: "Amazon.com".into(),
        });
        assert!(evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn sender_domain_no_match() {
        let msg = sample_message();
        let condition = Condition::Leaf(LeafCondition::SenderDomain {
            value: "example.com".into(),
        });
        assert!(!evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn sender_domain_malformed_email() {
        let mut msg = sample_message();
        msg.from_email = Some("invalid-email".into());
        let condition = Condition::Leaf(LeafCondition::SenderDomain {
            value: "invalid-email".into(),
        });
        assert!(!evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn subject_contains_match() {
        let msg = sample_message();
        let condition = Condition::Leaf(LeafCondition::SubjectContains {
            value: "Package".into(),
        });
        assert!(evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn subject_contains_no_match() {
        let msg = sample_message();
        let condition = Condition::Leaf(LeafCondition::SubjectContains {
            value: "delivered".into(),
        });
        assert!(!evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn subject_contains_missing() {
        let mut msg = sample_message();
        msg.subject = None;
        let condition = Condition::Leaf(LeafCondition::SubjectContains {
            value: "package".into(),
        });
        assert!(!evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn subject_regex_match() {
        let msg = sample_message();
        let condition = Condition::Leaf(LeafCondition::SubjectRegex {
            value: "(?i)package.*shipped".into(),
        });
        assert!(evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn subject_regex_no_match() {
        let msg = sample_message();
        let condition = Condition::Leaf(LeafCondition::SubjectRegex {
            value: "delivered".into(),
        });
        assert!(!evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn subject_regex_missing_subject() {
        let mut msg = sample_message();
        msg.subject = None;
        let condition = Condition::Leaf(LeafCondition::SubjectRegex {
            value: "package".into(),
        });
        assert!(!evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn subject_regex_invalid() {
        let msg = sample_message();
        let condition = Condition::Leaf(LeafCondition::SubjectRegex {
            value: "(unclosed".into(),
        });
        let mut ctx = EvaluationContext::new();
        let result = evaluate(&condition, &msg, &mut ctx);
        assert!(matches!(result, Err(ConditionError::InvalidRegex { .. })));
    }

    #[test]
    fn header_match_found() {
        let msg = sample_message();
        let condition = Condition::Leaf(LeafCondition::HeaderMatch {
            header: "Subject".into(),
            pattern: "package".into(),
        });
        assert!(evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn header_match_header_not_found() {
        let msg = sample_message();
        let condition = Condition::Leaf(LeafCondition::HeaderMatch {
            header: "Missing".into(),
            pattern: ".*".into(),
        });
        assert!(!evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn header_match_value_no_match() {
        let msg = sample_message();
        let condition = Condition::Leaf(LeafCondition::HeaderMatch {
            header: "Subject".into(),
            pattern: "^delivered$".into(),
        });
        assert!(!evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn header_match_case_insensitive_name() {
        let msg = sample_message();
        let condition = Condition::Leaf(LeafCondition::HeaderMatch {
            header: "subject".into(),
            pattern: "shipped".into(),
        });
        assert!(evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn header_match_duplicate_headers() {
        let mut msg = sample_message();
        // Email headers can be duplicated (e.g., Received headers)
        msg.headers = vec![
            Header {
                name: "From".into(),
                value: "Alice <alice@amazon.com>".into(),
            },
            Header {
                name: "X-Tag".into(),
                value: "first".into(),
            },
            Header {
                name: "X-Tag".into(),
                value: "second".into(),
            },
        ];

        // Should match the first X-Tag header
        let first_match = Condition::Leaf(LeafCondition::HeaderMatch {
            header: "X-Tag".into(),
            pattern: "first".into(),
        });
        assert!(evaluate_simple(first_match, &msg).unwrap());

        // Should also match the second X-Tag header
        let second_match = Condition::Leaf(LeafCondition::HeaderMatch {
            header: "X-Tag".into(),
            pattern: "second".into(),
        });
        assert!(evaluate_simple(second_match, &msg).unwrap());

        // Missing header should not match
        let missing_header = Condition::Leaf(LeafCondition::HeaderMatch {
            header: "x-not-present".into(),
            pattern: ".*".into(),
        });
        assert!(!evaluate_simple(missing_header, &msg).unwrap());
    }

    #[test]
    fn label_present_found() {
        let msg = sample_message();
        let condition = Condition::Leaf(LeafCondition::LabelPresent {
            value: "INBOX".into(),
        });
        assert!(evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn label_present_not_found() {
        let msg = sample_message();
        let condition = Condition::Leaf(LeafCondition::LabelPresent {
            value: "SPAM".into(),
        });
        assert!(!evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn label_present_empty_labels() {
        let mut msg = sample_message();
        msg.labels.clear();
        let condition = Condition::Leaf(LeafCondition::LabelPresent {
            value: "INBOX".into(),
        });
        assert!(!evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn and_all_true() {
        let msg = sample_message();
        let condition = Condition::Logical(LogicalCondition {
            op: LogicalOperator::And,
            children: vec![
                Condition::Leaf(LeafCondition::SenderDomain {
                    value: "amazon.com".into(),
                }),
                Condition::Leaf(LeafCondition::SubjectContains {
                    value: "package".into(),
                }),
            ],
        });
        assert!(evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn and_one_false() {
        let msg = sample_message();
        let condition = Condition::Logical(LogicalCondition {
            op: LogicalOperator::And,
            children: vec![
                Condition::Leaf(LeafCondition::SenderDomain {
                    value: "amazon.com".into(),
                }),
                Condition::Leaf(LeafCondition::SubjectContains {
                    value: "delivered".into(),
                }),
            ],
        });
        assert!(!evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn and_empty_children() {
        let msg = sample_message();
        let condition = Condition::Logical(LogicalCondition {
            op: LogicalOperator::And,
            children: vec![],
        });
        let result = evaluate_simple(condition, &msg);
        assert!(matches!(result, Err(ConditionError::EmptyTree)));
    }

    #[test]
    fn or_one_true() {
        let msg = sample_message();
        let condition = Condition::Logical(LogicalCondition {
            op: LogicalOperator::Or,
            children: vec![
                Condition::Leaf(LeafCondition::SubjectContains {
                    value: "delivered".into(),
                }),
                Condition::Leaf(LeafCondition::SubjectContains {
                    value: "shipped".into(),
                }),
            ],
        });
        assert!(evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn or_all_false() {
        let msg = sample_message();
        let condition = Condition::Logical(LogicalCondition {
            op: LogicalOperator::Or,
            children: vec![
                Condition::Leaf(LeafCondition::SubjectContains {
                    value: "delivered".into(),
                }),
                Condition::Leaf(LeafCondition::SubjectContains {
                    value: "tomorrow".into(),
                }),
            ],
        });
        assert!(!evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn or_empty_children() {
        let msg = sample_message();
        let condition = Condition::Logical(LogicalCondition {
            op: LogicalOperator::Or,
            children: vec![],
        });
        let result = evaluate_simple(condition, &msg);
        assert!(matches!(result, Err(ConditionError::EmptyTree)));
    }

    #[test]
    fn not_inverts_true() {
        let msg = sample_message();
        let condition = Condition::Logical(LogicalCondition {
            op: LogicalOperator::Not,
            children: vec![Condition::Leaf(LeafCondition::SubjectContains {
                value: "package".into(),
            })],
        });
        assert!(!evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn not_inverts_false() {
        let msg = sample_message();
        let condition = Condition::Logical(LogicalCondition {
            op: LogicalOperator::Not,
            children: vec![Condition::Leaf(LeafCondition::SubjectContains {
                value: "delivered".into(),
            })],
        });
        assert!(evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn not_wrong_child_count() {
        let msg = sample_message();
        let condition = Condition::Logical(LogicalCondition {
            op: LogicalOperator::Not,
            children: vec![],
        });
        let result = evaluate_simple(condition, &msg);
        assert!(matches!(
            result,
            Err(ConditionError::InvalidNotChildCount(0))
        ));
    }

    #[test]
    fn not_with_multiple_children_errors() {
        let msg = sample_message();
        let condition = Condition::Logical(LogicalCondition {
            op: LogicalOperator::Not,
            children: vec![
                Condition::Leaf(LeafCondition::SubjectContains {
                    value: "package".into(),
                }),
                Condition::Leaf(LeafCondition::SenderDomain {
                    value: "amazon.com".into(),
                }),
            ],
        });
        let result = evaluate_simple(condition, &msg);
        assert!(matches!(
            result,
            Err(ConditionError::InvalidNotChildCount(2))
        ));
    }

    #[test]
    fn nested_and_or() {
        let msg = sample_message();
        let condition = Condition::Logical(LogicalCondition {
            op: LogicalOperator::And,
            children: vec![
                Condition::Leaf(LeafCondition::SenderDomain {
                    value: "amazon.com".into(),
                }),
                Condition::Logical(LogicalCondition {
                    op: LogicalOperator::Or,
                    children: vec![
                        Condition::Leaf(LeafCondition::SubjectContains {
                            value: "shipped".into(),
                        }),
                        Condition::Leaf(LeafCondition::SubjectContains {
                            value: "delivered".into(),
                        }),
                    ],
                }),
            ],
        });
        assert!(evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn nested_or_and() {
        let msg = sample_message();
        let condition = Condition::Logical(LogicalCondition {
            op: LogicalOperator::Or,
            children: vec![
                Condition::Logical(LogicalCondition {
                    op: LogicalOperator::And,
                    children: vec![
                        Condition::Leaf(LeafCondition::SenderDomain {
                            value: "example.com".into(),
                        }),
                        Condition::Leaf(LeafCondition::SubjectContains {
                            value: "shipped".into(),
                        }),
                    ],
                }),
                Condition::Logical(LogicalCondition {
                    op: LogicalOperator::And,
                    children: vec![
                        Condition::Leaf(LeafCondition::SenderDomain {
                            value: "amazon.com".into(),
                        }),
                        Condition::Leaf(LeafCondition::SubjectContains {
                            value: "package".into(),
                        }),
                    ],
                }),
            ],
        });
        assert!(evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn deeply_nested() {
        let msg = sample_message();
        let condition = Condition::Logical(LogicalCondition {
            op: LogicalOperator::And,
            children: vec![
                Condition::Logical(LogicalCondition {
                    op: LogicalOperator::Or,
                    children: vec![Condition::Logical(LogicalCondition {
                        op: LogicalOperator::And,
                        children: vec![Condition::Leaf(LeafCondition::SubjectContains {
                            value: "package".into(),
                        })],
                    })],
                }),
                Condition::Leaf(LeafCondition::SenderDomain {
                    value: "amazon.com".into(),
                }),
            ],
        });
        assert!(evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn complex_tree() {
        let msg = sample_message();
        let condition_json = serde_json::json!({
            "op": "AND",
            "children": [
                { "type": "sender_domain", "value": "amazon.com" },
                { "op": "OR", "children": [
                  { "type": "subject_contains", "value": "shipped" },
                  { "type": "subject_contains", "value": "delivered" }
                ]}
            ]
        });

        let condition = parse_condition(&condition_json).expect("parse");
        assert!(evaluate_simple(condition, &msg).unwrap());
    }

    #[test]
    fn parse_leaf_condition() {
        let json = serde_json::json!({"type": "label_present", "value": "INBOX"});
        let parsed = parse_condition(&json).expect("parse");
        match parsed {
            Condition::Leaf(LeafCondition::LabelPresent { value }) => {
                assert_eq!(value, "INBOX");
            }
            other => panic!("unexpected condition: {:?}", other),
        }
    }

    #[test]
    fn parse_logical_condition() {
        let json = serde_json::json!({
            "op": "and",
            "children": [
                { "type": "label_present", "value": "INBOX" }
            ]
        });
        let parsed = parse_condition(&json).expect("parse");
        match parsed {
            Condition::Logical(logical) => {
                assert_eq!(logical.op, LogicalOperator::And);
                assert_eq!(logical.children.len(), 1);
            }
            other => panic!("unexpected condition: {:?}", other),
        }
    }

    #[test]
    fn parse_nested_tree() {
        let json = serde_json::json!({
            "op": "or",
            "children": [
                { "type": "sender_domain", "value": "amazon.com" },
                { "op": "and", "children": [
                    { "type": "subject_contains", "value": "package" },
                    { "type": "label_present", "value": "IMPORTANT" }
                ]}
            ]
        });

        let parsed = parse_condition(&json).expect("parse");
        if let Condition::Logical(logical) = parsed {
            assert_eq!(logical.op, LogicalOperator::Or);
            assert_eq!(logical.children.len(), 2);
        } else {
            panic!("expected logical condition");
        }
    }

    #[test]
    fn parse_invalid_json() {
        let json = serde_json::json!({"unknown": true});
        let parsed = parse_condition(&json);
        assert!(matches!(parsed, Err(ConditionError::InvalidJson(_))));
    }

    #[test]
    fn parse_empty_tree_errors() {
        let json = serde_json::Value::Null;
        let parsed = parse_condition(&json);
        assert!(matches!(parsed, Err(ConditionError::EmptyTree)));
    }

    #[test]
    fn parse_empty_logical_tree_errors() {
        let json = serde_json::json!({"op": "and", "children": []});
        let parsed = parse_condition(&json);
        assert!(matches!(parsed, Err(ConditionError::EmptyTree)));
    }

    #[test]
    fn parse_nested_empty_logical_tree_errors() {
        let json = serde_json::json!({
            "op": "and",
            "children": [
                { "op": "or", "children": [] }
            ]
        });
        let parsed = parse_condition(&json);
        assert!(matches!(parsed, Err(ConditionError::EmptyTree)));
    }
}
