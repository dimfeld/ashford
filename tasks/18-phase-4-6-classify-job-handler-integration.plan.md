---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Phase 4.6: Classify Job Handler & Integration"
goal: Create the classify job handler that orchestrates the full decision
  pipeline and integrate with existing job system
id: 18
uuid: 9def82bc-4c74-4945-882a-81a674f25cf1
generatedBy: agent
status: done
priority: high
container: false
temp: false
dependencies:
  - 14
  - 16
  - 17
parent: 4
references:
  "4": 5cf4cc37-3eb8-4f89-adae-421a751d13a1
  "14": 4faa40e3-cbc5-4d8c-a596-225ab64a50d9
  "16": b8c142c5-3335-4b87-9a94-28dbcc96af99
  "17": 85737737-8826-483b-9a82-87e7c0098c90
issue: []
pullRequest: []
docs:
  - docs/rules_engine.md
  - docs/decision_engine.md
  - docs/job_queue.md
planGeneratedAt: 2025-12-02T21:01:29.320Z
promptsGeneratedAt: 2025-12-02T21:01:29.320Z
createdAt: 2025-11-30T01:14:19.538Z
updatedAt: 2025-12-02T23:06:20.880Z
progressNotes:
  - timestamp: 2025-12-02T21:35:56.117Z
    text: "Completed foundational tasks for classify job handler: 1) Added
      MessageRepository::get_by_id method with tests, 2) Expanded JobDispatcher
      with llm_client and policy_config fields, updated constructor signature,
      3) Added map_executor_error function for rules engine errors, 4) Updated
      all job tests (mod.rs, ingest_gmail.rs, history_sync_gmail.rs,
      backfill_gmail.rs) for new JobDispatcher signature, 5) Added Default impl
      for PolicyConfig, 6) Updated main.rs to instantiate GenaiLLMClient and
      pass to JobDispatcher. All 311 tests pass."
    source: "implementer: Tasks 1,2,6,15"
  - timestamp: 2025-12-02T21:51:33.005Z
    text: Completed Tasks 3, 4, 5, and 7. Created classify.rs with
      load_llm_rules_for_message() helper for loading rules from all scopes
      (Global, Account, Domain, Sender) with deduplication. Added
      rule_match_to_decision_output() to convert deterministic matches to
      DecisionOutput format. Implemented full handle_classify() handler with
      two-phase approach (deterministic fast path, LLM slow path),
      SafetyEnforcer integration, and Decision/Action persistence. Registered
      classify job in dispatcher and exported JOB_TYPE_CLASSIFY. Fixed
      LlmCallContext field names (feature instead of call_type, optional
      org_id/user_id). All 322 tests pass.
    source: "implementer: Tasks 3,4,5,7"
  - timestamp: 2025-12-02T21:53:54.020Z
    text: "All tests pass for classify job implementation. Test coverage includes:
      11 tests for classify.rs covering load_llm_rules_for_message (3 tests: all
      scopes, no sender, deduplication), rule_match_to_decision_output (3 tests:
      confidence, safe_mode variants), generate_undo_hint (2 tests), error
      handling (3 tests: invalid payload, message not found, account not found -
      Task 9). The worker_marks_fatal_as_failed flaky failure was due to SQLite
      concurrency under parallel test execution - passes consistently when run
      individually or in subsequent full suite runs."
    source: "tester: Tasks 3,4,5,7"
  - timestamp: 2025-12-02T22:03:23.573Z
    text: "Completed Tasks 8 and 14. Task 8: Added classify job enqueueing to
      ingest_gmail.rs - imports (JOB_TYPE_CLASSIFY, JobQueue, QueueError),
      enqueue_classify_job helper function using idempotency key format
      'classify:{account_id}:{message_id}', priority 0, duplicate handling via
      debug log. Task 14: Updated ingest_fetches_and_persists_message test to
      verify classify job enqueued with correct payload (account_id and internal
      message UUID) and priority 0. Added two new tests:
      ingest_does_not_enqueue_classify_on_failure (verifies no classify job on
      404), ingest_handles_duplicate_classify_idempotency (verifies second
      ingest succeeds and only one classify job exists). All 324 tests pass."
    source: "implementer: Tasks 8,14"
  - timestamp: 2025-12-02T22:15:01.147Z
    text: "Completed all integration tests for classify job handler. Added
      call_count() method to MockLLMClient for tracking invocations. Task 10:
      Two tests for deterministic rule path - one for safe action (archive)
      creating Decision with source=Deterministic and Action with status=Queued,
      and one for dangerous action (delete) requiring approval with Action
      status=ApprovedPending. Both verify LLM is NOT called. Task 11: Test for
      LLM decision path - creates mock response with valid DecisionOutput tool
      call, verifies LLM called once, Decision has source=Llm, and Action
      created with correct status. Task 12: Test for safety enforcement - LLM
      returns delete action with low confidence (0.5), safety enforcer overrides
      to require approval, telemetry_json contains safety_overrides with both
      dangerous_action and low_confidence entries. Task 13: Four tests for LLM
      error handling - RateLimited returns Retryable with retry_after duration,
      AuthenticationFailed returns Fatal, ServerError returns Retryable, and
      NoToolCall (empty tool_calls) returns Fatal. All 333 tests pass."
    source: "implementer: Tasks 10,11,12,13"
  - timestamp: 2025-12-02T22:18:10.056Z
    text: "Verified all integration tests pass (333 tests total). Tests cover:
      deterministic rule path (safe/dangerous actions), LLM decision path,
      safety enforcement (overrides needs_approval), and LLM error handling
      (rate limit, auth failure, server error, no tool call). MockLLMClient
      properly supports call_count() for verifying LLM was/wasn't called.
      Initial flaky test failure on
      classify_safety_enforcement_overrides_to_require_approval was a race
      condition that doesn't reproduce - test passes consistently in isolation
      and in subsequent full runs. Build compiles without warnings."
    source: "tester: Tasks 10-13"
  - timestamp: 2025-12-02T22:31:24.354Z
    text: "Found issues: classify job does not enqueue action/approval jobs, safety
      overrides not reflected in stored decision_json, classify allows
      message/account mismatch because get_by_id ignores account and handler
      doesn't verify."
    source: "reviewer: code-review"
  - timestamp: 2025-12-02T22:44:07.146Z
    text: "Implemented fixes for review issues: added follow-up action/approval job
      enqueueing and stub handlers, enforced message/account ownership in
      classify, and aligned stored decision_json with safety overrides."
    source: "implementer: review-fixes"
  - timestamp: 2025-12-02T22:45:47.126Z
    text: Ran cargo test -p ashford-core from server/; all 336 crate tests +
      binary/integration suites passed with new classify follow-up changes.
    source: "tester: Phase 4.6 tests"
  - timestamp: 2025-12-02T23:02:18.578Z
    text: "Fixed SafeMode bypass issues in classify job handler. Modified
      handle_classify in classify.rs to conditionally skip SafetyEnforcer for
      deterministic rules with DangerousOverride or AlwaysSafe modes. Added
      SafetyResult to imports. Added two new integration tests:
      classify_dangerous_override_bypasses_safety_enforcement and
      classify_always_safe_bypasses_safety_enforcement. Both tests verify that
      explicit SafeMode overrides on deterministic rules properly bypass safety
      enforcement. All 338 tests pass."
    source: "implementer: Fix Safety Mode Bypass Issues"
  - timestamp: 2025-12-02T23:04:26.554Z
    text: "Verified safety mode bypass tests are complete and passing. All 338 tests
      pass. Key tests: classify_dangerous_override_bypasses_safety_enforcement
      verifies DangerousOverride with delete action does NOT require approval.
      classify_always_safe_bypasses_safety_enforcement verifies AlwaysSafe with
      forward action (in approval_always list) does NOT require approval. Test
      coverage summary: SafeMode::Default with safe action (archive) - verified
      does not require approval; SafeMode::Default with dangerous action
      (delete) - verified requires approval; SafeMode::DangerousOverride with
      dangerous action - verified bypasses approval; SafeMode::AlwaysSafe with
      approval_always action - verified bypasses approval; LLM path - verified
      safety enforcement always applies. No additional tests needed."
    source: "tester: Verify Safety Mode Bypass Fixes"
  - timestamp: 2025-12-02T23:05:43.992Z
    text: "Reviewed implementation. The fix correctly addresses the original issues:
      (1) SafeMode::DangerousOverride and AlwaysSafe now skip SafetyEnforcer
      entirely via skip_safety_enforcement flag, (2)
      rule_match_to_decision_output sets needs_approval=false for both override
      modes, (3) New integration tests verify the behavior. All 338 tests pass."
    source: "reviewer: SafeMode bypass fix"
  - timestamp: 2025-12-02T23:06:20.875Z
    text: Completed autofix of review findings. The major issue
      (SafeMode::DangerousOverride not bypassing safety enforcement) and both
      minor issues (AlwaysSafe broken for approval_always list, missing test)
      have been fixed. Implementation adds conditional skip_safety_enforcement
      logic to handle_classify() and includes 2 new integration tests. All 338
      tests pass. Reviewer marked implementation as ACCEPTABLE with only minor
      documentation/test clarity suggestions.
    source: "orchestrator: review fixes"
tasks:
  - title: Add MessageRepository::get_by_id method
    done: true
    description: Add a `get_by_id(org_id, user_id, message_id)` method to
      MessageRepository in `server/crates/ashford-core/src/messages.rs`. The
      classify job receives the internal UUID message_id (not
      provider_message_id), so we need this lookup method. Follow the existing
      pattern from `get_by_provider_id`. Return `MessageError::NotFound` if not
      found.
  - title: Expand JobDispatcher with LLM and policy dependencies
    done: true
    description: >-
      Modify `server/crates/ashford-core/src/jobs/mod.rs` to add required fields
      to JobDispatcher:

      - `pub llm_client: Arc<dyn LLMClient>`

      - `pub policy_config: PolicyConfig`


      Update `JobDispatcher::new()` signature to:

      ```rust

      pub fn new(
          db: Database,
          http: reqwest::Client,
          llm_client: Arc<dyn LLMClient>,
          policy_config: PolicyConfig,
      ) -> Self

      ```


      Add necessary imports for `Arc`, `LLMClient` trait, and `PolicyConfig`.
      Keep the `.with_gmail_api_base()` builder method as-is.
  - title: Add helper to load LLM rules for all applicable scopes
    done: true
    description: >-
      Add a helper function (either in classify.rs or rules/repositories.rs) to
      load LLM rules for all applicable scopes based on a message. The function
      should:

      1. Load global rules (scope=Global, scope_ref=None)

      2. Load account rules (scope=Account, scope_ref=account_id)

      3. Load domain rules (scope=Domain, scope_ref=sender_domain)

      4. Load sender rules (scope=Sender, scope_ref=sender_email)

      5. Merge and dedupe results


      This is needed because LlmRuleRepository::list_enabled_by_scope only
      handles one scope at a time.
  - title: Create helper to convert RuleMatch to DecisionOutput
    done: true
    description: >-
      Create a helper function to convert a deterministic RuleMatch into a
      DecisionOutput for consistent handling. The function should:

      - Set message_ref from the message

      - Map action_type and action_parameters from the rule

      - Set confidence to 1.0 (deterministic)

      - Set needs_approval based on safe_mode (DangerousOverride=false,
      AlwaysSafe=false, Default=check danger level)

      - Set rationale to describe the matched rule

      - Set empty explanations (salient_features, matched_directions,
      alternatives)

      - Set undo_hint based on action type

      - Set empty telemetry placeholder
  - title: Implement classify job handler
    done: true
    description: >-
      Create `server/crates/ashford-core/src/jobs/classify.rs` with the main
      handler:


      1. Define ClassifyPayload struct with account_id and message_id (internal
      UUID)

      2. Implement handle_classify(dispatcher, job) -> Result<(), JobError>

      3. Parse payload, return Fatal on invalid

      4. Load message via MessageRepository::get_by_id, return Fatal if not
      found

      5. Load account via AccountRepository::get_by_id, return Fatal if not
      found

      6. Evaluate deterministic rules via RuleExecutor

      7. If match: convert to DecisionOutput, apply safety, persist, create
      action

      8. If no match: load directions, load LLM rules, build prompt, call LLM

      9. Parse LLM response via DecisionOutput::parse_from_tool_calls

      10. Apply SafetyEnforcer to set final needs_approval

      11. Persist Decision and Action records

      12. Log classification result


      Use map_llm_error for LLM errors, map_account_error for account errors.
      Return retryable for database errors.
  - title: Add error mapping for rules engine
    done: true
    description: >-
      Add `map_executor_error(context: &str, err: ExecutorError) -> JobError`
      function to `server/crates/ashford-core/src/jobs/mod.rs`. Map:

      - ExecutorError::Database -> Retryable

      - ExecutorError::Sql -> Retryable  

      - ExecutorError::MessageMissingFrom -> Fatal (data corruption)
  - title: Register classify job in dispatcher
    done: true
    description: >-
      Update `server/crates/ashford-core/src/jobs/mod.rs`:

      1. Add `mod classify;`

      2. Add `use classify::handle_classify;`

      3. Add `pub const JOB_TYPE_CLASSIFY: &str = "classify";`

      4. Add match arm in JobExecutor::execute: `JOB_TYPE_CLASSIFY =>
      handle_classify(self, job).await`

      5. Export JOB_TYPE_CLASSIFY in lib.rs
  - title: Wire classify job into ingest_gmail handler
    done: true
    description: >-
      Modify `server/crates/ashford-core/src/jobs/ingest_gmail.rs` to enqueue
      classify job after message persistence:


      1. Import JobQueue, JOB_TYPE_CLASSIFY, QueueError

      2. After msg_repo.upsert() succeeds, get the persisted message's internal
      ID

      3. Create idempotency_key: `format!("classify:{}:{}", account_id,
      message.id)`

      4. Enqueue classify job with payload `{"account_id": account_id,
      "message_id": message.id}`

      5. Handle QueueError::DuplicateIdempotency silently (debug log)

      6. Other enqueue errors -> retryable JobError

      7. Use priority 0 for classify jobs
  - title: Add unit tests for classify job
    done: true
    description: |-
      Add tests in classify.rs:
      1. Test invalid payload returns Fatal
      2. Test message not found returns Fatal
      3. Test account not found returns Fatal
  - title: Add integration test for deterministic rule path
    done: true
    description: |-
      Add integration test in classify.rs that:
      1. Sets up test database with message, account, and deterministic rule
      2. Creates JobDispatcher with mock LLM client
      3. Runs handle_classify
      4. Verifies Decision created with source=Deterministic
      5. Verifies Action created with correct status based on safety
      6. Verifies LLM was NOT called (deterministic short-circuit)
  - title: Add integration test for LLM decision path
    done: true
    description: >-
      Add integration test in classify.rs that:

      1. Sets up test database with message, account, directions, LLM rules (no
      deterministic rules)

      2. Creates JobDispatcher with MockLLMClient configured to return valid
      DecisionOutput

      3. Runs handle_classify

      4. Verifies Decision created with source=Llm

      5. Verifies Action created

      6. Verifies LLM was called with correct prompt structure
  - title: Add integration test for safety enforcement
    done: true
    description: >-
      Add integration test in classify.rs that:

      1. Sets up scenario where LLM returns dangerous action (e.g., Delete) with
      low confidence

      2. Verifies SafetyEnforcer overrides needs_approval to true

      3. Verifies Action created with status=ApprovedPending

      4. Verifies telemetry_json contains safety_overrides
  - title: Add integration test for LLM error handling
    done: true
    description: |-
      Add integration tests for LLM error scenarios:
      1. Test RateLimited error returns Retryable with retry_after
      2. Test AuthenticationFailed returns Fatal
      3. Test ServerError returns Retryable
      4. Test decision parse error (NoToolCall) returns Fatal
  - title: Update ingest_gmail tests for classify enqueueing
    done: true
    description: >-
      Update existing ingest_gmail tests to verify classify job is enqueued:

      1. In `ingest_fetches_and_persists_message` test, verify classify job was
      enqueued after ingest completes

      2. Add test that classify job is NOT enqueued on ingest failure

      3. Add test that duplicate classify idempotency is handled gracefully
  - title: Update existing job tests for new JobDispatcher signature
    done: true
    description: >-
      Update all existing job tests in `server/crates/ashford-core/src/jobs/` to
      use the new `JobDispatcher::new()` signature with required `llm_client`
      and `policy_config` parameters:


      1. In `mod.rs` tests: Update `unknown_job_type_is_fatal` test

      2. In `ingest_gmail.rs` tests: Update `setup_account()` helper and all
      tests

      3. In `history_sync_gmail.rs` tests: Update test setup

      4. In `backfill_gmail.rs` tests: Update test setup


      For tests that don't need real LLM functionality, use
      `Arc::new(MockLLMClient::new())` and `PolicyConfig::default()`. Add helper
      function if needed to reduce duplication.
changedFiles:
  - docs/configuration.md
  - docs/data_model.md
  - docs/job_queue.md
  - server/crates/ashford-core/src/config.rs
  - server/crates/ashford-core/src/jobs/backfill_gmail.rs
  - server/crates/ashford-core/src/jobs/classify.rs
  - server/crates/ashford-core/src/jobs/history_sync_gmail.rs
  - server/crates/ashford-core/src/jobs/ingest_gmail.rs
  - server/crates/ashford-core/src/jobs/mod.rs
  - server/crates/ashford-core/src/lib.rs
  - server/crates/ashford-core/src/llm/mock.rs
  - server/crates/ashford-core/src/messages.rs
  - server/crates/ashford-core/tests/ingest_flow.rs
  - server/crates/ashford-server/src/main.rs
tags: []
---

Final integration layer that wires together all components into the classify job and connects to the existing ingestion pipeline.

## Key Components

### Classify Job Handler
**Job type**: `classify`
**Payload**:
```json
{
  "account_id": "string",
  "message_id": "string"
}
```

### Orchestration Flow
```rust
pub async fn handle_classify(
    dispatcher: &JobDispatcher,
    job: Job,
) -> Result<(), JobError> {
    // 1. Parse payload
    let payload: ClassifyPayload = ...;
    
    // 2. Load message and account
    let message = message_repo.get_by_id(...).await?;
    let account = account_repo.get_by_id(...).await?;
    
    // 3. Try deterministic rules (fast path)
    let deterministic_result = rule_engine
        .evaluate_deterministic(&message)
        .await?;
    
    if let Some(matched_rules) = deterministic_result {
        // Create decision from deterministic rules
        // Apply safety gating
        // Persist decision
        // Enqueue action job
        return Ok(());
    }
    
    // 4. LLM path (slow path)
    // Load directions and LLM rules
    let directions = directions_repo.get_all_enabled().await?;
    let llm_rules = llm_rules_repo.get_by_scope(&message).await?;
    
    // 5. Build prompt and call LLM
    let prompt = prompt_builder.build(&message, &directions, &llm_rules);
    let llm_response = llm_provider.complete(prompt).await?;
    
    // 6. Parse decision
    let mut decision = parse_decision(llm_response)?;
    
    // 7. Apply safety enforcement
    let safety_result = safety_enforcer.enforce(&mut decision);
    
    // 8. Persist decision
    let decision_id = decision_repo.create(&decision).await?;
    
    // 9. Enqueue next job
    if decision.needs_approval {
        // Enqueue approval request job
        queue.enqueue("approval.request", ...).await?;
    } else {
        // Enqueue action execution job
        queue.enqueue("action.execute", ...).await?;
    }
    
    Ok(())
}
```

### Error Handling
- Message not found → `JobError::Fatal`
- Account not found → `JobError::Fatal`
- LLM provider error → Map per provider (retryable vs fatal)
- Decision parse error → `JobError::Fatal` (log for debugging)
- Database error → `JobError::Retryable`

### Integration with Ingest Job
Wire classify job into `handle_ingest_gmail`:
```rust
// After message is persisted...
let idempotency_key = format!("gmail:{}:{}:classify", account_id, message_id);
queue.enqueue(
    "classify",
    json!({ "account_id": account_id, "message_id": message_id }),
    Some(idempotency_key),
    0,  // priority
).await?;
```

### JobDispatcher Update
Add to dispatcher match:
```rust
match job.job_type.as_str() {
    "ingest.gmail" => handle_ingest_gmail(self, job).await,
    "history.sync.gmail" => handle_history_sync_gmail(self, job).await,
    "backfill.gmail" => handle_backfill_gmail(self, job).await,
    "classify" => handle_classify(self, job).await,  // NEW
    _ => Err(JobError::Fatal(...)),
}
```

### Dependency Injection
Ensure JobDispatcher has access to:
- LLM provider instance
- Rule engine components
- Safety enforcer

### File Organization
```
ashford-core/src/jobs/
├── classify.rs      # Classify job handler
├── mod.rs           # Update dispatcher
```

### Testing
- End-to-end classify flow with deterministic match
- End-to-end classify flow with LLM path
- Safety override application in context
- Error handling for various failure modes
- Idempotency key prevents duplicate classification

### Performance Considerations
- LLM latency tracking in telemetry
- Database query optimization (indices already exist)
- Consider batching for high-volume scenarios (future enhancement)

## Research

### Summary
- The classify job handler will orchestrate the full decision pipeline, connecting the deterministic rules engine, LLM decision engine, and safety enforcer to produce classifications for ingested email messages.
- The primary integration point is the existing job system, which follows well-established patterns for job dispatching, error handling, and enqueueing dependent jobs.
- Key dependencies are already implemented: RuleExecutor for deterministic rules, GenaiLLMClient + PromptBuilder for LLM decisions, SafetyEnforcer for safety gating, and comprehensive repository layer for persistence.
- The main implementation work involves wiring these components together in a new `classify.rs` job handler and modifying `ingest_gmail.rs` to enqueue classify jobs after message persistence.

### Findings

#### Job System Architecture (server/crates/ashford-core/src/jobs/)

**JobDispatcher Structure:**
```rust
#[derive(Clone)]
pub struct JobDispatcher {
    pub db: Database,
    pub http: reqwest::Client,
    pub gmail_api_base: Option<String>,
}
```

The dispatcher implements `JobExecutor` trait and routes jobs via a match statement in `mod.rs`. To add the classify job:
1. Add `mod classify;` to `mod.rs`
2. Add `use classify::handle_classify;`
3. Add constant `pub const JOB_TYPE_CLASSIFY: &str = "classify";`
4. Add match arm: `JOB_TYPE_CLASSIFY => handle_classify(self, job).await`

**Existing Job Type Constants:**
- `JOB_TYPE_BACKFILL_GMAIL` = "backfill.gmail"
- `JOB_TYPE_INGEST_GMAIL` = "ingest.gmail"
- `JOB_TYPE_HISTORY_SYNC_GMAIL` = "history.sync.gmail"

**Job Payload Pattern:**
All handlers use serde deserialization at the start:
```rust
let payload: ClassifyPayload = serde_json::from_value(job.payload.clone())
    .map_err(|err| JobError::Fatal(format!("invalid classify payload: {err}")))?;
```

**Error Mapping Utilities:**
- `map_gmail_error(context, err)` - Maps GmailClientError to JobError
- `map_llm_error(context, err)` - Maps LLMError to JobError (already exists, marked `#[allow(dead_code)]`)
- `map_account_error(context, err)` - Maps AccountError to JobError

The `map_llm_error` function handles:
- `RateLimited` → Retryable with retry_after duration
- `AuthenticationFailed` → Fatal
- `InvalidRequest` → Fatal
- `ServerError` → Retryable
- `Timeout` → Retryable
- `ParseError` → Fatal (decision parsing failures are fatal)
- `ProviderError` → Retryable

**Idempotency Pattern:**
Jobs use idempotency keys to prevent duplicates:
```rust
let idempotency_key = format!("{JOB_TYPE_INGEST_GMAIL}:{account_id}:{message_id}");
```
For classify: `format!("classify:{account_id}:{message_id}")`

**Priority System:**
- Priority 1: High (ingest jobs from backfill/history)
- Priority 0: Default/Normal
- Priority -10: Low (backfill continuation)

For classify, priority 0 is appropriate as it follows ingest.

#### Deterministic Rules Engine (server/crates/ashford-core/src/rules/)

**RuleExecutor API:**
```rust
pub struct RuleExecutor {
    loader: RuleLoader,
}

impl RuleExecutor {
    pub fn new(db: Database) -> Self;

    pub async fn evaluate(
        &self,
        org_id: i64,
        user_id: i64,
        message: &Message,
    ) -> Result<Option<RuleMatch>, ExecutorError>
}

pub struct RuleMatch {
    pub rule: DeterministicRule,
    pub action_type: String,
    pub action_parameters: Value,
    pub safe_mode: SafeMode,
}
```

**Evaluation Flow:**
1. RuleLoader collects rules by scope (global → account → domain → sender)
2. Rules sorted by priority (lower = higher priority), then created_at, then ID
3. First matching rule returns immediately (short-circuit)
4. No match returns `None` → continue to LLM path

**ExecutorError Mapping:**
- `ExecutorError::Database(DbError)` → Retryable
- `ExecutorError::Sql(libsql::Error)` → Retryable
- `ExecutorError::MessageMissingFrom` → Fatal (data corruption)

#### LLM Decision Engine (server/crates/ashford-core/src/llm/)

**Components:**

1. **PromptBuilder** (`prompt.rs`):
```rust
let builder = PromptBuilder::new();
let messages = builder.build(
    &message,       // &Message
    &directions,    // &[Direction]
    &llm_rules,     // &[LlmRule]
    None,           // Option<&ThreadContext> - pass None for now
);
```

Returns `Vec<ChatMessage>` with system and user messages.

2. **Decision Tool** (`decision.rs`):
```rust
let tool = build_decision_tool();  // Creates tool with JSON schema
// DECISION_TOOL_NAME = "record_decision"
```

3. **LLMClient** (`mod.rs`):
```rust
#[async_trait]
pub trait LLMClient: Send + Sync {
    async fn complete(
        &self,
        request: CompletionRequest,
        context: LlmCallContext,
    ) -> Result<CompletionResponse, LLMError>;
}

// Production implementation
let client = GenaiLLMClient::new(db, model_config);
```

**CompletionRequest:**
```rust
pub struct CompletionRequest {
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub json_mode: bool,
    pub tools: Vec<Tool>,
}
```

**Response Parsing:**
```rust
DecisionOutput::parse_from_tool_calls(&response.tool_calls, DECISION_TOOL_NAME)
```

Returns `Result<DecisionOutput, DecisionParseError>`.

**DecisionParseError Mapping:**
- `NoToolCall` → Fatal (LLM didn't call the tool)
- `WrongToolName` → Fatal (LLM called wrong tool)
- `NoJsonFound` → Fatal
- `MalformedJson` → Fatal
- `Json(serde_json::Error)` → Fatal
- `Validation(DecisionValidationError)` → Fatal

All decision parse errors are fatal because they indicate prompt/model issues that won't resolve with retry.

#### Safety Enforcement (server/crates/ashford-core/src/decisions/safety.rs)

**SafetyEnforcer API:**
```rust
pub struct SafetyEnforcer {
    policy: PolicyConfig,
}

impl SafetyEnforcer {
    pub fn new(policy: PolicyConfig) -> Self;
    pub fn enforce(&self, decision: &DecisionOutput) -> SafetyResult;
}

pub struct SafetyResult {
    pub overrides_applied: Vec<SafetyOverride>,
    pub requires_approval: bool,
}
```

**Enforcement Logic (OR semantics):**
1. Danger level check: `ActionType::danger_level() == Dangerous` → DangerousAction override
2. Confidence threshold: `confidence < policy.confidence_default` → LowConfidence override
3. approval_always list: action in list → InApprovalAlwaysList override
4. LLM advisory flag: `decision.needs_approval == true` → LlmRequestedApproval override

**Telemetry:**
```rust
let telemetry_json = result.to_telemetry_json();
// { "safety_overrides": [...], "requires_approval": true }
```

#### Repository Layer

**MessageRepository** (`messages.rs`):
```rust
let msg_repo = MessageRepository::new(db.clone());
let message = msg_repo.get_by_provider_id(
    org_id, user_id, &account_id, &provider_message_id
).await?;
```

Note: The classify job receives internal `message_id` (UUID), not `provider_message_id`. Need to add/use `get_by_id()` method.

**AccountRepository** (`accounts.rs`):
```rust
let account_repo = AccountRepository::new(db.clone());
let account = account_repo.get_by_id(org_id, user_id, &account_id).await?;
```

**DirectionsRepository** (`rules/repositories.rs`):
```rust
let directions_repo = DirectionsRepository::new(db.clone());
let directions = directions_repo.list_enabled(org_id, user_id).await?;
```

**LlmRuleRepository** (`rules/repositories.rs`):
```rust
let llm_rules_repo = LlmRuleRepository::new(db.clone());
// Need to load rules by multiple scopes based on message sender/domain
let rules = llm_rules_repo.list_enabled_by_scope(org_id, user_id, scope, scope_ref).await?;
```

**DecisionRepository** (`decisions/repositories.rs`):
```rust
let decision_repo = DecisionRepository::new(db.clone());
let decision = decision_repo.create(NewDecision { ... }).await?;
```

**ActionRepository** (`decisions/repositories.rs`):
```rust
let action_repo = ActionRepository::new(db.clone());
let action = action_repo.create(NewAction { ... }).await?;
```

#### Configuration (server/crates/ashford-core/src/config.rs)

**ModelConfig:**
```rust
pub struct ModelConfig {
    pub provider: String,      // "openai", "google", etc.
    pub model: String,         // "gpt-4o-mini"
    pub temperature: f32,      // Default: 0.2
    pub max_output_tokens: u32,
}
```

**PolicyConfig:**
```rust
pub struct PolicyConfig {
    pub approval_always: Vec<String>,  // ["delete", "forward", "auto_reply", "escalate"]
    pub confidence_default: f32,       // Default: 0.7
}
```

The classify job needs access to both configs. Options:
1. Load from config file in handler
2. Add to JobDispatcher struct
3. Pass via environment/static config

#### Integration Point: ingest_gmail.rs

The classify job should be enqueued at the end of `handle_ingest_gmail()`, after the message is successfully persisted:

```rust
// After msg_repo.upsert(new_msg).await?
let queue = JobQueue::new(dispatcher.db.clone());
let idempotency_key = format!("classify:{}:{}", account_id, message.id);
match queue.enqueue(
    JOB_TYPE_CLASSIFY,
    json!({ "account_id": payload.account_id, "message_id": message.id }),
    Some(idempotency_key),
    0,  // priority
).await {
    Ok(_) => {}
    Err(QueueError::DuplicateIdempotency { .. }) => {
        debug!("classify job already enqueued");
    }
    Err(err) => {
        return Err(JobError::retryable(format!("failed to enqueue classify job: {err}")));
    }
}
```

Note: The `message.id` here is the internal UUID returned from `msg_repo.upsert()`, not the Gmail `message_id`.

#### Files to Create/Modify

**New Files:**
- `server/crates/ashford-core/src/jobs/classify.rs` - Main classify job handler

**Modified Files:**
- `server/crates/ashford-core/src/jobs/mod.rs` - Add classify job routing
- `server/crates/ashford-core/src/jobs/ingest_gmail.rs` - Enqueue classify after ingest
- `server/crates/ashford-core/src/messages.rs` - May need `get_by_id()` if not present

#### JobDispatcher Dependencies

The classify job needs additional dependencies beyond the current JobDispatcher fields. Options:

**Option A: Expand JobDispatcher struct**
```rust
pub struct JobDispatcher {
    pub db: Database,
    pub http: reqwest::Client,
    pub gmail_api_base: Option<String>,
    // New fields
    pub llm_client: Arc<dyn LLMClient>,
    pub model_config: ModelConfig,
    pub policy_config: PolicyConfig,
}
```

**Option B: Create classify-specific context**
```rust
// In classify.rs
struct ClassifyContext {
    llm_client: GenaiLLMClient,
    rule_executor: RuleExecutor,
    safety_enforcer: SafetyEnforcer,
    prompt_builder: PromptBuilder,
    // ... repos
}
```

Option A is cleaner for dependency injection but requires modifying all existing job tests.
Option B is more isolated but creates redundant instantiation.

Recommendation: Option A with builder pattern additions to JobDispatcher.

#### Testing Strategy

**Unit Tests:**
1. Payload parsing (invalid payload → Fatal)
2. Message not found → Fatal
3. Account not found → Fatal

**Integration Tests (with mocks):**
1. Deterministic rule match → creates decision with source="deterministic", enqueues action
2. LLM path → creates decision with source="llm", enqueues action
3. Safety override → sets needs_approval=true
4. LLM error handling (rate limit → retryable, auth fail → fatal)
5. Decision parse error → Fatal
6. Idempotency (duplicate classify jobs handled gracefully)

**Test Fixtures Needed:**
- Mock LLM client (already exists: MockLLMClient in `llm/mock.rs`)
- Test message with various sender/domain scenarios
- Test deterministic rules
- Test directions and LLM rules

### Risks & Constraints

1. **JobDispatcher Expansion**: Adding LLM client and config to JobDispatcher will require updating all existing job handler tests to provide these new dependencies. Consider using `Option<Arc<dyn LLMClient>>` to make it non-breaking.

2. **Message ID Type**: The plan specifies `message_id` in the payload, but need to verify whether this is the internal UUID or the Gmail provider ID. The ingest job persists messages with internal UUIDs, so classify should receive and use the internal UUID.

3. **LLM Client Initialization**: The GenaiLLMClient requires a Database reference for logging LLM calls. This is already available in JobDispatcher.

4. **Config Loading**: The classify job needs ModelConfig and PolicyConfig. These should be loaded once at application startup and passed to JobDispatcher, not reloaded per-job.

5. **Rule Scoping**: LLM rules need to be loaded for multiple scopes (global, account, sender domain, sender email). The repository currently has `list_enabled_by_scope` which takes a single scope. May need to call multiple times and merge results.

6. **Action Enqueueing**: The plan mentions enqueueing "approval.request" or "action.execute" jobs, but these job types don't exist yet. For now, create the Action record with appropriate status (ApprovedPending or Queued) and defer job creation to a future plan.

7. **Decision Source Handling**: Deterministic matches need to be converted to DecisionOutput format for consistent safety enforcement and persistence. The RuleMatch struct has different fields than DecisionOutput.

8. **Thread Context**: The PromptBuilder accepts an optional ThreadContext, but this feature isn't implemented. Pass None for now.

9. **Error Granularity**: Some errors that are marked Fatal in the plan (e.g., DecisionParseError) could benefit from retrying with a fallback strategy (e.g., require approval for unparseable decisions). Consider this for future improvement.

10. **Telemetry**: The plan mentions OpenTelemetry traces. The LlmCallContext struct supports trace_id, but need to ensure trace propagation through the classify flow.

### Design Decisions

**JobDispatcher Expansion (Chosen Approach):**
Expand JobDispatcher with required fields for LLM client and policy config:
```rust
pub struct JobDispatcher {
    pub db: Database,
    pub http: reqwest::Client,
    pub gmail_api_base: Option<String>,
    // New required fields for classify job
    pub llm_client: Arc<dyn LLMClient>,
    pub policy_config: PolicyConfig,
}
```

Update `JobDispatcher::new()` to require these parameters. All existing tests will be updated to provide mock/default values. This is cleaner than optional fields since classify is a core job type.

# Implementation Notes

Completed foundational tasks (1, 2, 6, 15) for the classify job handler.

## Task 1: MessageRepository::get_by_id method
Added a new get_by_id(org_id, user_id, message_id) method to MessageRepository in server/crates/ashford-core/src/messages.rs. This method looks up messages by their internal UUID (as opposed to provider_message_id used by get_by_provider_id). The implementation follows the same query pattern as get_by_provider_id, with proper org/user scoping. Returns MessageError::NotFound if the message doesn't exist. Added 3 tests: get_by_id_fetches_message, get_by_id_returns_not_found_for_missing, get_by_id_scopes_to_org_and_user.

## Task 2: JobDispatcher Expansion
Modified server/crates/ashford-core/src/jobs/mod.rs to add two new required fields to JobDispatcher:
- llm_client: Arc<dyn LLMClient> - for LLM API access in classify jobs
- policy_config: PolicyConfig - for safety enforcement configuration

Updated JobDispatcher::new() signature to require these parameters. The with_gmail_api_base() builder method remains unchanged. Also added Default implementation for PolicyConfig in config.rs with sensible defaults (approval_always includes delete, forward, auto_reply, escalate; confidence_default = 0.7).

## Task 6: Error Mapping for Rules Engine
Added map_executor_error(context, err) function in server/crates/ashford-core/src/jobs/mod.rs. Note: The actual ExecutorError enum has RuleLoader and Condition variants (not Database/Sql/MessageMissingFrom as described in the plan). The implementation correctly maps:
- ExecutorError::RuleLoader -> Retryable (wraps database errors)
- ExecutorError::Condition -> Fatal (invalid regex, missing field - data issues)
Added #[allow(dead_code)] annotation since the function will be used when classify job is implemented. Added 2 tests for the mapping.

## Task 15: Job Test Updates
Updated all existing job tests to use the new JobDispatcher::new() signature:
- server/crates/ashford-core/src/jobs/mod.rs - test_dispatcher helper function
- server/crates/ashford-core/src/jobs/ingest_gmail.rs - setup_account helper
- server/crates/ashford-core/src/jobs/history_sync_gmail.rs - setup_account helper
- server/crates/ashford-core/src/jobs/backfill_gmail.rs - setup_account helper
- server/crates/ashford-core/tests/ingest_flow.rs - integration test setup

All tests now use Arc::new(MockLLMClient::new()) and PolicyConfig::default() for the new required parameters.

## Additional Changes
- server/crates/ashford-server/src/main.rs - Updated to create GenaiLLMClient with config's model settings and pass to JobDispatcher

All 311 tests pass. Build succeeds without warnings.

Implemented core classify job handler (Tasks 3, 4, 5, 7).

## Task 3: load_llm_rules_for_message() helper
Created helper function in server/crates/ashford-core/src/jobs/classify.rs (lines 180-238) that loads LLM rules from all applicable scopes based on a message:
- Global rules (scope=Global, scope_ref=None)
- Account rules (scope=Account, scope_ref=account_id)
- Domain rules (scope=Domain, scope_ref=sender_domain extracted via extract_domain helper)
- Sender rules (scope=Sender, scope_ref=sender_email lowercased)
The function uses HashSet to deduplicate rules by ID when merging results from multiple scopes. Added extract_domain() helper to parse domain from email addresses. Three unit tests verify: loading from all scopes, handling missing sender, and deduplication.

## Task 4: rule_match_to_decision_output() converter
Created converter function (lines 246-291) that transforms deterministic RuleMatch into DecisionOutput for consistent handling:
- Sets confidence to 1.0 (deterministic match)
- Computes needs_approval based on SafeMode: DangerousOverride and AlwaysSafe bypass approval, Default checks ActionType::danger_level()
- Generates appropriate rationale describing the matched rule
- Sets empty explanations (salient_features, matched_directions, alternatives)
- Generates undo hints via generate_undo_hint() helper for all action types (archive, mark_read, delete, label, move_folder, forward, auto_reply, escalate, no_action)
- Sets empty telemetry placeholder
Five unit tests cover confidence setting, safe_mode handling, and undo hint generation.

## Task 5: handle_classify() main handler
Created the main classify job handler (lines 50-169) implementing the full orchestration flow:
1. Parses ClassifyPayload struct with account_id and message_id (internal UUID)
2. Loads message via MessageRepository::get_by_id, returns Fatal if not found
3. Loads account via AccountRepository::get_by_id for validation, returns Fatal if not found
4. Evaluates deterministic rules via RuleExecutor as fast path
5. If deterministic match: converts to DecisionOutput, applies SafetyEnforcer, persists Decision (source=Deterministic) and Action records
6. If no match: loads directions via DirectionsRepository, loads LLM rules via load_llm_rules_for_message helper, builds prompt via PromptBuilder, calls LLM
7. Parses LLM response via DecisionOutput::parse_from_tool_calls
8. Applies SafetyEnforcer to set final needs_approval flag
9. Persists Decision (source=Llm) and Action records
10. Sets Action status to ApprovedPending if approval required, Queued otherwise
11. Uses map_llm_error, map_account_error, map_executor_error for proper error handling
12. Logs classification results with tracing

Three error handling tests verify: invalid payload returns Fatal, message not found returns Fatal, account not found returns Fatal.

## Task 7: Dispatcher registration
Modified server/crates/ashford-core/src/jobs/mod.rs:
- Added mod classify; declaration
- Added use classify::handle_classify;
- Added pub const JOB_TYPE_CLASSIFY: &str = "classify";
- Added match arm in JobExecutor::execute for classify job type
Modified server/crates/ashford-core/src/lib.rs to export JOB_TYPE_CLASSIFY.

## Design Decisions
- SafetyEnforcer is applied to both deterministic and LLM decisions for consistent policy enforcement
- Action records created with status Queued or ApprovedPending (action execution jobs deferred to future plan)
- Provider hardcoded to "gmail" since only Gmail is currently supported
- Sender email lowercased for case-insensitive rule matching

All 322 tests pass. Build succeeds without warnings.

Completed Tasks 8, 9, and 14 for classify job integration with ingest pipeline.

## Task 8: Wire classify job into ingest_gmail handler
Modified server/crates/ashford-core/src/jobs/ingest_gmail.rs to enqueue a classify job after message persistence:

1. Added imports for debug tracing macro, JOB_TYPE_CLASSIFY, JobQueue, and QueueError
2. Changed msg_repo.upsert() to capture the returned Message struct as persisted_msg
3. Added enqueue_classify_job() helper function (lines 168-190) that:
   - Creates JSON payload with account_id and internal message_id (UUID)
   - Uses idempotency key format "{JOB_TYPE_CLASSIFY}:{account_id}:{message_id}"
   - Handles QueueError::DuplicateIdempotency silently with debug log
   - Returns retryable JobError for other queue errors
   - Uses priority 0 for classify jobs
4. Calls enqueue_classify_job() after successful message upsert (line 137)

## Task 9: Unit tests for classify job
Verified that the unit tests for classify job error handling were already implemented in the previous batch (Tasks 3,4,5,7). The tests exist in classify.rs:
- classify_invalid_payload_returns_fatal (line 643)
- classify_message_not_found_returns_fatal (line 670)
- classify_account_not_found_returns_fatal (line 706)

## Task 14: Update ingest_gmail tests for classify enqueueing
Added/updated tests in ingest_gmail.rs:

1. Updated ingest_fetches_and_persists_message test (lines 325-349):
   - Queries database directly to verify classify job was enqueued
   - Verifies payload contains correct account_id and internal message_id (UUID)
   - Verifies job priority is 0

2. Added ingest_does_not_enqueue_classify_on_failure test (lines 469-513):
   - Simulates Gmail API 404 failure
   - Verifies ingest returns Fatal error
   - Confirms no classify job was enqueued on failure

3. Added ingest_handles_duplicate_classify_idempotency test (lines 515-581):
   - Runs ingest twice for the same Gmail message
   - Verifies second ingest succeeds (duplicate idempotency handled gracefully)
   - Confirms only one classify job exists due to idempotency key

## Design Decisions
- Used internal UUID (persisted_msg.id) for classify job payload, not Gmail provider_message_id
- Idempotency key format follows existing pattern: JOB_TYPE:account_id:message_id
- Separate helper function enqueue_classify_job() for cleaner code organization

All 324 tests pass. Build succeeds without warnings.

Completed integration tests (Tasks 10, 11, 12, 13) for the classify job handler.

## Task 10: Integration test for deterministic rule path
Added two integration tests in server/crates/ashford-core/src/jobs/classify.rs:
- classify_deterministic_rule_match_creates_decision_and_action: Sets up test database with a message, account, and deterministic rule matching sender email. Creates JobDispatcher with mock LLM client. Verifies handle_classify creates a Decision with source=Deterministic and confidence=1.0, creates an Action with status=Queued (safe action), and confirms LLM was NOT called via MockLLMClient.call_count() == 0.
- classify_deterministic_dangerous_action_requires_approval: Tests that dangerous actions (delete) from deterministic rules result in needs_approval=true and Action status=ApprovedPending.

## Task 11: Integration test for LLM decision path
Added classify_llm_path_creates_decision_and_action test that sets up database with message, account, directions, and LLM rules but no deterministic rules. Configures MockLLMClient to return a valid DecisionOutput with tool call. Verifies handle_classify creates Decision with source=Llm, proper confidence and action_type, and confirms LLM was called exactly once.

## Task 12: Integration test for safety enforcement
Added classify_safety_enforcement_overrides_to_require_approval test. Sets up scenario where LLM returns a dangerous action (delete) with low confidence (0.5). Verifies SafetyEnforcer overrides needs_approval to true regardless of LLM's needs_approval value. Confirms Action has status=ApprovedPending and telemetry_json contains safety_overrides array with both 'dangerous' and 'confidence' override entries.

## Task 13: Integration test for LLM error handling
Added four tests for LLM error scenarios:
- classify_llm_rate_limited_returns_retryable: Configures MockLLMClient to return RateLimited error with 60s retry_after. Verifies JobError::Retryable is returned with correct retry_after duration.
- classify_llm_authentication_failed_returns_fatal: Verifies AuthenticationFailed error maps to JobError::Fatal.
- classify_llm_server_error_returns_retryable: Verifies ServerError maps to JobError::Retryable.
- classify_llm_no_tool_call_returns_fatal: Configures MockLLMClient to return response without tool calls, verifying decision parse error (NoToolCall) returns JobError::Fatal.

## MockLLMClient Enhancement
Modified server/crates/ashford-core/src/llm/mock.rs to add call tracking:
- Added call_count field using Arc<AtomicUsize> for thread-safe counting
- Added call_count() method that returns how many times complete() was called
- Updated complete() method to increment counter on each invocation
- Added test call_count_tracks_invocations to verify the new functionality

## Helper Functions Added
- create_sender_rule(): Creates a deterministic rule matching a specific sender email with configurable action type and safe_mode
- build_test_decision_output(): Builds a valid DecisionOutput for testing LLM responses with configurable action, confidence, and needs_approval

All 333 tests pass. Build compiles without warnings.

Addressed review issues for Phase 4.6 by wiring classify into downstream job pipeline, tightening account validation, and ensuring persisted decisions reflect safety overrides. Implemented follow-up job enqueueing in classify.rs so successful classifications now queue either action execution (action.gmail) or approval notifications (approval.notify) with idempotency handling; added helper enqueue_follow_up_job and queue imports. Added new job handlers action_gmail.rs and approval_notify.rs plus dispatcher constants/arms in jobs/mod.rs and exports in lib.rs. The action handler currently marks queued actions executing/completed as a stub; approval handler logs pending approvals while validating account ownership. Added explicit account/message ownership check in handle_classify to prevent cross-account classification. Updated safety enforcement flow to mutate decision_output.needs_approval before serialization so decision_json stays in sync with SafetyEnforcer results and added assertion in safety test. Added tests for account mismatch, action/approval job enqueueing, and ensured safety test checks stored needs_approval flag; adjusted test helpers to create unique accounts. All classify_* tests pass (cargo test -p ashford-core classify_).

Addressed reviewer follow-up handler issues: added map_action_error in server/crates/ashford-core/src/jobs/mod.rs to classify ActionError variants into fatal vs retryable so missing actions don’t loop forever; handle_action_gmail now processes actions in Executing by directly marking them Completed while keeping Queued->Executing->Completed transitions for fresh work, preventing retries from stalling after partial progress; get_by_id failures in action_gmail.rs and approval_notify.rs now use map_action_error so NotFound/invalid state returns Fatal instead of Retryable. Tested with cargo test -p ashford-core action_gmail (from server/). Tasks: reviewer fixes for action.gmail/approval.notify follow-up pipeline reliability.

## SafeMode Bypass Fix (Review Issue Resolution)

Fixed safety mode bypass issues identified in code review where SafeMode::DangerousOverride and SafeMode::AlwaysSafe on deterministic rules weren't actually bypassing safety enforcement.

### Problem
The original implementation in handle_classify() unconditionally applied SafetyEnforcer::enforce() after converting deterministic rule matches to DecisionOutput. This defeated the purpose of SafeMode overrides because:
1. DangerousOverride rules with dangerous actions (delete) still triggered DangerousAction safety override
2. AlwaysSafe rules with actions in approval_always list still triggered InApprovalAlwaysList override

### Solution
Modified server/crates/ashford-core/src/jobs/classify.rs:
1. Added check for skip_safety_enforcement flag based on rule_match.safe_mode
2. For SafeMode::DangerousOverride or SafeMode::AlwaysSafe, skip SafetyEnforcer::enforce() entirely
3. Use needs_approval value from rule_match_to_decision_output directly (which already respects SafeMode)
4. For SafeMode::Default, continue to apply safety enforcement as before

### Tests Added
- classify_dangerous_override_bypasses_safety_enforcement: Verifies DangerousOverride with delete action doesn't require approval
- classify_always_safe_bypasses_safety_enforcement: Verifies AlwaysSafe with forward action (in approval_always list) doesn't require approval

All 338 tests pass after the fix.
