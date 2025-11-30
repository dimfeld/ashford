---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Phase 4.3: LLM Provider Integration"
goal: Integrate LLM provider abstraction supporting multiple backends (OpenAI,
  Google, Anthropic)
id: 15
uuid: 01e10898-4dba-4343-902f-cd5ab57178eb
status: pending
priority: high
container: false
temp: false
dependencies:
  - 13
parent: 4
issue: []
docs:
  - docs/decision_engine.md
createdAt: 2025-11-30T01:14:19.052Z
updatedAt: 2025-11-30T01:14:19.052Z
tasks: []
tags: []
---

Foundation for LLM-based classification. This layer handles provider selection, API calls, error handling, and basic telemetry.

## Key Components

### Provider Selection
Evaluate options:
- **Option A**: `genai` crate (mentioned in docs, multi-provider abstraction)
- **Option B**: Custom trait-based abstraction with per-provider implementations

Decision criteria: stability, provider support, token counting, streaming support

### LLM Client Trait
```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LLMError>;
    fn name(&self) -> &str;
}

pub struct CompletionRequest {
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub json_mode: bool,  // Request JSON output
}

pub struct CompletionResponse {
    pub content: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub latency_ms: u64,
}
```

### Error Handling
Map provider errors to appropriate handling:
- Rate limits (429) → `JobError::Retryable`
- Auth errors (401) → `JobError::Retryable` (after refresh attempt)
- Invalid request (400) → `JobError::Fatal`
- Server errors (5xx) → `JobError::Retryable`
- Timeout → `JobError::Retryable`

### Configuration Integration
Use existing `ModelConfig` from config.rs:
```rust
pub struct ModelConfig {
    pub provider: String,      // "openai", "google", "anthropic"
    pub model: String,         // "gpt-4", "gemini-1.5-pro", "claude-3"
    pub temperature: f32,
    pub max_output_tokens: u32,
}
```

### Retry Logic
- Exponential backoff for retryable errors
- Configurable max retries
- Respect provider rate limit headers if present

### File Organization
```
ashford-core/src/llm/
├── mod.rs
├── provider.rs      # Trait definition and factory
├── openai.rs        # OpenAI implementation (if custom)
├── google.rs        # Google/Gemini implementation (if custom)
├── anthropic.rs     # Anthropic implementation (if custom)
```

### Testing
- Mock provider for unit tests
- Integration tests with real providers (optional, behind feature flag)
- Error handling tests for various failure modes
