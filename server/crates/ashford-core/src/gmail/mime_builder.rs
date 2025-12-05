use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use mail_builder::MessageBuilder;
use mail_builder::headers::address::Address;
use mail_builder::headers::message_id::MessageId;
use thiserror::Error;

/// Simple representation of an email address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailAddress {
    pub email: String,
    pub name: Option<String>,
}

impl EmailAddress {
    pub fn new(name: Option<impl Into<String>>, email: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            name: name.map(|n| n.into()),
        }
    }
}

impl From<&str> for EmailAddress {
    fn from(email: &str) -> Self {
        Self {
            email: email.to_string(),
            name: None,
        }
    }
}

impl From<String> for EmailAddress {
    fn from(email: String) -> Self {
        Self { email, name: None }
    }
}

/// Represents a binary attachment to include in the MIME message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MimeAttachment {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
}

/// High-level MIME message builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MimeMessage {
    pub from: EmailAddress,
    pub to: Vec<EmailAddress>,
    pub cc: Vec<EmailAddress>,
    pub bcc: Vec<EmailAddress>,
    pub subject: Option<String>,
    pub body_plain: Option<String>,
    pub body_html: Option<String>,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    pub attachments: Vec<MimeAttachment>,
}

impl MimeMessage {
    /// Build the RFC 5322 message as a string.
    pub fn to_rfc822(self) -> Result<String, MimeBuildError> {
        if self.to.is_empty() && self.cc.is_empty() && self.bcc.is_empty() {
            return Err(MimeBuildError::MissingRecipients);
        }

        if self.body_plain.is_none() && self.body_html.is_none() && self.attachments.is_empty() {
            return Err(MimeBuildError::MissingBody);
        }

        let mut builder = MessageBuilder::new().from(to_header_address(&self.from));

        if !self.to.is_empty() {
            builder = builder.to(address_list(&self.to));
        }
        if !self.cc.is_empty() {
            builder = builder.cc(address_list(&self.cc));
        }
        if !self.bcc.is_empty() {
            builder = builder.bcc(address_list(&self.bcc));
        }
        if let Some(subject) = self.subject.as_ref() {
            builder = builder.subject(subject.as_str());
        }
        if let Some(body) = self.body_plain.as_ref() {
            builder = builder.text_body(body.as_str());
        }
        if let Some(body) = self.body_html.as_ref() {
            builder = builder.html_body(body.as_str());
        }

        if let Some(in_reply_to) = self
            .in_reply_to
            .as_ref()
            .and_then(|id| normalize_message_id(id))
        {
            builder = builder.in_reply_to(MessageId::new(in_reply_to.clone()));
            builder = builder.references(MessageId::from(combined_references(
                &self.references,
                Some(&in_reply_to),
            )));
        } else if !self.references.is_empty() {
            builder =
                builder.references(MessageId::from(combined_references(&self.references, None)));
        }

        for attachment in self.attachments {
            let content_type = if attachment.content_type.is_empty() {
                "application/octet-stream".to_string()
            } else {
                attachment.content_type.clone()
            };
            builder = builder.attachment(
                content_type,
                attachment.filename.clone(),
                attachment.data.clone(),
            );
        }

        builder.write_to_string().map_err(MimeBuildError::Io)
    }

    /// Build the message and return it base64url encoded for the Gmail API.
    pub fn to_base64_url(self) -> Result<String, MimeBuildError> {
        let raw = self.to_rfc822()?;
        Ok(URL_SAFE_NO_PAD.encode(raw.as_bytes()))
    }
}

#[derive(Debug, Error)]
pub enum MimeBuildError {
    #[error("at least one recipient is required")]
    MissingRecipients,
    #[error("a body or attachment is required")]
    MissingBody,
    #[error("failed to build message: {0}")]
    Io(#[from] std::io::Error),
}

fn to_header_address(addr: &EmailAddress) -> Address<'static> {
    Address::new_address(addr.name.clone(), addr.email.clone())
}

fn address_list(addrs: &[EmailAddress]) -> Address<'static> {
    let list: Vec<Address<'static>> = addrs.iter().map(to_header_address).collect();
    Address::new_list(list)
}

/// Normalize a message ID by removing surrounding whitespace and angle brackets.
/// Returns `None` if the result is empty.
pub fn normalize_message_id(id: &str) -> Option<String> {
    let trimmed = id.trim().trim_matches('<').trim_matches('>');
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Deduplicate a list of message IDs, normalizing each one first.
/// Preserves order, keeping the first occurrence of each unique ID.
pub fn dedup_message_ids(ids: Vec<String>) -> Vec<String> {
    let mut seen = Vec::new();
    for id in ids {
        if let Some(normalized) = normalize_message_id(&id) {
            if !seen.iter().any(|existing: &String| existing == &normalized) {
                seen.push(normalized);
            }
        }
    }
    seen
}

fn combined_references(existing: &[String], in_reply_to: Option<&str>) -> Vec<String> {
    let mut all_ids: Vec<String> = existing.to_vec();
    if let Some(reply) = in_reply_to {
        all_ids.push(reply.to_string());
    }
    dedup_message_ids(all_ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;

    #[test]
    fn builds_mime_with_threading_and_attachment() {
        let message = MimeMessage {
            from: EmailAddress::new(Some("Sender"), "sender@example.com"),
            to: vec![EmailAddress::from("to@example.com")],
            cc: vec![EmailAddress::new(Some("CC User"), "cc@example.com")],
            bcc: vec![],
            subject: Some("Test Subject".to_string()),
            body_plain: Some("Plain body".to_string()),
            body_html: Some("<p>HTML body</p>".to_string()),
            in_reply_to: Some("<orig@id>".to_string()),
            references: vec!["<ref1@id>".to_string(), "ref2@id".to_string()],
            attachments: vec![MimeAttachment {
                filename: "note.txt".to_string(),
                content_type: "text/plain".to_string(),
                data: b"note".to_vec(),
            }],
        };

        let encoded = message.clone().to_base64_url().expect("build message");
        let raw_bytes = URL_SAFE_NO_PAD
            .decode(encoded.as_bytes())
            .expect("decode base64");
        let raw = String::from_utf8(raw_bytes).expect("utf8");

        assert!(raw.contains("sender@example.com"));
        assert!(raw.contains("to@example.com"));
        assert!(raw.contains("cc@example.com"));
        assert!(raw.contains("Test Subject"));
        assert!(raw.contains("In-Reply-To: <orig@id>"));
        assert!(raw.contains("<ref1@id>"));
        assert!(raw.contains("Content-Type: multipart/alternative"));
        assert!(raw.contains("Content-Type: text/plain"));
        assert!(raw.contains("Content-Type: text/html"));
        assert!(
            raw.contains("Content-Disposition: attachment; filename=\"note.txt\"")
                || raw.contains("Content-Disposition: attachment; filename=note.txt")
        );
    }

    #[test]
    fn errors_when_missing_recipients() {
        let message = MimeMessage {
            from: EmailAddress::from("sender@example.com"),
            to: vec![],
            cc: vec![],
            bcc: vec![],
            subject: None,
            body_plain: Some("Body".into()),
            body_html: None,
            in_reply_to: None,
            references: vec![],
            attachments: vec![],
        };

        let err = message.to_rfc822().expect_err("should fail");
        assert!(matches!(err, MimeBuildError::MissingRecipients));
    }

    #[test]
    fn errors_when_missing_body() {
        let message = MimeMessage {
            from: EmailAddress::from("sender@example.com"),
            to: vec![EmailAddress::from("to@example.com")],
            cc: vec![],
            bcc: vec![],
            subject: None,
            body_plain: None,
            body_html: None,
            in_reply_to: None,
            references: vec![],
            attachments: vec![],
        };

        let err = message.to_rfc822().expect_err("should fail");
        assert!(matches!(err, MimeBuildError::MissingBody));
    }

    #[test]
    fn normalize_message_id_strips_brackets_and_whitespace() {
        assert_eq!(
            normalize_message_id("<foo@bar.com>"),
            Some("foo@bar.com".to_string())
        );
        assert_eq!(
            normalize_message_id("  <foo@bar.com>  "),
            Some("foo@bar.com".to_string())
        );
        assert_eq!(
            normalize_message_id("foo@bar.com"),
            Some("foo@bar.com".to_string())
        );
        assert_eq!(
            normalize_message_id("  foo@bar.com  "),
            Some("foo@bar.com".to_string())
        );
    }

    #[test]
    fn normalize_message_id_returns_none_for_empty() {
        assert_eq!(normalize_message_id(""), None);
        assert_eq!(normalize_message_id("   "), None);
        assert_eq!(normalize_message_id("<>"), None);
        assert_eq!(normalize_message_id("  <>  "), None);
    }

    #[test]
    fn dedup_message_ids_removes_duplicates_preserving_order() {
        let ids = vec![
            "<a@b.com>".to_string(),
            "c@d.com".to_string(),
            "<a@b.com>".to_string(), // Duplicate
            "<e@f.com>".to_string(),
        ];
        let result = dedup_message_ids(ids);
        assert_eq!(
            result,
            vec![
                "a@b.com".to_string(),
                "c@d.com".to_string(),
                "e@f.com".to_string(),
            ]
        );
    }

    #[test]
    fn dedup_message_ids_normalizes_before_deduplicating() {
        // Same ID but different formatting should be deduplicated
        let ids = vec![
            "<msg@id.com>".to_string(),
            "msg@id.com".to_string(),
            "  <msg@id.com>  ".to_string(),
        ];
        let result = dedup_message_ids(ids);
        assert_eq!(result, vec!["msg@id.com".to_string()]);
    }

    #[test]
    fn dedup_message_ids_filters_empty_ids() {
        let ids = vec![
            "<a@b.com>".to_string(),
            "".to_string(),
            "<>".to_string(),
            "   ".to_string(),
            "<c@d.com>".to_string(),
        ];
        let result = dedup_message_ids(ids);
        assert_eq!(result, vec!["a@b.com".to_string(), "c@d.com".to_string(),]);
    }

    #[test]
    fn dedup_message_ids_handles_empty_input() {
        let ids: Vec<String> = vec![];
        let result = dedup_message_ids(ids);
        assert!(result.is_empty());
    }
}
