use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use super::{CompletionRequest, CompletionResponse, LLMClient, LLMError, LlmCallContext};

#[derive(Debug, Default, Clone)]
pub struct MockLLMClient {
    responses: Arc<Mutex<VecDeque<Result<CompletionResponse, LLMError>>>>,
}

impl MockLLMClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enqueue_response(&self, response: Result<CompletionResponse, LLMError>) {
        let mut guard = self.responses.lock().expect("lock responses");
        guard.push_back(response);
    }
}

#[async_trait]
impl LLMClient for MockLLMClient {
    async fn complete(
        &self,
        _request: CompletionRequest,
        _context: LlmCallContext,
    ) -> Result<CompletionResponse, LLMError> {
        let mut guard = self.responses.lock().expect("lock responses");
        guard.pop_front().unwrap_or_else(|| {
            Err(LLMError::ProviderError(
                "mock response not provided".to_string(),
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_enqueued_responses_in_order() {
        let mock = MockLLMClient::new();
        let response_one = CompletionResponse {
            content: "first".into(),
            model: "model-a".into(),
            input_tokens: 10,
            output_tokens: 2,
            latency_ms: 50,
            tool_calls: vec![],
        };
        let response_two = CompletionResponse {
            content: "second".into(),
            model: "model-b".into(),
            input_tokens: 20,
            output_tokens: 4,
            latency_ms: 75,
            tool_calls: vec![],
        };

        mock.enqueue_response(Ok(response_one.clone()));
        mock.enqueue_response(Err(LLMError::Timeout));
        mock.enqueue_response(Ok(response_two.clone()));

        let request = CompletionRequest {
            messages: vec![],
            temperature: 0.0,
            max_tokens: 0,
            json_mode: false,
            tools: vec![],
        };
        let context = LlmCallContext::new("test");

        assert_eq!(
            mock.complete(request.clone(), context.clone())
                .await
                .unwrap(),
            response_one
        );
        assert!(matches!(
            mock.complete(request.clone(), context.clone()).await,
            Err(LLMError::Timeout)
        ));
        assert_eq!(mock.complete(request, context).await.unwrap(), response_two);
    }

    #[tokio::test]
    async fn returns_error_when_queue_empty() {
        let mock = MockLLMClient::new();
        let request = CompletionRequest {
            messages: vec![],
            temperature: 0.0,
            max_tokens: 0,
            json_mode: false,
            tools: vec![],
        };
        let context = LlmCallContext::new("test");

        let result = mock.complete(request, context).await;
        assert!(
            matches!(result, Err(LLMError::ProviderError(msg)) if msg.contains("mock response not provided"))
        );
    }
}
