use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompletionRequest {
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub json_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompletionResponse {
    pub content: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub latency_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, json, to_value};

    #[test]
    fn chat_role_serializes_to_lowercase() {
        assert_eq!(to_value(ChatRole::System).unwrap(), json!("system"));
        assert_eq!(to_value(ChatRole::User).unwrap(), json!("user"));
        assert_eq!(to_value(ChatRole::Assistant).unwrap(), json!("assistant"));
    }

    #[test]
    fn chat_role_deserializes_from_lowercase_strings() {
        assert_eq!(
            from_str::<ChatRole>("\"system\"").unwrap(),
            ChatRole::System
        );
        assert_eq!(from_str::<ChatRole>("\"user\"").unwrap(), ChatRole::User);
        assert_eq!(
            from_str::<ChatRole>("\"assistant\"").unwrap(),
            ChatRole::Assistant
        );
    }

    #[test]
    fn chat_message_round_trips_through_json() {
        let message = ChatMessage {
            role: ChatRole::User,
            content: "Hello".to_string(),
        };

        let value = to_value(&message).expect("serialize");
        assert_eq!(value, json!({"role": "user", "content": "Hello"}));

        let decoded: ChatMessage = serde_json::from_value(value).expect("deserialize");
        assert_eq!(decoded, message);
    }

    #[test]
    fn completion_request_serializes_expected_shape() {
        let request = CompletionRequest {
            messages: vec![ChatMessage {
                role: ChatRole::System,
                content: "Be concise".to_string(),
            }],
            temperature: 0.7,
            max_tokens: 256,
            json_mode: true,
        };

        let value = to_value(&request).expect("serialize");

        let expected = json!({
            "messages": [{"role": "system", "content": "Be concise"}],
            "temperature": value["temperature"].clone(),
            "max_tokens": 256,
            "json_mode": true
        });

        assert_eq!(value, expected);
        assert!((value["temperature"].as_f64().unwrap() - 0.7).abs() < 1e-6);

        let decoded: CompletionRequest = serde_json::from_value(value).expect("deserialize");
        assert_eq!(decoded, request);
    }

    #[test]
    fn completion_response_round_trips_through_json() {
        let response = CompletionResponse {
            content: "ok".to_string(),
            model: "gpt-4o".to_string(),
            input_tokens: 42,
            output_tokens: 7,
            latency_ms: 1234,
        };

        let value = to_value(&response).expect("serialize");
        let decoded: CompletionResponse = serde_json::from_value(value).expect("deserialize");
        assert_eq!(decoded, response);
    }
}
