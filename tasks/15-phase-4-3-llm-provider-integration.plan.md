---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Phase 4.3: LLM Provider Integration"
goal: Integrate LLM provider abstraction supporting multiple backends (OpenAI,
  Google, Anthropic)
id: 15
uuid: 01e10898-4dba-4343-902f-cd5ab57178eb
generatedBy: agent
status: done
priority: high
container: false
temp: false
dependencies:
  - 13
parent: 4
references: {}
issue: []
pullRequest: []
docs:
  - docs/decision_engine.md
planGeneratedAt: 2025-12-01T22:29:12.212Z
promptsGeneratedAt: 2025-12-01T22:29:12.212Z
createdAt: 2025-11-30T01:14:19.052Z
updatedAt: 2025-12-02T02:32:59.954Z
progressNotes:
  - timestamp: 2025-12-02T00:58:39.132Z
    text: Added genai workspace dependency, created llm module scaffolding with
      types/error/mock, added map_llm_error with tests and reexports; cargo
      fmt/test passing.
    source: "implementer: tasks 1,2,3,4,6,8,9"
  - timestamp: 2025-12-02T01:02:49.871Z
    text: Added serde roundtrip tests for llm
      ChatRole/ChatMessage/CompletionRequest/CompletionResponse and Display
      tests for LLMError; cargo test -p ashford-core now passes (214 tests).
    source: "tester: llm coverage"
  - timestamp: 2025-12-02T01:23:05.101Z
    text: Implemented genai-backed LLM client with request logging, added llm_calls
      migration and repository. LLM client now builds ChatRequest/ChatOptions,
      maps genai errors to LLMError, records telemetry to llm_calls with
      context, and defaults org/user from constants. Added llm_calls table
      migration & wired into migrations list/tests; repository supports
      create/list with context JSON and tests pass.
    source: "implementer: tasks 5,11,12"
  - timestamp: 2025-12-02T01:28:57.541Z
    text: Added LLM module tests (error mapping, option/request builders, log_call
      persistence defaults) and verified with cargo test -p ashford-core.
    source: "tester: task5-11-12"
  - timestamp: 2025-12-02T01:49:03.297Z
    text: Added cfg(feature="llm-integration") OpenAI smoke test that exercises
      GenaiLLMClient end-to-end and verifies request logging; introduced
      llm-integration feature flag in ashford-core Cargo.toml.
    source: "implementer: task10"
  - timestamp: 2025-12-02T01:50:26.866Z
    text: Ran ashford-core tests with and without llm-integration feature; fixed
      llm_integration test by importing LLMClient so feature build passes. All
      tests now succeed (llm integration skipped without OPENAI_API_KEY).
    source: "tester: task10"
  - timestamp: 2025-12-02T02:12:00.955Z
    text: "Completed code review of LLM provider integration. All 12 tasks appear to
      be implemented. Tests pass (228 tests). Key components reviewed:
      GenaiLLMClient, LLMError, LlmCallRepository, MockLLMClient, map_llm_error,
      migration 005. Found one potential issue with AuthenticationFailed being
      marked retryable in map_llm_error but that contradicts the implementation
      note that says it was changed to fatal. Actually checked jobs/mod.rs and
      it's retryable there as documented. No major issues found."
    source: "reviewer: Phase 4.3 LLM Provider Integration"
  - timestamp: 2025-12-02T02:14:43.399Z
    text: Completed code review; identified rate-limit handling gap, model logging
      mismatch, auth retry policy issue, and missing index on user_id for
      llm_calls.
    source: "reviewer: code-review"
  - timestamp: 2025-12-02T02:17:52.954Z
    text: "Fixed all 4 review issues: (1) Changed AuthenticationFailed from
      Retryable to Fatal in map_llm_error to avoid pointless retries since LLM
      API keys cannot be refreshed, updated tests accordingly. (2) Added
      list_by_org method to LlmCallRepository for admin auditing use cases that
      need org-wide visibility regardless of user, with test coverage. (3)
      Exported LlmCallError from lib.rs via llm module re-exports. (4) Exported
      RateLimitInfo from lib.rs for external code to access retry_after_ms. All
      229 tests pass."
    source: "implementer: autofix"
  - timestamp: 2025-12-02T02:29:43.549Z
    text: Propagated LLM rate-limit retry_after into JobError and queue scheduling;
      logged actual provider model iden in llm call records; tests updated and
      passing.
    source: "implementer: review-fixes"
  - timestamp: 2025-12-02T02:30:52.614Z
    text: Ran cargo test -p ashford-core; all 230 unit tests pass and llm
      integration feature compiled/skipped (no OPENAI_API_KEY).
    source: "tester: Phase 4.3 LLM"
  - timestamp: 2025-12-02T02:32:59.950Z
    text: Added worker-level retry-after test to ensure JobError::retry_after
      propagates to not_before scheduling; ran cargo fmt and cargo test -p
      ashford-core (231 tests) passing.
    source: "tester: Phase 4.3 LLM"
tasks:
  - title: Add genai dependency to workspace
    done: true
    description: Add genai crate to server/Cargo.toml workspace dependencies and
      server/crates/ashford-core/Cargo.toml. Pin to a specific version for
      stability.
  - title: Create llm module structure
    done: true
    description: Create server/crates/ashford-core/src/llm/ directory with mod.rs,
      types.rs, error.rs, and mock.rs files. Add `pub mod llm;` to lib.rs.
  - title: Define LLM types
    done: true
    description: "In types.rs, define: ChatRole enum (System, User, Assistant),
      ChatMessage struct (role, content), CompletionRequest struct (messages,
      temperature, max_tokens, json_mode), CompletionResponse struct (content,
      model, input_tokens, output_tokens, latency_ms). Follow existing serde
      patterns."
  - title: Define LLMError enum
    done: true
    description: "In error.rs, define LLMError enum with variants: RateLimited,
      AuthenticationFailed, InvalidRequest(String), ServerError(String),
      Timeout, ParseError(String), ProviderError(String). Use thiserror for
      derive(Error)."
  - title: Implement LLMClient
    done: true
    description: "In mod.rs, implement LLMClient struct wrapping genai::Client and
      holding Database reference. Add complete() async method that: accepts
      CompletionRequest and LlmCallContext, constructs ChatRequest from
      CompletionRequest, measures latency with Instant, calls genai exec_chat(),
      extracts token counts from response.usage, logs the call to llm_calls
      table via LlmCallRepository (both successes and failures - store error
      message in error field for failed calls), maps genai errors to LLMError.
      Use ModelConfig for provider/model selection. Errors during logging should
      be logged via tracing but not fail the request."
  - title: Add map_llm_error function
    done: true
    description: "In jobs/mod.rs, add map_llm_error(context: &str, err: LLMError) ->
      JobError function following the existing map_gmail_error pattern. Rate
      limits, server errors, timeouts -> Retryable. Invalid requests, parse
      errors -> Fatal."
  - title: Create MockLLMClient for testing
    done: true
    description: In mock.rs, implement MockLLMClient that can be configured with
      canned responses or errors. Use
      Arc<Mutex<VecDeque<Result<CompletionResponse, LLMError>>>> for response
      queue. Implement same interface as LLMClient.
  - title: Add public re-exports to lib.rs
    done: true
    description: "In lib.rs, add pub use statements for: LLMClient, LLMError,
      CompletionRequest, CompletionResponse, ChatMessage, ChatRole,
      MockLLMClient. Follow existing re-export patterns."
  - title: Write unit tests for LLM error mapping
    done: true
    description: "Add #[cfg(test)] module in jobs/mod.rs with tests for
      map_llm_error. Test each LLMError variant maps to correct JobError type.
      Follow existing test patterns in the file."
  - title: Write integration test for LLM client
    done: true
    description: "Create tests/llm_integration.rs with #[cfg(feature =
      \"llm-integration\")] tests. Add 'llm-integration' feature to Cargo.toml.
      Test real API call with OpenAI or other provider. Verify response parsing
      and token counting."
  - title: Create migration for llm_calls table
    done: true
    description: "Create server/migrations/005_add_llm_calls.sql with llm_calls
      table: id (TEXT PRIMARY KEY), org_id (INTEGER), user_id (INTEGER), feature
      (TEXT - e.g., 'classification', 'rules_assistant'), context_json (TEXT -
      flexible context like account_id, message_id, rule_name), model (TEXT),
      request_json (TEXT - full prompt/messages), response_json (TEXT - full
      response), input_tokens (INTEGER), output_tokens (INTEGER), latency_ms
      (INTEGER), error (TEXT nullable), trace_id (TEXT nullable), created_at
      (TEXT). Add indices on (org_id, created_at) and (feature, created_at)."
    files: []
    docs: []
    steps: []
  - title: Define LlmCall types and repository
    done: true
    description: "Create llm/repository.rs with: LlmCallContext struct (feature,
      account_id, message_id, rule_name, etc. as Option fields), NewLlmCall
      struct for inserts, LlmCall struct for reads, LlmCallRepository with
      create() and list() methods. Follow existing repository patterns from
      accounts.rs. Add re-exports to lib.rs."
    files: []
    docs: []
    steps: []
changedFiles:
  - server/Cargo.lock
  - server/Cargo.toml
  - server/crates/ashford-core/Cargo.toml
  - server/crates/ashford-core/src/decisions/repositories.rs
  - server/crates/ashford-core/src/jobs/mod.rs
  - server/crates/ashford-core/src/lib.rs
  - server/crates/ashford-core/src/llm/error.rs
  - server/crates/ashford-core/src/llm/mock.rs
  - server/crates/ashford-core/src/llm/mod.rs
  - server/crates/ashford-core/src/llm/repository.rs
  - server/crates/ashford-core/src/llm/types.rs
  - server/crates/ashford-core/src/messages.rs
  - server/crates/ashford-core/src/migrations.rs
  - server/crates/ashford-core/tests/llm_integration.rs
  - server/migrations/005_add_llm_calls.sql
tags: []
---

Foundation for LLM-based classification. This layer handles provider selection, API calls, error handling, and basic telemetry.

## Key Components

### Provider Selection
**Decision: Use genai crate**

The genai crate provides multi-provider abstraction with support for OpenAI, Anthropic, Google Gemini, and others. This approach prioritizes rapid development while the crate handles provider-specific details.

Trade-offs accepted:
- Pre-1.0 versioning (possible breaking changes)
- Generic error types requiring mapping to JobError
- Less control over provider-specific features

Benefits:
- Immediate multi-provider support (9+ providers)
- Normalized token counting
- Structured output support
- Active maintenance

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

## Research

### Summary
- The LLM provider integration forms the foundation for the decision engine's classification capabilities
- Must integrate with existing `ModelConfig` in `config.rs` and map errors to `JobError::Retryable`/`Fatal` from `worker.rs`
- Two viable approaches: genai crate (rapid development, 9+ providers) vs custom trait abstraction (full control, explicit error handling)
- The codebase already follows consistent patterns for async traits, error mapping, and repository structures that should be replicated
- Classification workflow is triggered after deterministic rules fail to match, feeding into directions enforcement and decision persistence

### Findings

#### Existing Configuration System

**ModelConfig Location:** `server/crates/ashford-core/src/config.rs` (lines 41-46)

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelConfig {
    pub provider: String,
    pub model: String,
    pub temperature: f32,
    pub max_output_tokens: u32,
}
```

The existing ModelConfig is simple and flat, suitable for the LLM provider factory pattern. Configuration supports:
- Environment variable overrides via `apply_env_overrides()` method
- `env:` indirection markers resolved by `resolve_env_markers()` for secrets
- Example environment overrides: `MODEL` overrides `model` field

**Configuration Example (from docs):**
```toml
[model]
provider = "vercel"
model = "gemini-1.5-pro"
temperature = 0.2
max_output_tokens = 1024
```

#### Error Handling Patterns

**JobError Definition:** `server/crates/ashford-core/src/worker.rs` (lines 40-58)

```rust
#[derive(Debug, Error)]
pub enum JobError {
    #[error("retryable: {0}")]
    Retryable(String),
    #[error("fatal: {0}")]
    Fatal(String),
}
```

The codebase uses two-variant error enums with clear semantic meaning. Error mapping functions exist in `jobs/mod.rs` that demonstrate the established classification pattern:

**Error Mapping Example:** `server/crates/ashford-core/src/jobs/mod.rs` (lines 56-104)

```rust
pub(crate) fn map_gmail_error(context: &str, err: GmailClientError) -> JobError {
    match err {
        GmailClientError::Unauthorized => JobError::Retryable(format!("{context}: unauthorized")),
        GmailClientError::Http(ref http_err) => {
            if let Some(status) = http_err.status() {
                match status {
                    StatusCode::NOT_FOUND => JobError::Fatal(format!("{context}: resource not found (404)")),
                    StatusCode::TOO_MANY_REQUESTS | StatusCode::FORBIDDEN => {
                        JobError::Retryable(format!("{context}: rate limited ({status})"))
                    }
                    StatusCode::UNAUTHORIZED => JobError::Retryable(format!("{context}: unauthorized (401)")),
                    status if status.is_server_error() => JobError::Retryable(format!("{context}: server error {status}")),
                    status => JobError::Fatal(format!("{context}: http status {status}")),
                }
            } else {
                JobError::Retryable(format!("{context}: network error {http_err}"))
            }
        }
        // ... more mappings
    }
}
```

This pattern should be replicated for `map_llm_error()` in the new LLM module.

#### Retry and Backoff System

**Backoff Implementation:** `server/crates/ashford-core/src/queue.rs` (lines 438-446)

```rust
fn backoff_with_jitter(attempts: i64) -> Duration {
    let attempts = attempts.max(1);
    let exp = attempts.min(20);
    let base = 2_i64.saturating_pow(exp as u32);
    let delay_secs = base.min(300);  // max 300 seconds (5 minutes)
    let mut rng = rand::thread_rng();
    let factor: f64 = rng.gen_range(0.75..=1.25);
    Duration::from_secs_f64((delay_secs as f64) * factor)
}
```

The job queue already handles retry logic with exponential backoff. LLM errors mapped to `JobError::Retryable` will automatically use this system. Default max_attempts is 5 (line 114).

#### Async Trait Patterns

**TokenStore Trait Example:** `server/crates/ashford-core/src/gmail/oauth.rs` (lines 38-55)

```rust
#[async_trait]
pub trait TokenStore: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;
    async fn save_tokens(&self, tokens: &OAuthTokens) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, Default)]
pub struct NoopTokenStore;

#[async_trait]
impl TokenStore for NoopTokenStore {
    type Error = Infallible;
    async fn save_tokens(&self, _tokens: &OAuthTokens) -> Result<(), Self::Error> {
        Ok(())
    }
}
```

Key conventions:
- All async traits use `#[async_trait]` from the `async-trait` crate (already in dependencies)
- Traits require `Send + Sync` bounds for thread safety
- Noop implementations provided for testing
- Associated error types with strict constraints

**GmailClient Parametrization:** `server/crates/ashford-core/src/gmail/client.rs` (lines 34-44)

```rust
pub struct GmailClient<S: TokenStore> {
    http: Client,
    token_store: Arc<S>,
    tokens: Arc<RwLock<OAuthTokens>>,
    refresh_lock: Arc<Mutex<()>>,
    // ... other fields
}
```

This shows how to parametrize clients on traits for testability.

#### Module Organization

The codebase follows a consistent structure in `server/crates/ashford-core/src/`:
- Each functional area is a self-contained module or directory
- Sub-modules follow: `mod.rs` for organization, `types.rs` for data models
- `lib.rs` explicitly re-exports public types
- Related concerns grouped together (e.g., all Gmail in `gmail/`)

**Public API Re-exports:** `server/crates/ashford-core/src/lib.rs` demonstrates the pattern:
```rust
pub mod gmail;
pub use gmail::{GmailClient, GmailClientError, NoopTokenStore, OAuthTokens, TokenStore, ...};
```

#### Testing Patterns

From `server/crates/ashford-core/tests/ingest_flow.rs`:
- WireMock for HTTP mocking of external APIs
- TempDir for isolated database testing
- Fast worker configs for integration tests
- No mocks for internal components

```rust
#[tokio::test]
async fn worker_processes_history_and_ingests_message() {
    let (db, account_repo, queue, _dir, account_id) = setup_account().await;
    // ...
}
```

#### Decision Engine Architecture

From `docs/decision_engine.md` and related documentation:

**Five-Layer Prompt Structure:**
1. SYSTEM - Agent role and JSON output contract
2. DIRECTIONS - Global guardrails (hard constraints)
3. LLM RULES - Scoped situational guidance
4. MESSAGE - Email context (from, to, subject, body, headers)
5. TASK - Required JSON schema and constraints

**Output Contract:** The LLM must return structured JSON matching the decision schema:
```json
{
  "decision": {
    "action": "apply_label|archive|...",
    "parameters": {},
    "confidence": 0.0,
    "needs_approval": true,
    "rationale": "string"
  },
  "telemetry": {
    "model": "provider:model@version",
    "latency_ms": 0,
    "input_tokens": 0,
    "output_tokens": 0
  }
}
```

**Post-LLM Validation (Rust-side):**
- JSON schema + semantic validation
- Direction enforcement (override if model violates guardrails)
- Dangerous action policy (irreversible actions require approval)
- Confidence thresholds (low confidence → needs_approval)

#### genai Crate Evaluation

**Status:** Active, pre-1.0 (0.4.x), maintained by Jeremy Chone

**Provider Support (9 providers):**
- OpenAI (gpt-4, gpt-4o, gpt-4o-mini)
- Anthropic (claude-3-haiku, claude-3-opus, claude-3.5-sonnet)
- Google Gemini (gemini-2.0-flash, gemini-1.5-pro)
- xAI Grok, Ollama, Groq, DeepSeek, Cohere

**Strengths:**
- Time to market: Ready now, proven in production (AIPACK runtime)
- Structured output and streaming support built-in
- Normalized token counting across providers
- AWS Bedrock/Vertex AI integration via endpoint overrides

**Weaknesses:**
- Pre-1.0 versioning: Breaking changes possible between minor versions
- Generic error types (webc::Error) require manual mapping to JobError
- Limited access to provider-specific features
- Crate controls API evolution, not the project

**Custom Trait Alternative Strengths:**
- Full control over error handling aligned with JobError
- No external dependency risk
- Cleaner integration with directions enforcement
- Can add provider-specific features (retry-after headers, etc.)

**Custom Trait Weaknesses:**
- 3-5 days development per provider
- Ongoing maintenance as provider APIs evolve

**Recommendation:** Hybrid approach - define custom `LLMProvider` trait, implement with genai as initial backend, can swap to custom implementations later if needed.

#### Existing Dependencies

From `server/Cargo.toml` and `server/crates/ashford-core/Cargo.toml`:

**Already Available:**
- `async-trait = "0.1.83"` - for async trait definitions
- `reqwest` with rustls-tls - HTTP client for API calls
- `serde` + `serde_json` - JSON serialization
- `thiserror` - custom error types
- `chrono` - timestamp handling
- `tokio` with full features - async runtime
- `tracing` - structured logging

**Would Need to Add:**
- `genai` (if using genai approach)
- Or provider-specific SDKs (if custom approach)

#### Secrets Management

The codebase handles API keys via environment variable indirection in config:
```toml
bot_token = "env:DISCORD_BOT_TOKEN"
```

LLM API keys should follow this pattern:
```toml
[model]
provider = "openai"
api_key = "env:OPENAI_API_KEY"
```

The `resolve_env_markers()` function in `config.rs` handles resolution at config load time.

### Risks & Constraints

#### Provider API Stability
- OpenAI, Anthropic, and Google APIs are stable but evolving
- New models may require updates to token counting or parameter handling
- genai crate abstracts this but is pre-1.0 itself

#### Error Classification Complexity
- LLM providers return varied error responses
- Rate limit headers (Retry-After, x-ratelimit-*) should be respected when present
- Some 4xx errors are retryable (429, 403 often means rate limit)
- Need clear mapping to JobError variants

#### Token Counting Accuracy
- Provider-reported tokens may differ from estimates
- Streaming mode may not provide accurate token counts until completion
- Input token estimation for prompt construction is useful for cost prediction

#### JSON Output Reliability
- Not all providers guarantee valid JSON in json_mode
- Post-processing validation is essential regardless of provider guarantees
- Schema validation must be lenient enough to handle minor variations

#### Latency Measurement
- Should measure wall-clock time from request start to response completion
- Include network latency, not just API processing time
- Use for both telemetry and timeout decisions

#### Credentials Security
- API keys must not be logged
- Use env: indirection pattern from config.rs
- Consider key rotation support in future

#### Testing Considerations
- Mock provider essential for unit tests (following existing WireMock patterns)
- Integration tests with real providers should be behind feature flag
- Need representative error responses for each failure mode

## Implementation Plan

### Expected Behavior/Outcome
- A new `llm` module in ashford-core that provides LLM completion capabilities via genai
- Configuration extended to support API keys for multiple providers
- Errors from LLM providers correctly mapped to `JobError::Retryable` or `JobError::Fatal`
- Telemetry captured (model, tokens, latency) for each completion request
- Mock implementation available for unit testing

**States:**
- Success: Completion returned with content and telemetry
- Retryable Error: Rate limit, server error, timeout, auth refresh needed
- Fatal Error: Invalid request, malformed response, missing API key

### Key Findings

**Product & User Story:**
This phase provides the foundation layer for LLM-based email classification. The LLM client will be called by the classification job when deterministic rules don't match, enabling intelligent email sorting based on natural language rules and directions.

**Technical Plan:**
1. Add genai dependency to workspace
2. Create `llm/` module with types, client wrapper, and error handling
3. Extend ModelConfig with `api_key` field using env: indirection
4. Implement `map_llm_error()` following existing patterns in `jobs/mod.rs`
5. Create `MockLLMClient` for testing
6. Add re-exports to `lib.rs`

**Risks:**
- genai breaking changes in future versions (mitigated by pinning version)
- Provider API changes affecting token counting (genai abstracts this)

### Acceptance Criteria
- [ ] `LLMClient` struct wraps genai and exposes `complete()` method
- [ ] `CompletionRequest` and `CompletionResponse` types defined
- [ ] `LLMError` enum covers rate limit, auth, invalid request, server error, timeout, parse error
- [ ] `map_llm_error()` correctly classifies errors as Retryable or Fatal
- [ ] `MockLLMClient` implemented for unit testing
- [ ] Unit tests cover error mapping for all LLMError variants
- [ ] Integration test (behind feature flag) validates real provider call
- [ ] `llm_calls` table created with migration for full request/response logging
- [ ] `LlmCallContext` captures call origin (feature, org_id, user_id, account_id, rule_name, etc.)
- [ ] Every LLM call is logged to database with full request, response, tokens, and latency

### Dependencies & Constraints
- **Dependencies:** Depends on existing `ModelConfig` in config.rs, `JobError` in worker.rs
- **Technical Constraints:** Must use async/await patterns consistent with codebase; API keys must not be logged

### Implementation Notes

**Recommended Approach:**
1. Start with minimal `LLMClient` wrapping genai's `Client::default()`
2. Implement non-streaming `complete()` first (streaming not needed for classification)
3. Use genai's `ChatRequest::default().with_system()` and `.append_message()` for prompt construction
4. Extract token counts from `ChatResponse::usage` field
5. Measure latency with `std::time::Instant` around the API call

**Potential Gotchas:**
- genai uses `ModelIden::from_str()` for model parsing - format is "provider:model" or just "model" with provider auto-detected
- API keys are loaded from environment variables by genai automatically (e.g., `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`)
- For custom endpoints (Vertex AI, Bedrock), additional configuration may be needed

**File Organization:**
```
server/crates/ashford-core/src/llm/
├── mod.rs          # Re-exports, LLMClient struct
├── types.rs        # CompletionRequest, CompletionResponse, ChatMessage
├── error.rs        # LLMError enum
├── repository.rs   # LlmCallContext, LlmCall, LlmCallRepository
└── mock.rs         # MockLLMClient for testing
```

Implemented LLM scaffolding and error mapping for Phase 4.3. Completed tasks: Task 1 (Add genai dependency to workspace), Task 2 (Create llm module structure), Task 3 (Define LLM types), Task 4 (Define LLMError enum), Task 6 (Add map_llm_error function), Task 8 (Add public re-exports to lib.rs), Task 9 (Write unit tests for LLM error mapping).\n\nDependencies: Added genai to workspace dependencies in server/Cargo.toml and referenced it from ashford-core/Cargo.toml; current lock resolves to genai 0.2.4.\n\nLLM module: Created server/crates/ashford-core/src/llm/{mod.rs,error.rs,types.rs,mock.rs}. types.rs defines ChatRole (system/user/assistant), ChatMessage, CompletionRequest (temperature, max_tokens, json_mode), and CompletionResponse (content, model, input/output tokens, latency). error.rs introduces LLMError variants covering rate limits, auth failures, invalid requests, server errors, timeouts, parse errors, and generic provider errors with thiserror derives. mock.rs holds a placeholder MockLLMClient constructor for upcoming tests. mod.rs re-exports the LLM types and error for crate-wide use.\n\nJob error mapping: Added map_llm_error in jobs/mod.rs to translate LLMError into JobError retryable/fatal semantics consistent with Gmail mapping (429/401/server/timeouts/provider -> Retryable; invalid request/parse -> Fatal). Annotated with allow(dead_code) until the LLM client wiring lands. Added unit tests ensuring retryable vs fatal classification and message contents.\n\nPublic API: lib.rs now exposes the new llm module items (ChatMessage, ChatRole, CompletionRequest, CompletionResponse, LLMError, MockLLMClient).\n\nFormatting/side effects: cargo fmt touched existing files, reordering imports in decisions/repositories.rs and messages.rs without functional changes.\n\nTesting: Ran  after formatting; tests pass and map_llm_error warnings cleared.

Updated workspace genai to 0.4.4 in server/Cargo.toml and Cargo.lock so upcoming multi-provider features are available for the real client. Added an async LLMClient trait in llm/mod.rs, re-exported it through lib.rs to meet the planned public API, and refactored MockLLMClient into a functional queue-backed mock (Arc<Mutex<VecDeque<Result<CompletionResponse, LLMError>>>>) with enqueue_response plus async complete() returning the next queued result; added tokio tests covering ordering and empty-queue behavior. map_llm_error now treats AuthenticationFailed as fatal to avoid pointless retries without a refresh path; tests updated. Changes touch server/crates/ashford-core/src/llm/{mod.rs,mock.rs}, jobs/mod.rs, lib.rs, server/Cargo.toml, server/Cargo.lock. Related tasks: Task 1 (genai version), Task 5/7/8 (LLMClient interface & mock), Task 6 (error mapping). Verified with cargo test -p ashford-core (216 tests).

Implemented tasks 5, 11, and 12. Added genai-backed GenaiLLMClient that now takes a CompletionRequest plus LlmCallContext, builds provider-specific chat requests/options, measures latency, maps genai errors to LLMError (including adapter-not-supported, rate limits, timeouts, auth, parse), and logs both successes and failures to the database. Logging is non-fatal and records request/response JSON, token counts, latency, and error string via the new repository.
Created llm/repository.rs with LlmCallContext (feature plus optional org/user/account/message/rule metadata), NewLlmCall, LlmCall, and LlmCallRepository supporting create/list queries with org/user scoping and feature filter; added unit tests for insert/list filtering. Updated the LLM trait signature and mock client to accept context.
Added migration 005_add_llm_calls.sql defining the llm_calls table (org_id/user_id, feature, context_json, model, request/response JSON, token counts, latency, error, trace_id, created_at) plus indices on (org_id, created_at) and (feature, created_at). Wired the migration into migrations.rs, adjusted migration tests (table existence and count now 5). Exported new types through lib.rs.
Design choices: namespace models as <provider>::<model> for genai adapter resolution; fallback org/user defaults to constants when context does not specify them; request logging serializes the input request rather than the genai request for readability; token counts default to zero when provider usage is absent. Tests: cargo test -p ashford-core (218 tests) after cargo fmt.

Implemented rate-limit metadata support by extending LLMError::RateLimited to carry RateLimitInfo (retry_after_ms) and parsing Retry-After/X-RateLimit-Reset headers in map_webc_error, ensuring job mapping and logs include instructed backoff while treating authentication failures as retryable. GenaiLLMClient now accepts an injectable ChatExecutor (with with_executor constructor) for testability; log_call normalizes context org/user IDs to defaults before persistence for consistent context_json. Added stubbed ChatExecutor tests covering the complete() happy path (request/options construction, token capture, logging) and rate-limit failure path (retry-after propagation), plus updated existing error mapping tests. Modified files: server/crates/ashford-core/src/llm/error.rs, llm/mod.rs, jobs/mod.rs; cargo test -p ashford-core passes.

Implemented Task 10: Write integration test for LLM client. Added an opt-in crate feature llm-integration in server/crates/ashford-core/Cargo.toml so real provider tests only run when explicitly enabled. Introduced tests/llm_integration.rs (guarded by cfg(feature="llm-integration")) that performs a live OpenAI chat call through GenaiLLMClient using a minimal ModelConfig (provider openai, default model gpt-4o-mini or override via LLM_INTEGRATION_MODEL), asserting the reply contains 'pong', latency is recorded, and the call is persisted via LlmCallRepository with default org/user scoping. The test skips cleanly when OPENAI_API_KEY is absent, keeping the default test suite unaffected (shows 0 tests when the feature is off).

Added token counting coverage to Task 10 (LLM integration test). Updated server/crates/ashford-core/tests/llm_integration.rs to assert the live completion response reports nonzero input_tokens and output_tokens, and confirmed the persisted LlmCall row stores matching token counts, ensuring response parsing and logging retain usage data. Ran cargo test -p ashford-core from server/ to verify the crate builds and tests pass with the feature-gated integration test still compiling/skipping cleanly without OPENAI_API_KEY.

Autofix for review issues completed. Fixed four issues: (1) Changed AuthenticationFailed from Retryable to Fatal in map_llm_error (jobs/mod.rs lines 100-102) - LLM API keys cannot be refreshed like OAuth tokens, so retrying auth failures is pointless; updated tests accordingly. (2) Added list_by_org method to LlmCallRepository (repository.rs lines 173-200) for admin auditing use cases - queries by org_id only without requiring user_id, with comprehensive test coverage. (3) Exported LlmCallError from llm/mod.rs and lib.rs for external error handling. (4) Exported RateLimitInfo from llm/mod.rs and lib.rs so external code can access retry_after_ms when pattern matching on LLMError::RateLimited. All 229 tests pass.

Implemented review fixes for Issue 1 (rate-limit metadata respected) and Issue 2 (log actual provider model). Added retry-after awareness to job retry flow: JobError::Retryable now carries an optional retry_after Duration with helper constructors; worker propagate this through FinalizeAction into queue.fail; queue.fail accepts an optional delay and sets not_before based on Retry-After instead of generic jitter when provided. map_llm_error now preserves RateLimitInfo.retry_after_ms by returning JobError::retryable_after to honor provider backoff guidance. Added queue test fail_uses_explicit_retry_after_when_provided and extended map_llm_error tests to assert retry_after propagation. Updated LLM logging to store the actual provider_model_iden returned by genai instead of the requested model; log_call now takes a model parameter, success paths pass the provider model, failures fall back to the configured model, and tests assert the recorded model matches the response. Key files: worker.rs, queue.rs, jobs/mod.rs, gmail job handlers (retryable helper), llm/mod.rs, related tests; cargo fmt run and all ashford-core tests passing.
