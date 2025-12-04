use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Minimal message stub returned by list endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageId {
    pub id: String,
    #[serde(rename = "threadId")]
    pub thread_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessagePartBody {
    pub size: i64,
    pub data: Option<String>,
    #[serde(rename = "attachmentId")]
    pub attachment_id: Option<String>,
}

/// Attachment payload returned by the Gmail attachments.get endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageAttachment {
    #[serde(rename = "attachmentId", default)]
    pub attachment_id: Option<String>,
    pub size: Option<i64>,
    /// Base64url-encoded binary data of the attachment.
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct Header {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessagePart {
    #[serde(rename = "partId")]
    pub part_id: Option<String>,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    pub filename: Option<String>,
    #[serde(default)]
    pub headers: Vec<Header>,
    pub body: Option<MessagePartBody>,
    #[serde(default)]
    pub parts: Vec<MessagePart>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
    pub id: String,
    #[serde(rename = "threadId")]
    pub thread_id: Option<String>,
    #[serde(rename = "labelIds", default)]
    pub label_ids: Vec<String>,
    pub snippet: Option<String>,
    #[serde(rename = "historyId")]
    pub history_id: Option<String>,
    #[serde(rename = "internalDate")]
    pub internal_date: Option<String>,
    pub payload: Option<MessagePart>,
    #[serde(rename = "sizeEstimate")]
    pub size_estimate: Option<u64>,
    pub raw: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Thread {
    pub id: String,
    pub snippet: Option<String>,
    #[serde(rename = "historyId")]
    pub history_id: Option<String>,
    #[serde(default)]
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryMessageChange {
    pub message: MessageId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryLabelChange {
    pub message: MessageId,
    #[serde(rename = "labelIds", default)]
    pub label_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryRecord {
    pub id: String,
    #[serde(rename = "messages")]
    pub messages: Option<Vec<MessageId>>,
    #[serde(rename = "messagesAdded")]
    pub messages_added: Option<Vec<HistoryMessageChange>>,
    #[serde(rename = "messagesDeleted")]
    pub messages_deleted: Option<Vec<HistoryMessageChange>>,
    #[serde(rename = "labelsAdded")]
    pub labels_added: Option<Vec<HistoryLabelChange>>,
    #[serde(rename = "labelsRemoved")]
    pub labels_removed: Option<Vec<HistoryLabelChange>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListHistoryResponse {
    #[serde(default)]
    pub history: Vec<HistoryRecord>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
    #[serde(rename = "historyId")]
    pub history_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListMessagesResponse {
    #[serde(default)]
    pub messages: Vec<MessageId>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
    #[serde(rename = "resultSizeEstimate")]
    pub result_size_estimate: Option<u64>,
}

/// Response from the Gmail Users.profile endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Profile {
    #[serde(rename = "emailAddress")]
    pub email_address: String,
    #[serde(rename = "messagesTotal")]
    pub messages_total: Option<u64>,
    #[serde(rename = "threadsTotal")]
    pub threads_total: Option<u64>,
    #[serde(rename = "historyId")]
    pub history_id: String,
}

/// Color information for a Gmail label.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LabelColor {
    #[serde(rename = "backgroundColor")]
    pub background_color: Option<String>,
    #[serde(rename = "textColor")]
    pub text_color: Option<String>,
}

/// A Gmail label from the labels.list API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Label {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub label_type: Option<String>,
    #[serde(rename = "messageListVisibility")]
    pub message_list_visibility: Option<String>,
    #[serde(rename = "labelListVisibility")]
    pub label_list_visibility: Option<String>,
    pub color: Option<LabelColor>,
}

/// Response from the Gmail Users.labels.list endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListLabelsResponse {
    #[serde(default)]
    pub labels: Vec<Label>,
}

/// Request body for the Gmail Users.messages.modify endpoint.
/// Used to add or remove labels from a message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModifyMessageRequest {
    /// Label IDs to add to the message.
    #[serde(rename = "addLabelIds", skip_serializing_if = "Option::is_none")]
    pub add_label_ids: Option<Vec<String>>,
    /// Label IDs to remove from the message.
    #[serde(rename = "removeLabelIds", skip_serializing_if = "Option::is_none")]
    pub remove_label_ids: Option<Vec<String>>,
}

/// Request body for the Gmail Users.messages.send endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    /// Base64url-encoded RFC 5322 message.
    pub raw: String,
    /// Gmail thread ID to associate the sent message with (for replies).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
}

/// Response from the Gmail Users.messages.send endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageResponse {
    pub id: String,
    pub thread_id: String,
    #[serde(default)]
    pub label_ids: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modify_message_request_serializes_with_both_fields() {
        let request = ModifyMessageRequest {
            add_label_ids: Some(vec!["STARRED".into(), "Label_123".into()]),
            remove_label_ids: Some(vec!["UNREAD".into()]),
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "addLabelIds": ["STARRED", "Label_123"],
                "removeLabelIds": ["UNREAD"]
            })
        );
    }

    #[test]
    fn modify_message_request_omits_none_add_labels() {
        let request = ModifyMessageRequest {
            add_label_ids: None,
            remove_label_ids: Some(vec!["INBOX".into()]),
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "removeLabelIds": ["INBOX"]
            })
        );
        // Verify the field is not present at all
        assert!(!json.as_object().unwrap().contains_key("addLabelIds"));
    }

    #[test]
    fn modify_message_request_omits_none_remove_labels() {
        let request = ModifyMessageRequest {
            add_label_ids: Some(vec!["STARRED".into()]),
            remove_label_ids: None,
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "addLabelIds": ["STARRED"]
            })
        );
        // Verify the field is not present at all
        assert!(!json.as_object().unwrap().contains_key("removeLabelIds"));
    }

    #[test]
    fn modify_message_request_both_none_serializes_to_empty_object() {
        let request = ModifyMessageRequest {
            add_label_ids: None,
            remove_label_ids: None,
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json, serde_json::json!({}));
    }

    #[test]
    fn modify_message_request_deserializes_correctly() {
        let json = serde_json::json!({
            "addLabelIds": ["STARRED"],
            "removeLabelIds": ["UNREAD", "INBOX"]
        });
        let request: ModifyMessageRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.add_label_ids, Some(vec!["STARRED".to_string()]));
        assert_eq!(
            request.remove_label_ids,
            Some(vec!["UNREAD".to_string(), "INBOX".to_string()])
        );
    }

    #[test]
    fn modify_message_request_deserializes_with_missing_fields() {
        let json = serde_json::json!({
            "addLabelIds": ["Label_1"]
        });
        let request: ModifyMessageRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.add_label_ids, Some(vec!["Label_1".to_string()]));
        assert_eq!(request.remove_label_ids, None);
    }
}
