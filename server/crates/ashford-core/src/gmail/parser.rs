use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};

use crate::gmail::types::{Message, MessagePart};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Recipient {
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ParsedMessage {
    pub from_email: Option<String>,
    pub from_name: Option<String>,
    pub to: Vec<Recipient>,
    pub cc: Vec<Recipient>,
    pub bcc: Vec<Recipient>,
    pub subject: Option<String>,
    pub body_plain: Option<String>,
    pub body_html: Option<String>,
}

pub fn parse_message(message: &Message) -> ParsedMessage {
    let payload = message.payload.as_ref();

    let from = header_value(payload, "From").and_then(parse_single_recipient);
    let to = header_value(payload, "To")
        .map(parse_recipient_list)
        .unwrap_or_default();
    let cc = header_value(payload, "Cc")
        .map(parse_recipient_list)
        .unwrap_or_default();
    let bcc = header_value(payload, "Bcc")
        .map(parse_recipient_list)
        .unwrap_or_default();
    let subject = header_value(payload, "Subject");

    let mut body_plain = None;
    let mut body_html = None;
    if let Some(part) = payload {
        extract_bodies(part, &mut body_plain, &mut body_html, 0);
    }

    ParsedMessage {
        from_email: from.as_ref().map(|r| r.email.clone()),
        from_name: from.and_then(|r| r.name),
        to,
        cc,
        bcc,
        subject,
        body_plain,
        body_html,
    }
}

fn header_value(payload: Option<&MessagePart>, name: &str) -> Option<String> {
    payload.and_then(|p| {
        p.headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case(name))
            .map(|h| h.value.clone())
    })
}

fn parse_single_recipient(value: String) -> Option<Recipient> {
    parse_recipient(value.trim())
}

fn parse_recipient_list(value: String) -> Vec<Recipient> {
    split_addresses(&value)
        .into_iter()
        .filter_map(|s| parse_recipient(s.trim()))
        .collect()
}

fn parse_recipient(input: &str) -> Option<Recipient> {
    if input.is_empty() {
        return None;
    }

    if let (Some(start), Some(end)) = (input.find('<'), input.rfind('>')) {
        let email = input[start + 1..end].trim();
        if email.is_empty() {
            return None;
        }
        let name_raw = input[..start].trim();
        let name = if name_raw.is_empty() {
            None
        } else {
            Some(strip_quotes(name_raw))
        };
        return Some(Recipient {
            email: email.to_string(),
            name,
        });
    }

    let trimmed = input.trim().trim_matches('<').trim_matches('>');
    if trimmed.is_empty() {
        None
    } else {
        Some(Recipient {
            email: trimmed.to_string(),
            name: None,
        })
    }
}

fn strip_quotes(input: &str) -> String {
    let stripped = input
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(input);
    // Unescape escaped quotes within the string
    stripped.replace("\\\"", "\"")
}

fn split_addresses(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut prev_char: Option<char> = None;

    for ch in input.chars() {
        match ch {
            '"' => {
                // Check if this quote is escaped (preceded by backslash)
                let is_escaped = prev_char == Some('\\');
                if !is_escaped {
                    in_quotes = !in_quotes;
                }
                current.push(ch);
            }
            ',' if !in_quotes => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
        prev_char = Some(ch);
    }

    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }

    parts
}

/// Maximum depth for MIME tree traversal to prevent stack overflow from malicious emails
const MAX_MIME_DEPTH: usize = 50;

fn extract_bodies(
    part: &MessagePart,
    body_plain: &mut Option<String>,
    body_html: &mut Option<String>,
    depth: usize,
) {
    // Guard against deeply nested MIME structures that could cause stack overflow
    if depth > MAX_MIME_DEPTH {
        return;
    }

    if let Some(mime) = part.mime_type.as_deref() {
        if let Some(body) = part.body.as_ref() {
            if let Some(data) = body.data.as_ref() {
                let decoded = decode_body(data);
                match mime {
                    m if m.eq_ignore_ascii_case("text/plain") => {
                        if body_plain.is_none() {
                            *body_plain = decoded;
                        }
                    }
                    m if m.eq_ignore_ascii_case("text/html") => {
                        if body_html.is_none() {
                            *body_html = decoded;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    for child in &part.parts {
        extract_bodies(child, body_plain, body_html, depth + 1);
    }
}

fn decode_body(data: &str) -> Option<String> {
    if let Ok(bytes) = URL_SAFE_NO_PAD.decode(data) {
        return Some(String::from_utf8_lossy(&bytes).into_owned());
    }

    if let Ok(bytes) = STANDARD.decode(data) {
        return Some(String::from_utf8_lossy(&bytes).into_owned());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gmail::types::{Header, MessagePartBody};
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;

    fn make_part(mime: &str, text: &str) -> MessagePart {
        MessagePart {
            part_id: None,
            mime_type: Some(mime.to_string()),
            filename: None,
            headers: vec![],
            body: Some(MessagePartBody {
                size: text.len() as i64,
                data: Some(URL_SAFE_NO_PAD.encode(text.as_bytes())),
                attachment_id: None,
            }),
            parts: vec![],
        }
    }

    fn make_message(part: MessagePart, headers: Vec<Header>) -> Message {
        Message {
            id: "msg".into(),
            thread_id: Some("t1".into()),
            label_ids: vec![],
            snippet: None,
            history_id: None,
            internal_date: None,
            payload: Some(MessagePart { headers, ..part }),
            size_estimate: None,
            raw: None,
        }
    }

    #[test]
    fn parses_single_part_plain_text() {
        let headers = vec![
            Header {
                name: "From".into(),
                value: "Alice <alice@example.com>".into(),
            },
            Header {
                name: "To".into(),
                value: "Bob <bob@example.com>".into(),
            },
            Header {
                name: "Subject".into(),
                value: "Hello".into(),
            },
        ];
        let message = make_message(make_part("text/plain", "Hello world"), headers);
        let parsed = parse_message(&message);

        assert_eq!(parsed.from_email.as_deref(), Some("alice@example.com"));
        assert_eq!(parsed.from_name.as_deref(), Some("Alice"));
        assert_eq!(parsed.to.len(), 1);
        assert_eq!(parsed.subject.as_deref(), Some("Hello"));
        assert_eq!(parsed.body_plain.as_deref(), Some("Hello world"));
        assert!(parsed.body_html.is_none());
    }

    #[test]
    fn parses_multipart_alternative() {
        let plain = make_part("text/plain", "Plain body");
        let html = make_part("text/html", "<p>HTML</p>");
        let payload = MessagePart {
            part_id: None,
            mime_type: Some("multipart/alternative".into()),
            filename: None,
            headers: vec![Header {
                name: "To".into(),
                value: "Bob <bob@example.com>".into(),
            }],
            body: None,
            parts: vec![plain.clone(), html.clone()],
        };

        let headers = vec![Header {
            name: "From".into(),
            value: "Alice <alice@example.com>".into(),
        }];
        let message = make_message(payload, headers);
        let parsed = parse_message(&message);

        assert_eq!(parsed.body_plain.as_deref(), Some("Plain body"));
        assert_eq!(parsed.body_html.as_deref(), Some("<p>HTML</p>"));
    }

    #[test]
    fn parses_nested_multipart_mixed_with_alternative() {
        let plain = make_part("text/plain", "Nested plain");
        let html = make_part("text/html", "<p>Nested html</p>");
        let alternative = MessagePart {
            part_id: None,
            mime_type: Some("multipart/alternative".into()),
            filename: None,
            headers: vec![],
            body: None,
            parts: vec![plain, html],
        };

        let mixed = MessagePart {
            part_id: None,
            mime_type: Some("multipart/mixed".into()),
            filename: None,
            headers: vec![Header {
                name: "To".into(),
                value: "bob@example.com".into(),
            }],
            body: None,
            parts: vec![alternative],
        };

        let message = make_message(
            mixed,
            vec![Header {
                name: "From".into(),
                value: "<sender@example.com>".into(),
            }],
        );
        let parsed = parse_message(&message);

        assert_eq!(parsed.from_email.as_deref(), Some("sender@example.com"));
        assert!(parsed.from_name.is_none());
        assert_eq!(parsed.body_plain.as_deref(), Some("Nested plain"));
        assert_eq!(parsed.body_html.as_deref(), Some("<p>Nested html</p>"));
    }

    #[test]
    fn parses_multiple_recipients_and_preserves_names() {
        let headers = vec![
            Header {
                name: "To".into(),
                value: "Bob <bob@example.com>, \"Carol, Sr.\" <carol@example.com>".into(),
            },
            Header {
                name: "Cc".into(),
                value: "dave@example.com".into(),
            },
            Header {
                name: "Bcc".into(),
                value: "<erin@example.com>".into(),
            },
        ];
        let message = make_message(make_part("text/plain", "body"), headers);
        let parsed = parse_message(&message);

        assert_eq!(parsed.to.len(), 2);
        assert_eq!(parsed.to[0].name.as_deref(), Some("Bob"));
        assert_eq!(parsed.to[1].name.as_deref(), Some("Carol, Sr."));
        assert_eq!(parsed.cc[0].email, "dave@example.com");
        assert_eq!(parsed.bcc[0].email, "erin@example.com");
    }

    #[test]
    fn decodes_base64url_body() {
        let data = URL_SAFE_NO_PAD.encode("hello-base64url".as_bytes());
        let part = MessagePart {
            part_id: None,
            mime_type: Some("text/plain".into()),
            filename: None,
            headers: vec![],
            body: Some(crate::gmail::types::MessagePartBody {
                size: 0,
                data: Some(data),
                attachment_id: None,
            }),
            parts: vec![],
        };

        let message = make_message(part, vec![]);
        let parsed = parse_message(&message);
        assert_eq!(parsed.body_plain.as_deref(), Some("hello-base64url"));
    }

    #[test]
    fn handles_escaped_quotes_in_names() {
        // Test escaped quotes within quoted strings per RFC 5322
        let headers = vec![Header {
            name: "To".into(),
            value: r#""John \"Jr.\" Doe" <john@example.com>, "Plain Name" <plain@example.com>"#
                .into(),
        }];
        let message = make_message(make_part("text/plain", "body"), headers);
        let parsed = parse_message(&message);

        assert_eq!(parsed.to.len(), 2);
        assert_eq!(parsed.to[0].email, "john@example.com");
        assert_eq!(parsed.to[0].name.as_deref(), Some("John \"Jr.\" Doe"));
        assert_eq!(parsed.to[1].email, "plain@example.com");
        assert_eq!(parsed.to[1].name.as_deref(), Some("Plain Name"));
    }

    #[test]
    fn depth_limit_prevents_stack_overflow() {
        // Create a deeply nested MIME structure
        fn make_deeply_nested(depth: usize) -> MessagePart {
            if depth == 0 {
                make_part("text/plain", "deep content")
            } else {
                MessagePart {
                    part_id: None,
                    mime_type: Some("multipart/mixed".into()),
                    filename: None,
                    headers: vec![],
                    body: None,
                    parts: vec![make_deeply_nested(depth - 1)],
                }
            }
        }

        // Create a structure deeper than MAX_MIME_DEPTH (50)
        let deep_message = make_message(make_deeply_nested(60), vec![]);
        let parsed = parse_message(&deep_message);

        // The deeply nested content should not be found due to depth limit
        assert!(parsed.body_plain.is_none());
    }
}
