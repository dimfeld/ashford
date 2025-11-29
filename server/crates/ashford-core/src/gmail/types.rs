use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
