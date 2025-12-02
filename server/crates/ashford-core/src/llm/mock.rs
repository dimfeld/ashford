use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use super::{CompletionRequest, CompletionResponse, LLMClient, LLMError, LlmCallContext};

#[derive(Debug, Default, Clone)]
pub struct MockLLMClient {
    responses: Arc<Mutex<VecDeque<Result<CompletionResponse, LLMError>>>>,
    call_count: Arc<AtomicUsize>,
}

impl MockLLMClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enqueue_response(&self, response: Result<CompletionResponse, LLMError>) {
        let mut guard = self.responses.lock().expect("lock responses");
        guard.push_back(response);
    }

    /// Returns the number of times `complete` has been called.
    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl LLMClient for MockLLMClient {
    async fn complete(
        &self,
        _request: CompletionRequest,
        _context: LlmCallContext,
    ) -> Result<CompletionResponse, LLMError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
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

    #[tokio::test]
    async fn call_count_tracks_invocations() {
        let mock = MockLLMClient::new();
        let response = CompletionResponse {
            content: "ok".into(),
            model: "model".into(),
            input_tokens: 1,
            output_tokens: 1,
            latency_ms: 10,
            tool_calls: vec![],
        };
        mock.enqueue_response(Ok(response.clone()));
        mock.enqueue_response(Ok(response));

        let request = CompletionRequest {
            messages: vec![],
            temperature: 0.0,
            max_tokens: 0,
            json_mode: false,
            tools: vec![],
        };
        let context = LlmCallContext::new("test");

        assert_eq!(mock.call_count(), 0);
        let _ = mock.complete(request.clone(), context.clone()).await;
        assert_eq!(mock.call_count(), 1);
        let _ = mock.complete(request, context).await;
        assert_eq!(mock.call_count(), 2);
    }
}
