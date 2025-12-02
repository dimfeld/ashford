#![cfg(feature = "llm-integration")]

use ashford_core::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
use ashford_core::llm::{
    ChatMessage, ChatRole, CompletionRequest, GenaiLLMClient, LlmCallContext, LlmCallRepository,
};
use ashford_core::migrations::run_migrations;
use ashford_core::{Database, LLMClient, config::ModelConfig};
use tempfile::TempDir;

fn has_required_env() -> bool {
    std::env::var("OPENAI_API_KEY").is_ok()
}

fn integration_model() -> String {
    std::env::var("LLM_INTEGRATION_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string())
}

#[tokio::test]
async fn openai_completion_logs_and_returns_content() -> Result<(), Box<dyn std::error::Error>> {
    if !has_required_env() {
        eprintln!("skipping llm integration test: OPENAI_API_KEY not set");
        return Ok(());
    }

    let dir = TempDir::new()?;
    let db_path = dir.path().join("db.sqlite");
    let db = Database::new(&db_path).await?;
    run_migrations(&db).await?;

    let model_config = ModelConfig {
        provider: "openai".into(),
        model: integration_model(),
        temperature: 0.0,
        max_output_tokens: 32,
    };

    let client = GenaiLLMClient::new(db.clone(), model_config);

    let request = CompletionRequest {
        messages: vec![
            ChatMessage {
                role: ChatRole::System,
                content: "You are a test harness. Reply with the single word 'pong'.".into(),
            },
            ChatMessage {
                role: ChatRole::User,
                content: "say it now".into(),
            },
        ],
        temperature: 0.0,
        max_tokens: 8,
        json_mode: false,
    };

    let context = LlmCallContext::new("llm_integration");
    let response = client.complete(request, context.clone()).await?;

    let content = response.content.trim().to_lowercase();
    assert!(content.contains("pong"), "model response: {}", content);
    assert!(response.latency_ms > 0);
    assert!(
        response.input_tokens > 0,
        "expected input_tokens to be counted"
    );
    assert!(
        response.output_tokens > 0,
        "expected output_tokens to be counted"
    );

    let repo = LlmCallRepository::new(db.clone());
    let calls = repo
        .list(
            DEFAULT_ORG_ID,
            DEFAULT_USER_ID,
            Some("llm_integration"),
            Some(1),
        )
        .await?;
    assert!(
        !calls.is_empty(),
        "expected llm call to be logged for feature llm_integration"
    );
    let call = &calls[0];
    assert_eq!(call.feature, "llm_integration");
    assert!(call.response_json.is_some());
    assert!(call.error.is_none());
    assert_eq!(call.model, response.model);
    assert_eq!(call.context.org_id, Some(DEFAULT_ORG_ID));
    assert_eq!(call.context.user_id, Some(DEFAULT_USER_ID));
    assert_eq!(call.input_tokens, Some(response.input_tokens));
    assert_eq!(call.output_tokens, Some(response.output_tokens));

    Ok(())
}
