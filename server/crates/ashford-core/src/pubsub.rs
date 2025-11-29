use std::str;

use base64::Engine as _;
use google_cloud_auth::credentials::CredentialsFile;
use google_cloud_googleapis::pubsub::v1::PubsubMessage;
use google_cloud_pubsub::client::{Client, ClientConfig};
use serde::Deserialize;
use thiserror::Error;

use crate::accounts::AccountError;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GmailNotification {
    #[serde(rename = "emailAddress")]
    pub email_address: String,
    #[serde(rename = "historyId")]
    pub history_id: String,
}

#[derive(Debug, Error)]
pub enum PubsubError {
    #[error("credentials error: {0}")]
    Credentials(#[from] google_cloud_auth::error::Error),
    #[error("pubsub client error: {0}")]
    Client(#[from] google_cloud_pubsub::client::Error),
    #[error("pubsub status error: {0}")]
    Status(#[from] google_cloud_gax::grpc::Status),
    #[error("base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("utf8 decode error: {0}")]
    Utf8(#[from] str::Utf8Error),
    #[error("json decode error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("account error: {0}")]
    Account(#[from] AccountError),
    #[error("queue error: {0}")]
    Queue(#[from] crate::queue::QueueError),
}

/// Create a Pub/Sub client authenticated with the provided service account JSON.
pub async fn subscriber_client_from_service_account(
    service_account_json: &str,
) -> Result<Client, PubsubError> {
    let credentials = CredentialsFile::new_from_str(service_account_json).await?;
    let config = ClientConfig::default()
        .with_credentials(credentials)
        .await?;
    let client = Client::new(config).await?;
    Ok(client)
}

/// Decode a Gmail Pub/Sub notification payload into a strongly typed struct.
pub fn parse_gmail_notification(message: &PubsubMessage) -> Result<GmailNotification, PubsubError> {
    // Gmail places a base64-encoded JSON object in the message data field.
    let data_str = str::from_utf8(&message.data)?;
    // Gmail Pub/Sub uses URL-safe base64 without padding.
    let decoded = base64::prelude::BASE64_URL_SAFE_NO_PAD.decode(data_str)?;
    let notification: GmailNotification = serde_json::from_slice(&decoded)?;
    Ok(notification)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::prelude::*;
    use google_cloud_googleapis::pubsub::v1::PubsubMessage;

    #[test]
    fn parses_valid_gmail_notification() {
        let payload = serde_json::json!({
            "emailAddress": "user@example.com",
            "historyId": "12345"
        })
        .to_string();
        let encoded = BASE64_URL_SAFE_NO_PAD.encode(payload.as_bytes());

        let message = PubsubMessage {
            data: encoded.as_bytes().to_vec(),
            ..Default::default()
        };

        let parsed = parse_gmail_notification(&message).expect("parse succeeds");
        assert_eq!(parsed.email_address, "user@example.com");
        assert_eq!(parsed.history_id, "12345");
    }

    #[test]
    fn rejects_invalid_base64_payload() {
        let message = PubsubMessage {
            data: b"not-base64".to_vec(),
            ..Default::default()
        };

        let err = parse_gmail_notification(&message).expect_err("should fail to decode");
        assert!(matches!(err, PubsubError::Base64(_)));
    }

    #[test]
    fn rejects_invalid_json_after_decode() {
        let encoded = BASE64_URL_SAFE_NO_PAD.encode(b"not-json");
        let message = PubsubMessage {
            data: encoded.as_bytes().to_vec(),
            ..Default::default()
        };

        let err = parse_gmail_notification(&message).expect_err("should fail to parse json");
        assert!(matches!(err, PubsubError::Json(_)));
    }
}
