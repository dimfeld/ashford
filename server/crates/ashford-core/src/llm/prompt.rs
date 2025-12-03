use crate::gmail::types::Header;
use crate::labels::Label;
use crate::llm::decision::{ActionType, DecisionOutput};
use crate::llm::types::{ChatMessage, ChatRole, Tool};
use crate::messages::{Mailbox, Message};
use crate::rules::types::{Direction, LlmRule};
use schemars::schema_for;

/// Placeholder for future thread context summaries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThreadContext {}

#[derive(Debug, Clone)]
pub struct PromptBuilder {
    max_body_length: usize,
    max_subject_length: usize,
}

#[derive(Debug, Clone, Default)]
pub struct PromptBuilderConfig {
    pub max_body_length: Option<usize>,
    pub max_subject_length: Option<usize>,
}

const DEFAULT_MAX_BODY_LENGTH: usize = 8_000;
const DEFAULT_MAX_SUBJECT_LENGTH: usize = 500;

impl PromptBuilder {
    pub fn new() -> Self {
        Self {
            max_body_length: DEFAULT_MAX_BODY_LENGTH,
            max_subject_length: DEFAULT_MAX_SUBJECT_LENGTH,
        }
    }

    pub fn with_config(config: PromptBuilderConfig) -> Self {
        Self {
            max_body_length: config.max_body_length.unwrap_or(DEFAULT_MAX_BODY_LENGTH),
            max_subject_length: config
                .max_subject_length
                .unwrap_or(DEFAULT_MAX_SUBJECT_LENGTH),
        }
    }

    pub fn build(
        &self,
        message: &Message,
        directions: &[Direction],
        llm_rules: &[LlmRule],
        thread_context: Option<&ThreadContext>,
        available_labels: &[Label],
    ) -> Vec<ChatMessage> {
        let system = self.build_system_message();

        let mut user_sections = Vec::new();
        let directions_section = build_directions_section(directions);
        if !directions_section.is_empty() {
            user_sections.push(directions_section);
        }

        let rules_section = build_llm_rules_section(llm_rules);
        if !rules_section.is_empty() {
            user_sections.push(rules_section);
        }

        user_sections.push(self.build_message_context(message, thread_context));

        let labels_section = build_available_labels_section(available_labels);
        if !labels_section.is_empty() {
            user_sections.push(labels_section);
        }

        user_sections.push(build_task_directive());

        let user_content = user_sections.join("\n\n");
        let user = ChatMessage {
            role: ChatRole::User,
            content: user_content,
        };

        vec![system, user]
    }

    fn build_system_message(&self) -> ChatMessage {
        let content = [
            "You are the email classification and action engine.",
            "You MUST call the `record_decision` tool to provide your classification decision.",
            "You MUST follow the DIRECTIONS section strictly.",
            "You MUST NOT hallucinate.",
            "If uncertain, choose a safe and reversible action.",
        ]
        .join("\n");

        ChatMessage {
            role: ChatRole::System,
            content,
        }
    }

    fn build_message_context(
        &self,
        message: &Message,
        thread_context: Option<&ThreadContext>,
    ) -> String {
        let mut lines = Vec::new();
        lines.push("MESSAGE CONTEXT:".to_string());

        let from = format_from(message);
        lines.push(format!("From: {from}"));

        let to = format_mailbox_list(&message.to);
        lines.push(format!("To: {to}"));

        if !message.cc.is_empty() {
            let cc = format_mailbox_list(&message.cc);
            lines.push(format!("Cc: {cc}"));
        }

        if !message.bcc.is_empty() {
            let bcc = format_mailbox_list(&message.bcc);
            lines.push(format!("Bcc: {bcc}"));
        }

        if let Some(subject) = message.subject.as_ref() {
            let truncated = truncate_text(subject, self.max_subject_length);
            lines.push(format!("Subject: {truncated}"));
        }

        if let Some(snippet) = message.snippet.as_ref() {
            lines.push(format!("Snippet: {snippet}"));
        }

        let headers = filter_relevant_headers(&message.headers);
        if !headers.is_empty() {
            lines.push("Headers:".to_string());
            for header in headers {
                lines.push(format!("- {}: {}", header.name, header.value));
            }
        }

        lines.push(format!(
            "Labels: {}",
            serde_json::to_string(&message.labels).unwrap_or_else(|_| "[]".to_string())
        ));

        if let Some(body) = get_body_text(message, self.max_body_length) {
            lines.push("Body:".to_string());
            lines.push(body);
        }

        if let Some(_ctx) = thread_context {
            // Reserved for future thread summaries.
        }

        lines.join("\n")
    }
}

pub fn build_directions_section(directions: &[Direction]) -> String {
    if directions.is_empty() {
        return String::new();
    }

    let mut section = String::from("DIRECTIONS:");
    for (idx, dir) in directions.iter().enumerate() {
        section.push('\n');
        section.push_str(&(idx + 1).to_string());
        section.push_str(". ");
        section.push_str(&dir.content);
    }
    section
}

pub fn build_llm_rules_section(rules: &[LlmRule]) -> String {
    if rules.is_empty() {
        return String::new();
    }

    let mut parts = Vec::new();
    for rule in rules {
        let mut lines = Vec::new();
        lines.push(format!("LLM RULE: {}", rule.name));
        if let Some(desc) = rule.description.as_ref() {
            if !desc.trim().is_empty() {
                lines.push(desc.to_string());
            }
        }
        lines.push(rule.rule_text.clone());
        parts.push(lines.join("\n"));
    }

    parts.join("\n\n")
}

/// Builds the AVAILABLE LABELS section for the prompt.
/// Each label is formatted as:
/// - `{name}` if no description
/// - `{name}: {description}` if description exists
pub fn build_available_labels_section(labels: &[Label]) -> String {
    if labels.is_empty() {
        return String::new();
    }

    let mut lines = Vec::new();
    lines.push("AVAILABLE LABELS:".to_string());

    for label in labels {
        let line = match label.description.as_ref() {
            Some(desc) if !desc.trim().is_empty() => format!("- {}: {}", label.name, desc),
            _ => format!("- {}", label.name),
        };
        lines.push(line);
    }

    lines.join("\n")
}

pub fn truncate_text(text: &str, max_len: usize) -> String {
    if max_len == 0 {
        return String::new();
    }

    let char_count = text.chars().count();
    if char_count <= max_len {
        return text.to_string();
    }

    let ellipsis = "...";
    if max_len <= ellipsis.len() {
        return ellipsis[..max_len].to_string();
    }

    let target = max_len - ellipsis.len();
    let mut truncated: String = text.chars().take(target).collect();

    if let Some((idx, _)) = truncated.char_indices().rfind(|(_, ch)| ch.is_whitespace()) {
        truncated.truncate(idx);
    }

    truncated.push_str(ellipsis);
    truncated
}

pub fn strip_html(html: &str) -> String {
    let mut bytes = std::io::Cursor::new(html.as_bytes());
    html2text::from_read(&mut bytes, 80)
        .trim()
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn get_body_text(message: &Message, max_len: usize) -> Option<String> {
    if let Some(plain) = message.body_plain.as_ref() {
        return Some(truncate_text(plain, max_len));
    }

    message
        .body_html
        .as_ref()
        .map(|html| truncate_text(&strip_html(html), max_len))
}

pub fn filter_relevant_headers<'a>(headers: &'a [Header]) -> Vec<&'a Header> {
    const WHITELIST: &[&str] = &[
        "list-id",
        "return-path",
        "x-priority",
        "x-mailer",
        "reply-to",
        "precedence",
    ];

    headers
        .iter()
        .filter(|h| WHITELIST.contains(&h.name.to_ascii_lowercase().as_str()))
        .collect()
}

fn format_mailbox(mailbox: &Mailbox) -> String {
    match mailbox.name.as_ref() {
        Some(name) if !name.trim().is_empty() => format!("{} <{}>", name, mailbox.email),
        _ => mailbox.email.clone(),
    }
}

fn format_mailbox_list(list: &[Mailbox]) -> String {
    if list.is_empty() {
        return "(none)".to_string();
    }

    list.iter()
        .map(format_mailbox)
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_from(message: &Message) -> String {
    match (message.from_name.as_ref(), message.from_email.as_ref()) {
        (Some(name), Some(email)) if !name.trim().is_empty() => format!("{name} <{email}>"),
        (_, Some(email)) => email.to_string(),
        _ => "(unknown)".to_string(),
    }
}

/// The name of the decision tool that the LLM should call.
pub const DECISION_TOOL_NAME: &str = "record_decision";

/// Builds the decision tool with the DecisionOutput JSON schema.
/// The LLM will call this tool to provide structured output.
pub fn build_decision_tool() -> Tool {
    let schema = schema_for!(DecisionOutput);
    let schema_value = serde_json::to_value(schema).expect("schema should serialize");

    Tool::new(DECISION_TOOL_NAME)
        .with_description(
            "Record the classification decision for this email message. \
             You MUST call this tool to provide your decision.",
        )
        .with_schema(schema_value)
}

fn build_task_directive() -> String {
    let actions = [
        ActionType::ApplyLabel,
        ActionType::MarkRead,
        ActionType::MarkUnread,
        ActionType::Archive,
        ActionType::Delete,
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
    let actions_list = actions
        .iter()
        .map(ActionType::as_str)
        .collect::<Vec<_>>()
        .join(", ");

    [
        "TASK:",
        "Analyze this email and call the `record_decision` tool with your classification decision.",
        "",
        "Valid action types:",
        &actions_list,
        "",
        "Requirements:",
        "- Confidence MUST be between 0.0 and 1.0 inclusive.",
        "- If the action is destructive (e.g., delete) and confidence is low, set needs_approval to true.",
        "- Ensure undo_hint.inverse_action can reverse the primary decision.",
        "- You MUST call the record_decision tool - do not return plain text.",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gmail::types::Header;
    use crate::messages::Mailbox;
    use chrono::Utc;

    fn sample_message() -> Message {
        Message {
            id: "msg_1".into(),
            account_id: "acc_1".into(),
            thread_id: "thr_1".into(),
            provider_message_id: "prov".into(),
            from_email: Some("alice@example.com".into()),
            from_name: Some("Alice".into()),
            to: vec![
                Mailbox {
                    email: "bob@example.com".into(),
                    name: Some("Bob".into()),
                },
                Mailbox {
                    email: "carol@example.com".into(),
                    name: None,
                },
            ],
            cc: vec![],
            bcc: vec![],
            subject: Some("Weekly newsletter and updates".into()),
            snippet: Some("Short snippet".into()),
            received_at: Some(Utc::now()),
            internal_date: None,
            labels: vec!["INBOX".into(), "STARRED".into()],
            headers: vec![
                Header {
                    name: "List-Id".into(),
                    value: "<list.project>".into(),
                },
                Header {
                    name: "X-Extra".into(),
                    value: "ignored".into(),
                },
            ],
            body_plain: Some("Hello world".into()),
            body_html: Some("<p>Hello <strong>world</strong></p>".into()),
            raw_json: serde_json::json!({"raw": true}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            org_id: 1,
            user_id: 1,
        }
    }

    #[test]
    fn directions_section_formats_numbered_list() {
        let directions = vec![
            Direction {
                id: "d1".into(),
                org_id: 1,
                user_id: None,
                content: "First".into(),
                enabled: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            Direction {
                id: "d2".into(),
                org_id: 1,
                user_id: None,
                content: "Second".into(),
                enabled: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ];

        let formatted = build_directions_section(&directions);
        assert!(formatted.starts_with("DIRECTIONS:\n1. First\n2. Second"));

        let empty = build_directions_section(&[]);
        assert!(empty.is_empty());
    }

    #[test]
    fn llm_rules_section_handles_descriptions() {
        let rules = vec![
            LlmRule {
                id: "r1".into(),
                org_id: 1,
                user_id: None,
                name: "Rule One".into(),
                description: Some("Describe".into()),
                scope: crate::rules::types::RuleScope::Global,
                scope_ref: None,
                rule_text: "Do X".into(),
                enabled: true,
                metadata_json: serde_json::json!({}),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            LlmRule {
                id: "r2".into(),
                org_id: 1,
                user_id: None,
                name: "Rule Two".into(),
                description: None,
                scope: crate::rules::types::RuleScope::Global,
                scope_ref: None,
                rule_text: "Do Y".into(),
                enabled: true,
                metadata_json: serde_json::json!({}),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ];

        let formatted = build_llm_rules_section(&rules);
        assert!(formatted.contains("LLM RULE: Rule One\nDescribe\nDo X"));
        assert!(formatted.contains("LLM RULE: Rule Two\nDo Y"));

        let empty = build_llm_rules_section(&[]);
        assert!(empty.is_empty());
    }

    #[test]
    fn truncate_text_respects_boundaries() {
        let text = "This is a long body that should be truncated";
        let truncated = truncate_text(text, 20);
        assert!(truncated.ends_with("..."));
        assert!(truncated.len() <= 20);

        let short = truncate_text("short", 20);
        assert_eq!(short, "short");
    }

    #[test]
    fn truncate_text_handles_small_limits_and_zero() {
        assert_eq!(truncate_text("anything", 0), "");
        assert_eq!(truncate_text("abcdef", 2), "..");
        let truncated = truncate_text("abcdef", 3);
        assert_eq!(truncated, "...");
    }

    #[test]
    fn strip_html_removes_tags_and_preserves_text() {
        let html = r#"
            <html>
                <head><style>.hidden { display:none; }</style></head>
                <body>
                    <p>Hello&nbsp;<strong>World</strong></p>
                    <table><tr><td>Cell</td></tr></table>
                    <script>alert('x');</script>
                </body>
            </html>
        "#;
        let text = strip_html(html);
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
        assert!(text.contains("Cell"));
        assert!(!text.contains("alert('x')"));
    }

    #[test]
    fn get_body_text_prefers_plain() {
        let mut message = sample_message();
        let body = get_body_text(&message, 100).unwrap();
        assert_eq!(body, "Hello world");

        message.body_plain = None;
        let body_html = get_body_text(&message, 100).unwrap();
        assert!(body_html.contains("Hello"));
    }

    #[test]
    fn filter_relevant_headers_whitelists_expected_names() {
        let headers = vec![
            Header {
                name: "List-Id".into(),
                value: "list".into(),
            },
            Header {
                name: "Return-Path".into(),
                value: "bounce".into(),
            },
            Header {
                name: "X-Unused".into(),
                value: "ignored".into(),
            },
        ];

        let filtered = filter_relevant_headers(&headers);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].name, "List-Id");
        assert_eq!(filtered[1].name, "Return-Path");
    }

    #[test]
    fn filter_relevant_headers_is_case_insensitive() {
        let headers = vec![
            Header {
                name: "x-priority".into(),
                value: "high".into(),
            },
            Header {
                name: "X-MAILER".into(),
                value: "mailer".into(),
            },
        ];

        let filtered = filter_relevant_headers(&headers);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].value, "high");
        assert_eq!(filtered[1].value, "mailer");
    }

    #[test]
    fn message_context_includes_core_fields() {
        let builder = PromptBuilder::new();
        let context = builder.build_message_context(&sample_message(), None);

        assert!(context.contains("MESSAGE CONTEXT:"));
        assert!(context.contains("From: Alice <alice@example.com>"));
        assert!(context.contains("To: Bob <bob@example.com>, carol@example.com"));
        assert!(context.contains("Subject: Weekly newsletter"));
        assert!(context.contains("Snippet: Short snippet"));
        assert!(context.contains("Headers:"));
        assert!(context.contains("Labels: [\"INBOX\",\"STARRED\"]"));
        assert!(context.contains("Body:"));
    }

    #[test]
    fn message_context_handles_missing_optional_fields() {
        let builder = PromptBuilder::new();
        let mut msg = sample_message();
        msg.from_email = None;
        msg.from_name = None;
        msg.to.clear();
        msg.subject = None;
        msg.snippet = None;
        msg.body_plain = None;
        msg.body_html = None;
        msg.headers.clear();
        msg.labels.clear();

        let context = builder.build_message_context(&msg, None);
        assert!(context.contains("From: (unknown)"));
        assert!(context.contains("To: (none)"));
        assert!(!context.contains("Cc:"));
        assert!(!context.contains("Bcc:"));
        assert!(!context.contains("Subject:"));
        assert!(!context.contains("Snippet:"));
        assert!(!context.contains("Body:"));
    }

    #[test]
    fn build_respects_custom_limits_for_subject_and_body() {
        let builder = PromptBuilder::with_config(PromptBuilderConfig {
            max_body_length: Some(12),
            max_subject_length: Some(10),
        });
        let mut msg = sample_message();
        msg.subject = Some("Extremely long subject line for testing truncation".into());
        msg.body_plain = Some("Body content that will be truncated".into());

        let ctx = builder.build_message_context(&msg, None);
        let subject_line = ctx
            .lines()
            .find(|l| l.starts_with("Subject:"))
            .expect("subject line present");
        assert!(subject_line.len() <= "Subject: ".len() + 10);
        assert!(subject_line.ends_with("..."));

        let body_line = ctx
            .lines()
            .skip_while(|l| *l != "Body:")
            .nth(1)
            .expect("body text line");
        assert!(body_line.len() <= 12);
        assert!(body_line.ends_with("..."));
    }

    #[test]
    fn get_body_text_truncates_html_fallback() {
        let mut msg = sample_message();
        msg.body_plain = None;
        msg.body_html = Some("<p>Hello world from <strong>HTML</strong> body</p>".into());

        let body = get_body_text(&msg, 8).expect("body");
        assert_eq!(body, "Hello...");
    }

    #[test]
    fn task_directive_lists_all_actions() {
        let directive = build_task_directive();
        for action in [
            ActionType::ApplyLabel,
            ActionType::MarkRead,
            ActionType::MarkUnread,
            ActionType::Archive,
            ActionType::Delete,
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
            assert!(
                directive.contains(action.as_str()),
                "directive missing action {}",
                action.as_str()
            );
        }
    }

    #[test]
    fn build_omits_empty_directions_and_rules_sections() {
        let builder = PromptBuilder::new();
        let messages = builder.build(&sample_message(), &[], &[], None, &[]);
        assert_eq!(messages.len(), 2);
        let user_content = &messages[1].content;
        assert!(!user_content.contains("DIRECTIONS:"));
        assert!(!user_content.contains("LLM RULE:"));
        assert!(!user_content.contains("AVAILABLE LABELS:"));
        assert!(user_content.contains("MESSAGE CONTEXT:"));
        assert!(user_content.contains("TASK:"));
    }

    #[test]
    fn build_returns_two_messages_with_sections() {
        let builder = PromptBuilder::new();
        let message = sample_message();
        let directions = vec![Direction {
            id: "d1".into(),
            org_id: 1,
            user_id: None,
            content: "Be safe".into(),
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }];
        let rules = vec![LlmRule {
            id: "r1".into(),
            org_id: 1,
            user_id: None,
            name: "Newsletter".into(),
            description: None,
            scope: crate::rules::types::RuleScope::Global,
            scope_ref: None,
            rule_text: "If newsletter, archive".into(),
            enabled: true,
            metadata_json: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }];

        let messages = builder.build(&message, &directions, &rules, None, &[]);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, ChatRole::System);
        assert_eq!(messages[1].role, ChatRole::User);
        assert!(messages[1].content.contains("DIRECTIONS:"));
        assert!(messages[1].content.contains("LLM RULE: Newsletter"));
        assert!(messages[1].content.contains("MESSAGE CONTEXT:"));
        assert!(messages[1].content.contains("TASK:"));
    }

    fn sample_label(provider_label_id: &str, name: &str, description: Option<&str>) -> Label {
        Label {
            id: format!("id_{}", provider_label_id),
            account_id: "acc_1".into(),
            provider_label_id: provider_label_id.into(),
            name: name.into(),
            label_type: "user".into(),
            description: description.map(|s| s.into()),
            available_to_classifier: true,
            message_list_visibility: Some("show".into()),
            label_list_visibility: Some("labelShow".into()),
            background_color: None,
            text_color: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            org_id: 1,
            user_id: 1,
        }
    }

    #[test]
    fn available_labels_section_formats_correctly() {
        let labels = vec![
            sample_label("Label_1", "Work", Some("Emails related to work")),
            sample_label("Label_2", "Personal", None),
            sample_label("Label_3", "Important", Some("High priority items")),
        ];

        let section = build_available_labels_section(&labels);

        assert!(section.starts_with("AVAILABLE LABELS:"));
        assert!(section.contains("- Work: Emails related to work"));
        assert!(section.contains("- Personal"));
        assert!(!section.contains("- Personal:")); // No description, no colon
        assert!(section.contains("- Important: High priority items"));
    }

    #[test]
    fn available_labels_section_empty_returns_empty() {
        let section = build_available_labels_section(&[]);
        assert!(section.is_empty());
    }

    #[test]
    fn available_labels_section_handles_empty_descriptions() {
        let labels = vec![
            sample_label("Label_1", "Work", Some("")), // Empty string description
            sample_label("Label_2", "Personal", Some("   ")), // Whitespace only
        ];

        let section = build_available_labels_section(&labels);

        // Both should appear without descriptions since they're empty/whitespace
        assert!(section.contains("- Work"));
        assert!(!section.contains("- Work:"));
        assert!(section.contains("- Personal"));
        assert!(!section.contains("- Personal:"));
    }

    #[test]
    fn build_includes_available_labels_section() {
        let builder = PromptBuilder::new();
        let message = sample_message();
        let labels = vec![
            sample_label("Label_1", "Work", Some("Work emails")),
            sample_label("Label_2", "Personal", None),
        ];

        let messages = builder.build(&message, &[], &[], None, &labels);
        let user_content = &messages[1].content;

        assert!(user_content.contains("AVAILABLE LABELS:"));
        assert!(user_content.contains("- Work: Work emails"));
        assert!(user_content.contains("- Personal"));
        // Verify it appears before TASK
        let labels_pos = user_content.find("AVAILABLE LABELS:").unwrap();
        let task_pos = user_content.find("TASK:").unwrap();
        assert!(
            labels_pos < task_pos,
            "AVAILABLE LABELS should appear before TASK"
        );
    }

    #[test]
    fn build_available_labels_after_message_context() {
        let builder = PromptBuilder::new();
        let message = sample_message();
        let labels = vec![sample_label("Label_1", "Work", None)];

        let messages = builder.build(&message, &[], &[], None, &labels);
        let user_content = &messages[1].content;

        // Verify order: MESSAGE CONTEXT -> AVAILABLE LABELS -> TASK
        let msg_ctx_pos = user_content.find("MESSAGE CONTEXT:").unwrap();
        let labels_pos = user_content.find("AVAILABLE LABELS:").unwrap();
        let task_pos = user_content.find("TASK:").unwrap();

        assert!(
            msg_ctx_pos < labels_pos,
            "MESSAGE CONTEXT should be before AVAILABLE LABELS"
        );
        assert!(
            labels_pos < task_pos,
            "AVAILABLE LABELS should be before TASK"
        );
    }

    #[test]
    fn available_labels_section_with_special_characters() {
        let labels = vec![
            sample_label("Label_1", "Work/Projects", Some("Slash in name")),
            sample_label("Label_2", "Family & Friends", None),
            sample_label("Label_3", "Priority!!!", Some("With exclamation")),
            sample_label("Label_4", "TODO: Urgent", None),
            sample_label("Label_5", "Label with \"quotes\"", Some("Has quotes")),
        ];

        let section = build_available_labels_section(&labels);

        assert!(section.contains("- Work/Projects: Slash in name"));
        assert!(section.contains("- Family & Friends"));
        assert!(section.contains("- Priority!!!: With exclamation"));
        assert!(section.contains("- TODO: Urgent"));
        assert!(section.contains("- Label with \"quotes\": Has quotes"));
    }

    #[test]
    fn available_labels_section_with_unicode() {
        let labels = vec![
            sample_label("Label_1", "Travail", Some("French for work")),
            sample_label("Label_2", "Wichtig", Some("German for important")),
            sample_label("Label_3", "Praca", None),
        ];

        let section = build_available_labels_section(&labels);

        assert!(section.contains("- Travail: French for work"));
        assert!(section.contains("- Wichtig: German for important"));
        assert!(section.contains("- Praca"));
    }

    #[test]
    fn available_labels_section_with_long_description() {
        let long_desc = "A".repeat(500);
        let labels = vec![sample_label("Label_1", "Work", Some(&long_desc))];

        let section = build_available_labels_section(&labels);

        // Should include the full description (no truncation in label section)
        assert!(section.contains(&format!("- Work: {}", long_desc)));
    }

    #[test]
    fn available_labels_section_with_system_label() {
        // System labels like INBOX have provider_label_id matching name
        let labels = vec![
            sample_label("INBOX", "INBOX", Some("Primary inbox")),
            sample_label("STARRED", "STARRED", None),
            sample_label("SENT", "SENT", Some("Sent messages")),
        ];

        let section = build_available_labels_section(&labels);

        assert!(section.contains("- INBOX: Primary inbox"));
        assert!(section.contains("- STARRED"));
        assert!(!section.contains("- STARRED:")); // No description
        assert!(section.contains("- SENT: Sent messages"));
    }

    #[test]
    fn available_labels_section_single_label() {
        let labels = vec![sample_label("Label_1", "OnlyLabel", Some("Solo"))];

        let section = build_available_labels_section(&labels);

        assert!(section.starts_with("AVAILABLE LABELS:"));
        assert!(section.contains("- OnlyLabel: Solo"));
        // Should be exactly two lines (header + one label)
        let lines: Vec<&str> = section.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn available_labels_section_description_with_newlines_ignored() {
        // Descriptions with embedded newlines should be handled
        // (though in practice they shouldn't have newlines)
        let labels = vec![sample_label("Label_1", "Work", Some("Line one\nLine two"))];

        let section = build_available_labels_section(&labels);

        // The current implementation doesn't strip newlines - it includes them as-is
        // This test documents current behavior
        assert!(section.contains("- Work: Line one\nLine two"));
    }
}
