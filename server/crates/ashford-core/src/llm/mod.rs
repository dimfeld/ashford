pub mod decision;
pub mod error;
pub mod mock;
pub mod prompt;
pub mod repository;
pub mod types;

pub use decision::{
    ActionType, ConsideredAlternative, DecisionDetails, DecisionOutput, DecisionParseError,
    DecisionValidationError, Explanations, MessageRef, TelemetryPlaceholder, UndoHint,
};
pub use error::{LLMError, RateLimitInfo};
pub use mock::MockLLMClient;
pub use prompt::{build_decision_tool, PromptBuilder, PromptBuilderConfig, ThreadContext, DECISION_TOOL_NAME};
pub use repository::{LlmCall, LlmCallContext, LlmCallError, LlmCallRepository, NewLlmCall};
pub use types::{ChatMessage, ChatRole, CompletionRequest, CompletionResponse, Tool, ToolCall, ToolCallResult};

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use genai::{
    Client as GenaiClient, Error as GenaiError,
    chat::{
        ChatMessage as GenaiChatMessage, ChatOptions, ChatRequest, ChatResponse,
        ChatResponseFormat, MessageContent,
    },
    webc,
};
use reqwest::{
    StatusCode,
    header::{HeaderMap, HeaderValue, RETRY_AFTER},
};
use tracing::warn;

use crate::config::ModelConfig;
use crate::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
use crate::db::Database;

/// Minimal async interface for LLM clients used throughout the crate.
#[async_trait]
pub trait LLMClient: Send + Sync {
    async fn complete(
        &self,
        request: CompletionRequest,
        context: LlmCallContext,
    ) -> Result<CompletionResponse, LLMError>;
}

#[async_trait]
pub trait ChatExecutor: Send + Sync {
    async fn exec_chat(
        &self,
        model: &str,
        request: ChatRequest,
        options: Option<&ChatOptions>,
    ) -> Result<ChatResponse, GenaiError>;
}

#[async_trait]
impl ChatExecutor for GenaiClient {
    async fn exec_chat(
        &self,
        model: &str,
        request: ChatRequest,
        options: Option<&ChatOptions>,
    ) -> Result<ChatResponse, GenaiError> {
        GenaiClient::exec_chat(self, model, request, options).await
    }
}

/// Default LLM client backed by the genai crate.
pub struct GenaiLLMClient {
    chat: Arc<dyn ChatExecutor>,
    model: String,
    repo: LlmCallRepository,
}

impl GenaiLLMClient {
    pub fn new(db: Database, model_config: ModelConfig) -> Self {
        let chat: Arc<dyn ChatExecutor> = Arc::new(GenaiClient::default());
        Self::with_executor(db, model_config, chat)
    }

    pub fn with_executor(
        db: Database,
        model_config: ModelConfig,
        chat: Arc<dyn ChatExecutor>,
    ) -> Self {
        let model = namespaced_model(&model_config);
        Self {
            chat,
            model,
            repo: LlmCallRepository::new(db),
        }
    }

    fn build_chat_request(&self, request: &CompletionRequest) -> ChatRequest {
        let messages = request
            .messages
            .iter()
            .map(to_genai_message)
            .collect::<Vec<_>>();
        let mut chat_request = ChatRequest::from_messages(messages);

        if !request.tools.is_empty() {
            chat_request = chat_request.with_tools(request.tools.clone());
        }

        chat_request
    }

    fn build_chat_options(&self, request: &CompletionRequest) -> ChatOptions {
        let mut options = ChatOptions::default()
            .with_temperature(request.temperature as f64)
            .with_max_tokens(request.max_tokens);

        if request.json_mode {
            options = options.with_response_format(ChatResponseFormat::JsonMode);
        }

        options
    }

    async fn log_call(
        &self,
        context: &LlmCallContext,
        model: &str,
        request_json: serde_json::Value,
        response_json: Option<serde_json::Value>,
        input_tokens: Option<u32>,
        output_tokens: Option<u32>,
        latency_ms: Option<u64>,
        error: Option<String>,
    ) {
        let org_id = context.org_id.unwrap_or(DEFAULT_ORG_ID);
        let user_id = context.user_id.unwrap_or(DEFAULT_USER_ID);
        let mut context = context.clone();
        context.org_id = Some(org_id);
        context.user_id = Some(user_id);
        let new_call = NewLlmCall {
            org_id,
            user_id,
            context,
            model: model.to_string(),
            request_json,
            response_json,
            input_tokens,
            output_tokens,
            latency_ms,
            error: error.clone(),
            trace_id: None,
        };

        if let Err(log_err) = self.repo.create(new_call).await {
            warn!(error = ?log_err, "failed to record llm call");
        }
    }
}

#[async_trait]
impl LLMClient for GenaiLLMClient {
    async fn complete(
        &self,
        request: CompletionRequest,
        context: LlmCallContext,
    ) -> Result<CompletionResponse, LLMError> {
        let chat_request = self.build_chat_request(&request);
        let options = self.build_chat_options(&request);

        let request_json = serde_json::to_value(&request)
            .unwrap_or_else(|err| serde_json::json!({"error": err.to_string()}));

        let start = Instant::now();
        let result = self
            .chat
            .exec_chat(&self.model, chat_request, Some(&options))
            .await;
        let latency_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(response) => {
                let content = response.first_text().unwrap_or("").to_string();
                let provider_model = response.provider_model_iden.to_string();
                let (input_tokens, output_tokens) = usage_tokens(&response.usage);
                let response_json = serde_json::to_value(&response).ok();

                // Extract tool calls from the response
                let tool_calls = response
                    .tool_calls()
                    .iter()
                    .map(|tc| types::ToolCallResult {
                        call_id: tc.call_id.clone(),
                        fn_name: tc.fn_name.clone(),
                        fn_arguments: tc.fn_arguments.clone(),
                    })
                    .collect();

                self.log_call(
                    &context,
                    &provider_model,
                    request_json,
                    response_json.clone(),
                    Some(input_tokens),
                    Some(output_tokens),
                    Some(latency_ms),
                    None,
                )
                .await;

                Ok(CompletionResponse {
                    content,
                    model: provider_model,
                    input_tokens,
                    output_tokens,
                    latency_ms,
                    tool_calls,
                })
            }
            Err(err) => {
                let mapped = map_genai_error(err);
                self.log_call(
                    &context,
                    &self.model,
                    request_json,
                    None,
                    None,
                    None,
                    Some(latency_ms),
                    Some(mapped.to_string()),
                )
                .await;
                Err(mapped)
            }
        }
    }
}

fn to_genai_message(message: &ChatMessage) -> GenaiChatMessage {
    match message.role {
        ChatRole::System => GenaiChatMessage::system(text_content(&message.content)),
        ChatRole::User => GenaiChatMessage::user(text_content(&message.content)),
        ChatRole::Assistant => GenaiChatMessage::assistant(text_content(&message.content)),
    }
}

fn text_content(content: &str) -> MessageContent {
    MessageContent::from_text(content.to_string())
}

fn namespaced_model(cfg: &ModelConfig) -> String {
    if cfg.provider.is_empty() {
        cfg.model.clone()
    } else {
        format!("{}::{}", cfg.provider.to_lowercase(), cfg.model)
    }
}

fn usage_tokens(usage: &genai::chat::Usage) -> (u32, u32) {
    let input = usage.prompt_tokens.unwrap_or_default().max(0) as u32;
    let output = usage.completion_tokens.unwrap_or_default().max(0) as u32;
    (input, output)
}

fn map_genai_error(err: GenaiError) -> LLMError {
    match err {
        GenaiError::RequiresApiKey { .. }
        | GenaiError::NoAuthResolver { .. }
        | GenaiError::NoAuthData { .. } => LLMError::AuthenticationFailed,
        GenaiError::ChatReqHasNoMessages { .. }
        | GenaiError::LastChatMessageIsNotUser { .. }
        | GenaiError::MessageRoleNotSupported { .. }
        | GenaiError::MessageContentTypeNotSupported { .. }
        | GenaiError::JsonModeWithoutInstruction
        | GenaiError::VerbosityParsing { .. }
        | GenaiError::ReasoningParsingError { .. }
        | GenaiError::ServiceTierParsing { .. }
        | GenaiError::ModelMapperFailed { .. }
        | GenaiError::AdapterNotSupported { .. }
        | GenaiError::Resolver { .. } => LLMError::InvalidRequest(err.to_string()),
        GenaiError::InvalidJsonResponseElement { .. } | GenaiError::StreamParse { .. } => {
            LLMError::ParseError(err.to_string())
        }
        GenaiError::NoChatResponse { .. } => LLMError::ServerError(err.to_string()),
        GenaiError::WebAdapterCall { webc_error, .. }
        | GenaiError::WebModelCall { webc_error, .. } => map_webc_error(webc_error),
        GenaiError::ChatResponse { .. } | GenaiError::WebStream { .. } => {
            LLMError::ProviderError(err.to_string())
        }
        GenaiError::Internal(msg) => LLMError::ProviderError(msg),
        GenaiError::EventSourceClone(e) => LLMError::ProviderError(e.to_string()),
        GenaiError::JsonValueExt(e) => LLMError::ParseError(e.to_string()),
        GenaiError::ReqwestEventSource(err) => LLMError::ProviderError(err.to_string()),
        GenaiError::SerdeJson(err) => LLMError::ParseError(err.to_string()),
    }
}

fn map_webc_error(err: webc::Error) -> LLMError {
    match &err {
        webc::Error::ResponseFailedStatus {
            status, headers, ..
        } => {
            let retry_after_ms = retry_after_ms_from_headers(headers);
            match *status {
                StatusCode::TOO_MANY_REQUESTS | StatusCode::FORBIDDEN => {
                    LLMError::RateLimited(RateLimitInfo::new(retry_after_ms))
                }
                StatusCode::UNAUTHORIZED => LLMError::AuthenticationFailed,
                status if status.is_client_error() => LLMError::InvalidRequest(status.to_string()),
                status if status.is_server_error() => LLMError::ServerError(status.to_string()),
                status => LLMError::ProviderError(status.to_string()),
            }
        }
        webc::Error::Reqwest(req_err) => {
            if req_err.is_timeout() {
                LLMError::Timeout
            } else {
                LLMError::ProviderError(req_err.to_string())
            }
        }
        webc::Error::ResponseFailedNotJson { .. } => LLMError::ParseError(err.to_string()),
        webc::Error::JsonValueExt(parse_err) => LLMError::ParseError(parse_err.to_string()),
        webc::Error::EventSourceClone(clone_err) => LLMError::ProviderError(clone_err.to_string()),
    }
}

fn retry_after_ms_from_headers(headers: &HeaderMap) -> Option<u64> {
    if let Some(value) = headers.get(RETRY_AFTER) {
        if let Some(ms) = parse_retry_after(value) {
            return Some(ms);
        }
    }

    headers.get("x-ratelimit-reset").and_then(parse_epoch_reset)
}

fn parse_retry_after(value: &HeaderValue) -> Option<u64> {
    let raw = value.to_str().ok()?.trim();
    if let Ok(secs) = raw.parse::<u64>() {
        return Some(secs.saturating_mul(1000));
    }

    // HTTP-date format
    if let Ok(dt) = DateTime::parse_from_rfc2822(raw) {
        let now = Utc::now();
        let delta_ms = (dt.with_timezone(&Utc) - now).num_milliseconds();
        if delta_ms > 0 {
            return Some(delta_ms as u64);
        }
    }

    None
}

fn parse_epoch_reset(value: &HeaderValue) -> Option<u64> {
    let raw = value.to_str().ok()?.trim();
    let reset_epoch = raw.parse::<i64>().ok()?;
    let now_epoch = Utc::now().timestamp();
    let delta_ms = (reset_epoch - now_epoch).saturating_mul(1000);
    (delta_ms > 0).then_some(delta_ms as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
    use crate::llm::error::RateLimitInfo;
    use crate::migrations::run_migrations;
    use chrono::Utc;
    use genai::{ModelIden, adapter::AdapterKind, chat::Usage};
    use reqwest::{
        StatusCode,
        header::{HeaderMap, HeaderValue, RETRY_AFTER},
    };
    use std::sync::Mutex;
    use tempfile::TempDir;

    fn test_model_config() -> ModelConfig {
        ModelConfig {
            provider: "OpenAI".into(),
            model: "gpt-4o-mini".into(),
            temperature: 0.2,
            max_output_tokens: 256,
        }
    }

    #[test]
    fn namespaced_model_handles_provider_casing() {
        let model = namespaced_model(&test_model_config());
        assert_eq!(model, "openai::gpt-4o-mini");
    }

    #[test]
    fn namespaced_model_without_provider_returns_model() {
        let mut cfg = test_model_config();
        cfg.provider.clear();
        let model = namespaced_model(&cfg);
        assert_eq!(model, "gpt-4o-mini");
    }

    #[test]
    fn usage_tokens_defaults_and_clamps() {
        let mut usage = Usage::default();
        assert_eq!(usage_tokens(&usage), (0, 0));

        usage.prompt_tokens = Some(-5);
        usage.completion_tokens = Some(7);
        assert_eq!(usage_tokens(&usage), (0, 7));
    }

    #[tokio::test]
    async fn build_chat_request_converts_messages() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("db.sqlite");
        let db = Database::new(&db_path).await.expect("db");
        let client = GenaiLLMClient {
            chat: Arc::new(GenaiClient::default()),
            model: "openai::gpt-4o-mini".into(),
            repo: LlmCallRepository::new(db),
        };

        let request = CompletionRequest {
            messages: vec![
                ChatMessage {
                    role: ChatRole::System,
                    content: "system".into(),
                },
                ChatMessage {
                    role: ChatRole::User,
                    content: "hi there".into(),
                },
            ],
            temperature: 0.1,
            max_tokens: 32,
            json_mode: false,
            tools: vec![],
        };

        let built = client.build_chat_request(&request);
        assert_eq!(built.messages.len(), 2);
        assert!(matches!(
            built.messages[0].role,
            genai::chat::ChatRole::System
        ));
        assert_eq!(built.messages[0].content.first_text(), Some("system"));
        assert!(matches!(
            built.messages[1].role,
            genai::chat::ChatRole::User
        ));
        assert_eq!(built.messages[1].content.first_text(), Some("hi there"));
    }

    #[tokio::test]
    async fn build_chat_options_sets_temperature_tokens_and_json_mode() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("db.sqlite");
        let db = Database::new(&db_path).await.expect("db");
        let client = GenaiLLMClient {
            chat: Arc::new(GenaiClient::default()),
            model: "openai::gpt-4o-mini".into(),
            repo: LlmCallRepository::new(db),
        };

        let request = CompletionRequest {
            messages: vec![],
            temperature: 0.42,
            max_tokens: 128,
            json_mode: true,
            tools: vec![],
        };

        let options = client.build_chat_options(&request);
        assert_eq!(options.max_tokens, Some(128));
        assert!(
            (options.temperature.unwrap() - 0.42).abs() < 1e-6,
            "temperature should be propagated"
        );
        assert!(matches!(
            options.response_format,
            Some(ChatResponseFormat::JsonMode)
        ));
    }

    #[test]
    fn map_genai_error_maps_categories() {
        let model_iden = ModelIden::from((genai::adapter::AdapterKind::OpenAI, "gpt-4o-mini"));

        let rate_limit = GenaiError::WebModelCall {
            model_iden: model_iden.clone(),
            webc_error: webc::Error::ResponseFailedStatus {
                status: StatusCode::TOO_MANY_REQUESTS,
                body: String::new(),
                headers: Box::new(HeaderMap::new()),
            },
        };
        assert!(matches!(
            map_genai_error(rate_limit),
            LLMError::RateLimited(_)
        ));

        let invalid = GenaiError::ChatReqHasNoMessages {
            model_iden: model_iden.clone(),
        };
        assert!(matches!(
            map_genai_error(invalid),
            LLMError::InvalidRequest(_)
        ));

        let parse = GenaiError::InvalidJsonResponseElement { info: "bad" };
        assert!(matches!(map_genai_error(parse), LLMError::ParseError(_)));

        let auth = GenaiError::RequiresApiKey {
            model_iden: model_iden.clone(),
        };
        assert!(matches!(
            map_genai_error(auth),
            LLMError::AuthenticationFailed
        ));

        let provider = GenaiError::ChatResponse {
            model_iden,
            body: serde_json::json!({"error": "oops"}),
        };
        assert!(matches!(
            map_genai_error(provider),
            LLMError::ProviderError(_)
        ));
    }

    #[test]
    fn map_webc_error_classifies_status_codes() {
        let rate = webc::Error::ResponseFailedStatus {
            status: StatusCode::TOO_MANY_REQUESTS,
            body: String::new(),
            headers: Box::new(HeaderMap::new()),
        };
        assert!(matches!(map_webc_error(rate), LLMError::RateLimited(_)));

        let forbidden = webc::Error::ResponseFailedStatus {
            status: StatusCode::FORBIDDEN,
            body: String::new(),
            headers: Box::new(HeaderMap::new()),
        };
        assert!(matches!(
            map_webc_error(forbidden),
            LLMError::RateLimited(_)
        ));

        let client = webc::Error::ResponseFailedStatus {
            status: StatusCode::BAD_REQUEST,
            body: String::new(),
            headers: Box::new(HeaderMap::new()),
        };
        assert!(matches!(
            map_webc_error(client),
            LLMError::InvalidRequest(_)
        ));

        let server = webc::Error::ResponseFailedStatus {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            body: String::new(),
            headers: Box::new(HeaderMap::new()),
        };
        assert!(matches!(map_webc_error(server), LLMError::ServerError(_)));

        let parse = webc::Error::ResponseFailedNotJson {
            content_type: "text/plain".into(),
        };
        assert!(matches!(map_webc_error(parse), LLMError::ParseError(_)));
    }

    #[tokio::test]
    async fn log_call_records_defaults_and_payload() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("db.sqlite");
        let db = Database::new(&db_path).await.expect("db");
        run_migrations(&db).await.expect("migrations");

        let client = GenaiLLMClient::new(db.clone(), test_model_config());

        let context = LlmCallContext::new("classification");
        client
            .log_call(
                &context,
                &client.model,
                serde_json::json!({"msg": "hi"}),
                None,
                Some(3),
                Some(4),
                Some(50),
                Some("boom".into()),
            )
            .await;

        let calls = client
            .repo
            .list(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                Some("classification"),
                Some(10),
            )
            .await
            .expect("list");
        assert_eq!(calls.len(), 1);

        let call = &calls[0];
        assert_eq!(call.org_id, DEFAULT_ORG_ID);
        assert_eq!(call.user_id, DEFAULT_USER_ID);
        assert_eq!(call.feature, "classification");
        assert_eq!(call.context.org_id, Some(DEFAULT_ORG_ID));
        assert_eq!(call.context.user_id, Some(DEFAULT_USER_ID));
        assert_eq!(call.request_json["msg"], "hi");
        assert_eq!(call.input_tokens, Some(3));
        assert_eq!(call.output_tokens, Some(4));
        assert_eq!(call.latency_ms, Some(50));
        assert_eq!(call.error.as_deref(), Some("boom"));
        assert!(call.created_at <= Utc::now());
    }

    #[derive(Default)]
    struct StubChatExecutor {
        responses: Mutex<Vec<Result<ChatResponse, GenaiError>>>,
        calls: Mutex<Vec<(String, ChatRequest, Option<ChatOptions>)>>,
    }

    impl StubChatExecutor {
        fn new(response: Result<ChatResponse, GenaiError>) -> Self {
            Self {
                responses: Mutex::new(vec![response]),
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl ChatExecutor for StubChatExecutor {
        async fn exec_chat(
            &self,
            model: &str,
            request: ChatRequest,
            options: Option<&ChatOptions>,
        ) -> Result<ChatResponse, GenaiError> {
            self.calls.lock().expect("calls").push((
                model.to_string(),
                request.clone(),
                options.cloned(),
            ));

            self.responses
                .lock()
                .expect("responses")
                .pop()
                .unwrap_or_else(|| Err(GenaiError::Internal("stub missing response".into())))
        }
    }

    #[tokio::test]
    async fn complete_logs_successful_call() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("db.sqlite");
        let db = Database::new(&db_path).await.expect("db");
        run_migrations(&db).await.expect("migrations");

        let response = ChatResponse {
            content: MessageContent::from_text("ok"),
            reasoning_content: None,
            model_iden: ModelIden::new(AdapterKind::OpenAI, "gpt-4o-mini"),
            provider_model_iden: ModelIden::new(AdapterKind::OpenAI, "gpt-4o-mini"),
            usage: Usage {
                prompt_tokens: Some(5),
                completion_tokens: Some(7),
                total_tokens: None,
                ..Default::default()
            },
            captured_raw_body: None,
        };

        let expected_model = response.provider_model_iden.to_string();
        let stub = Arc::new(StubChatExecutor::new(Ok(response)));
        let client = GenaiLLMClient::with_executor(db.clone(), test_model_config(), stub.clone());

        let request = CompletionRequest {
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: "hello".into(),
            }],
            temperature: 0.5,
            max_tokens: 64,
            json_mode: true,
            tools: vec![],
        };
        let context = LlmCallContext {
            feature: "classification".into(),
            org_id: Some(123),
            user_id: Some(456),
            ..Default::default()
        };

        let completion = client
            .complete(request.clone(), context.clone())
            .await
            .expect("completion");
        assert_eq!(completion.content, "ok");
        assert_eq!(completion.input_tokens, 5);
        assert_eq!(completion.output_tokens, 7);
        assert_eq!(completion.model, expected_model);
        assert!(completion.tool_calls.is_empty());

        let calls = client
            .repo
            .list(123, 456, Some("classification"), Some(1))
            .await
            .expect("list");
        assert_eq!(calls.len(), 1);
        let call = &calls[0];
        assert!(call.response_json.is_some());
        assert!(call.error.is_none());
        assert_eq!(call.context.org_id, Some(123));
        assert_eq!(call.context.user_id, Some(456));
        assert_eq!(call.model, expected_model, "should log provider model iden");
        assert_eq!(call.request_json["messages"][0]["content"], "hello");

        let recorded = stub.calls.lock().expect("calls");
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].0, "openai::gpt-4o-mini");
        let options = recorded[0].2.as_ref().expect("options recorded");
        assert_eq!(options.max_tokens, Some(request.max_tokens));
        assert!(
            (options.temperature.unwrap() - request.temperature as f64).abs() < 1e-6,
            "temperature should match"
        );
        assert!(matches!(
            options.response_format,
            Some(ChatResponseFormat::JsonMode)
        ));
    }

    #[tokio::test]
    async fn complete_logs_rate_limit_with_retry_after() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("db.sqlite");
        let db = Database::new(&db_path).await.expect("db");
        run_migrations(&db).await.expect("migrations");

        let mut headers = HeaderMap::new();
        headers.insert(RETRY_AFTER, HeaderValue::from_static("2"));
        let rate_limit_error = GenaiError::WebModelCall {
            model_iden: ModelIden::new(AdapterKind::OpenAI, "gpt-4o-mini"),
            webc_error: webc::Error::ResponseFailedStatus {
                status: StatusCode::TOO_MANY_REQUESTS,
                body: String::new(),
                headers: Box::new(headers),
            },
        };

        let stub = Arc::new(StubChatExecutor::new(Err(rate_limit_error)));
        let client = GenaiLLMClient::with_executor(db.clone(), test_model_config(), stub.clone());

        let request = CompletionRequest {
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: "hi".into(),
            }],
            temperature: 0.0,
            max_tokens: 16,
            json_mode: false,
            tools: vec![],
        };
        let context = LlmCallContext::new("classification");

        let result = client.complete(request.clone(), context.clone()).await;
        match result {
            Err(LLMError::RateLimited(RateLimitInfo { retry_after_ms })) => {
                assert_eq!(retry_after_ms, Some(2000))
            }
            other => panic!("expected rate limited error, got {other:?}"),
        }

        let calls = client
            .repo
            .list(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                Some("classification"),
                Some(1),
            )
            .await
            .expect("list");
        assert_eq!(calls.len(), 1);
        let call = &calls[0];
        assert_eq!(
            call.error.as_deref(),
            Some("rate limited (retry after 2000ms)")
        );
        assert!(call.response_json.is_none());
        assert_eq!(call.context.org_id, Some(DEFAULT_ORG_ID));
        assert_eq!(call.context.user_id, Some(DEFAULT_USER_ID));

        let recorded = stub.calls.lock().expect("calls");
        assert_eq!(recorded.len(), 1, "stub should capture call");
    }
}
