---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Phase 4.4: Prompt Construction & LLM Decision Engine"
goal: Build the 5-layer prompt construction system and LLM decision
  parsing/validation
id: 16
uuid: b8c142c5-3335-4b87-9a94-28dbcc96af99
generatedBy: agent
status: done
priority: high
container: false
temp: false
dependencies:
  - 13
  - 15
parent: 4
references: {}
issue: []
pullRequest: []
docs:
  - docs/decision_engine.md
  - docs/rules_engine.md
planGeneratedAt: 2025-12-02T07:41:41.065Z
promptsGeneratedAt: 2025-12-02T07:41:41.065Z
createdAt: 2025-11-30T01:14:19.216Z
updatedAt: 2025-12-02T08:13:20.475Z
progressNotes:
  - timestamp: 2025-12-02T07:50:39.924Z
    text: Implemented decision types, validation, parsing helpers and tests; added
      llm::decision module wiring.
    source: "implementer: tasks1-5"
  - timestamp: 2025-12-02T07:52:23.917Z
    text: Added additional decision parser validation tests (boundary confidences,
      empty rationale/why_not, code fence without language) and reran cargo test
      -p ashford-core decision (pass).
    source: "tester: phase-4-4"
  - timestamp: 2025-12-02T08:01:55.984Z
    text: Implemented PromptBuilder with all five layers, body truncation/html
      stripping, header filtering, and task directive. Added html2text
      dependency, module exports, unit tests in prompt.rs, plus integration test
      covering prompt-to-decision flow. Ran cargo fmt and cargo test -p
      ashford-core (all pass).
    source: "implementer: tasks6-16"
  - timestamp: 2025-12-02T08:05:07.386Z
    text: Added prompt.rs test coverage for truncation limits, case-insensitive
      header filtering, missing-field message contexts, task directive action
      list, and empty directions/rules; reran cargo test -p ashford-core
      successfully.
    source: "tester: phase-4-4"
  - timestamp: 2025-12-02T08:12:10.325Z
    text: Completed code review. All 264 tests pass. Implementation follows plan
      requirements well. Found one documentation inconsistency regarding
      inverse_action types in undo_hint - the docs specify additional
      inverse-specific actions (unapply_label, restore, etc.) but implementation
      reuses ActionType enum. This is acceptable since the Rust-side docs in
      decision_engine.md were updated to match the implementation.
    source: "reviewer: Phase 4.4"
  - timestamp: 2025-12-02T08:13:20.470Z
    text: Ran cargo test -p ashford-core (all tests) to validate new prompt/decision
      modules; suite passed.
    source: "reviewer: Phase 4.4 prompt/decision"
tasks:
  - title: Create ActionType enum and decision types
    done: true
    description: >-
      Create `server/crates/ashford-core/src/llm/decision.rs` with:

      - `ActionType` enum with 15 variants (ApplyLabel, MarkRead, MarkUnread,
      Archive, Delete, Move, Star, Unstar, Forward, AutoReply, CreateTask,
      Snooze, AddNote, Escalate, None)

      - Use `#[serde(rename_all = "snake_case")]` for JSON serialization

      - Implement `as_str()` and `from_str()` methods for string conversion

      - Add unit tests for serialization round-trips
  - title: Create DecisionOutput and supporting structs
    done: true
    description: >-
      In `decision.rs`, add the full decision contract structs:

      - `MessageRef` with provider, account_id, thread_id, message_id

      - `DecisionDetails` with action, parameters, confidence (f64),
      needs_approval, rationale

      - `Explanations` with salient_features, matched_directions,
      considered_alternatives

      - `ConsideredAlternative` with action, confidence, why_not

      - `UndoHint` with inverse_action, inverse_parameters

      - `TelemetryPlaceholder` (empty struct for future extension)

      - `DecisionOutput` combining all of the above

      - All structs derive Serialize, Deserialize, Debug, Clone, PartialEq
  - title: Implement decision validation
    done: true
    description: |-
      In `decision.rs`, add validation logic:
      - `DecisionOutput::validate()` method that checks:
        - confidence is in range [0.0, 1.0]
        - action type is valid (already enforced by enum)
        - required string fields are non-empty
      - Create `DecisionValidationError` enum with specific error variants
      - Add unit tests for valid and invalid decisions
  - title: Implement JSON extraction from LLM response
    done: true
    description: >-
      In `decision.rs`, add `extract_json_from_response(response: &str) ->
      Result<&str, DecisionParseError>`:

      - Handle responses with markdown code blocks (```json ... ```)

      - Handle responses with extra text before/after JSON

      - Find first `{` and matching `}`

      - Return slice containing just the JSON

      - Add `DecisionParseError` enum with NoJsonFound, MalformedJson variants

      - Unit tests for various LLM response formats
  - title: Implement DecisionOutput parsing
    done: true
    description: >-
      In `decision.rs`, add `DecisionOutput::parse(response: &str) ->
      Result<DecisionOutput, DecisionParseError>`:

      - Extract JSON using `extract_json_from_response()`

      - Parse with serde_json

      - Validate with `validate()`

      - Return parsed and validated decision

      - Unit tests with valid JSON, malformed JSON, missing fields, invalid
      values
  - title: Create PromptBuilder struct and configuration
    done: true
    description: |-
      Create `server/crates/ashford-core/src/llm/prompt.rs` with:
      - `PromptBuilder` struct with configuration fields:
        - `max_body_length: usize` (default 8000)
        - `max_subject_length: usize` (default 500)
      - `PromptBuilderConfig` for optional configuration
      - `ThreadContext` placeholder struct (empty for now, for API stability)
      - Builder pattern: `PromptBuilder::new()` and `with_config()`
  - title: Implement Layer 1 - System Message
    done: true
    description: >-
      In `prompt.rs`, add private method `build_system_message() ->
      ChatMessage`:

      - Role definition: "You are the email classification and action engine"

      - Output contract: "produce a single JSON decision object"

      - Constraints: "MUST follow DIRECTIONS", "MUST NOT hallucinate"

      - Safety guidance: "If uncertain, choose safe and reversible action"

      - Return ChatMessage with ChatRole::System
  - title: Implement Layer 2 - Directions formatting
    done: true
    description: >-
      In `prompt.rs`, add private method `build_directions_section(directions:
      &[Direction]) -> String`:

      - Format as "DIRECTIONS:\n1. <content>\n2. <content>..."

      - Handle empty directions list gracefully (return empty string or skip
      section)

      - Each direction's content on its own numbered line

      - Unit tests with 0, 1, and multiple directions
  - title: Implement Layer 3 - LLM Rules formatting
    done: true
    description: >-
      In `prompt.rs`, add private method `build_llm_rules_section(rules:
      &[LlmRule]) -> String`:

      - Format each rule as:
        ```
        LLM RULE: <name>
        <description>
        <rule_text>
        ```
      - Handle empty rules list gracefully

      - Handle missing description (skip that line)

      - Unit tests with 0, 1, and multiple rules
  - title: Implement body text processing utilities
    done: true
    description: >-
      In `prompt.rs`, add helper functions:


      - `truncate_text(text: &str, max_len: usize) -> String` - truncate with
      "..." suffix at word boundary if possible


      - `strip_html(html: &str) -> String` - use `html2text` crate for robust
      HTML-to-text conversion:
        - Handles HTML entities (&amp;, &nbsp;, etc.)
        - Preserves paragraph structure
        - Strips scripts/styles
        - Normalizes whitespace

      - `get_body_text(message: &Message, max_len: usize) -> Option<String>` -
      prefer body_plain, fall back to stripped body_html


      - Add `html2text` to Cargo.toml dependencies


      - Unit tests for truncation, HTML stripping with complex email HTML
      (tables, nested divs, entities)
  - title: Implement header filtering
    done: true
    description: >-
      In `prompt.rs`, add helper function `filter_relevant_headers(headers:
      &[Header]) -> Vec<&Header>`:

      - Whitelist of useful headers: List-Id, Return-Path, X-Priority, X-Mailer,
      Reply-To, Precedence

      - Case-insensitive header name matching

      - Return filtered slice

      - Unit tests with mixed headers
  - title: Implement Layer 4 - Message Context formatting
    done: true
    description: >-
      In `prompt.rs`, add private method `build_message_context(message:
      &Message, thread_context: Option<&ThreadContext>) -> String`:

      - Format From with name and email

      - Format To/CC/BCC as comma-separated lists

      - Include Subject (truncated)

      - Include Snippet

      - Include filtered headers

      - Include Labels as JSON array

      - Include Body (truncated, HTML-stripped if needed)

      - Skip thread context for now (placeholder for future)

      - Unit tests with various message configurations
  - title: Implement Layer 5 - Task Directive
    done: true
    description: |-
      In `prompt.rs`, add private method `build_task_directive() -> String`:
      - Specify expected JSON schema structure
      - List valid action types
      - Specify confidence constraints ([0.0, 1.0])
      - Include approval logic hints
      - Include undo_hint expectations
  - title: Implement PromptBuilder::build() method
    done: true
    description: |-
      In `prompt.rs`, implement the main build method:
      ```rust
      pub fn build(
          &self,
          message: &Message,
          directions: &[Direction],
          llm_rules: &[LlmRule],
          thread_context: Option<&ThreadContext>,
      ) -> Vec<ChatMessage>
      ```
      - Combine all 5 layers into a Vec<ChatMessage>
      - Layer 1 as System message
      - Layers 2-5 combined into a single User message
      - Return the message list ready for CompletionRequest
  - title: Add module exports and integrate with LLM module
    done: true
    description: |-
      Update `server/crates/ashford-core/src/llm/mod.rs`:
      - Add `mod decision;` and `mod prompt;`
      - Add public exports for key types:
        - `DecisionOutput`, `ActionType`, `DecisionDetails`, `MessageRef`, etc.
        - `PromptBuilder`, `PromptBuilderConfig`, `ThreadContext`
        - Error types
      - Ensure all new types are accessible from crate root
  - title: Add integration test for full prompt-to-decision flow
    done: true
    description: >-
      Create test in `server/crates/ashford-core/src/llm/` or tests directory:

      - Build a prompt using PromptBuilder with sample message, directions,
      rules

      - Verify prompt structure (correct number of messages, contains expected
      sections)

      - Parse a sample valid LLM response into DecisionOutput

      - Verify all fields populated correctly

      - Test error handling with malformed response
changedFiles:
  - docs/decision_engine.md
  - server/Cargo.lock
  - server/crates/ashford-core/Cargo.toml
  - server/crates/ashford-core/src/llm/decision.rs
  - server/crates/ashford-core/src/llm/mod.rs
  - server/crates/ashford-core/src/llm/prompt.rs
  - server/crates/ashford-core/tests/llm_prompt_decision_flow.rs
tags: []
---

Core LLM decision engine that constructs prompts from rules/directions and parses model output into structured decisions.

## Key Components

### Prompt Builder (5-Layer Structure)

**Layer 1 - System Message**:
```
You are the email classification and action engine.
Your task is to produce a single JSON decision object following the required schema.
You MUST follow the DIRECTIONS section strictly.
You MUST NOT hallucinate.
If uncertain, choose a safe and reversible action.
```

**Layer 2 - DIRECTIONS**:
- Load all enabled directions from DirectionsRepository
- Format as numbered list:
```
DIRECTIONS:
1. Never delete or permanently remove email unless explicitly allowed.
2. When uncertain, prefer labeling or archiving over destructive actions.
...
```

**Layer 3 - LLM RULES**:
- Load applicable LLM rules by scope
- Format each rule:
```
LLM RULE: <name>
<description>
<rule_text>
```

**Layer 4 - MESSAGE CONTEXT**:
- From/To/CC/BCC
- Subject
- Snippet
- Relevant headers (List-Id, Return-Path, etc.)
- Current labels
- Body text (sanitized, truncated if needed)
- Thread summary (if available)

**Layer 5 - TASK Directive**:
- Specify exact JSON schema expected
- Include valid action types
- Include confidence constraints
- Include approval logic hints

### Decision JSON Contract (Serde Structs)
```rust
pub struct DecisionOutput {
    pub message_ref: MessageRef,
    pub decision: DecisionDetails,
    pub explanations: Explanations,
    pub undo_hint: UndoHint,
    pub telemetry: TelemetryPlaceholder,
}

pub struct DecisionDetails {
    pub action: ActionType,
    pub parameters: Value,  // Action-specific params
    pub confidence: f32,
    pub needs_approval: bool,
    pub rationale: String,
}
```

### Action Types Enum
```rust
pub enum ActionType {
    ApplyLabel,
    MarkRead,
    MarkUnread,
    Archive,
    Delete,
    Move,
    Star,
    Unstar,
    Forward,
    AutoReply,
    CreateTask,
    Snooze,
    AddNote,
    Escalate,
    None,
}
```

### Prompt Building API
```rust
pub struct PromptBuilder {
    // ...
}

impl PromptBuilder {
    pub async fn build(
        &self,
        message: &Message,
        directions: &[Direction],
        llm_rules: &[LLMRule],
        thread_context: Option<&ThreadContext>,
    ) -> Vec<ChatMessage>;
}
```

### Decision Parsing
- Parse JSON response with serde_json
- Validate all required fields present
- Validate confidence in [0.0, 1.0]
- Validate action type is known
- Handle partial/malformed responses gracefully

### File Organization
```
ashford-core/src/llm/
├── prompt.rs        # 5-layer prompt builder
├── decision.rs      # Decision structs and parsing
```

### Testing
- Prompt construction unit tests
- Decision parsing with valid/invalid JSON
- Fuzzy response handling tests

## Research

### Summary
- The codebase already has a well-structured LLM integration layer in `server/crates/ashford-core/src/llm/` with provider abstraction, types, error handling, and call logging.
- The rules engine in `server/crates/ashford-core/src/rules/` provides `Direction`, `LlmRule`, and their repositories with scope-based loading already implemented.
- The decisions module in `server/crates/ashford-core/src/decisions/` has `Decision` and `Action` types with flexible string-based action types and JSON parameters.
- The message structure in `server/crates/ashford-core/src/messages.rs` contains all fields needed for prompt construction (from/to/cc/bcc, subject, headers, labels, body_plain, body_html).
- This plan needs to add two new files (`prompt.rs` and `decision.rs`) to the LLM module and integrate with existing infrastructure.

### Findings

#### LLM Module Infrastructure (server/crates/ashford-core/src/llm/)

**Existing Files:**
- `mod.rs` (775 lines) - Core client implementation with `LLMClient` trait, `GenaiLLMClient` implementation using the `genai` crate
- `types.rs` (116 lines) - Core types: `ChatMessage`, `ChatRole`, `CompletionRequest`, `CompletionResponse`
- `error.rs` (83 lines) - `LLMError` enum with rate limiting, auth, parse errors
- `repository.rs` (437 lines) - `LlmCallRepository` for persisting all LLM calls with full request/response
- `mock.rs` (104 lines) - Test double for mocking LLM responses

**Key Types to Use:**
```rust
pub struct ChatMessage {
    pub role: ChatRole,  // System, User, Assistant
    pub content: String,
}

pub struct CompletionRequest {
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub json_mode: bool,
}

pub struct CompletionResponse {
    pub content: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub latency_ms: u64,
}
```

**LlmCallContext for Logging:**
```rust
pub struct LlmCallContext {
    pub feature: String,          // e.g., "classification"
    pub org_id: Option<i64>,
    pub user_id: Option<i64>,
    pub account_id: Option<String>,
    pub message_id: Option<String>,
    pub thread_id: Option<String>,
    pub rule_name: Option<String>,
    pub rule_id: Option<String>,
}
```

#### Rules Engine (server/crates/ashford-core/src/rules/)

**Existing Files:**
- `types.rs` - `Direction`, `LlmRule`, `DeterministicRule`, `RuleScope`, `SafeMode`
- `repositories.rs` - `DirectionsRepository`, `LlmRuleRepository`, `DeterministicRuleRepository`
- `deterministic.rs` - `RuleLoader` for loading applicable rules by scope
- `conditions.rs` - Condition parsing and evaluation

**Direction Structure:**
```rust
pub struct Direction {
    pub id: String,
    pub org_id: i64,
    pub user_id: Option<i64>,
    pub content: String,  // The direction text
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**LlmRule Structure:**
```rust
pub struct LlmRule {
    pub id: String,
    pub org_id: i64,
    pub user_id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub scope: RuleScope,  // Global, Account, Sender, Domain
    pub scope_ref: Option<String>,
    pub rule_text: String,  // Natural language rule
    pub enabled: bool,
    pub metadata_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**Repository Methods Available:**
- `DirectionsRepository::list_enabled(org_id, user_id)` - Returns enabled directions
- `LlmRuleRepository::list_enabled_by_scope(org_id, user_id, scope, scope_ref)` - Returns enabled rules by scope
- `RuleLoader::load_applicable_rules(org_id, user_id, account_id, sender_email)` - Loads rules across all applicable scopes

#### Message Structure (server/crates/ashford-core/src/messages.rs)

**Message Fields Available for Prompt:**
```rust
pub struct Message {
    pub id: String,
    pub account_id: String,
    pub thread_id: String,
    pub from_email: Option<String>,
    pub from_name: Option<String>,
    pub to: Vec<Mailbox>,      // {email, name?}
    pub cc: Vec<Mailbox>,
    pub bcc: Vec<Mailbox>,
    pub subject: Option<String>,
    pub snippet: Option<String>,
    pub labels: Vec<String>,   // Gmail labels like "INBOX", "STARRED"
    pub headers: Vec<Header>,  // {name, value}
    pub body_plain: Option<String>,
    pub body_html: Option<String>,
    // ... timestamps, org_id, user_id
}
```

**Relevant Headers to Extract:**
- `List-Id` - Mailing list identification
- `Return-Path` - Bounce address
- `X-Priority` - Email priority
- `X-Mailer` - Sending client
- Custom headers for rule matching

#### Decisions Module (server/crates/ashford-core/src/decisions/)

**Decision Structure:**
```rust
pub struct Decision {
    pub id: String,
    pub org_id: i64,
    pub user_id: i64,
    pub account_id: String,
    pub message_id: String,
    pub source: DecisionSource,  // Llm or Deterministic
    pub decision_json: Value,    // Full decision payload
    pub action_type: Option<String>,  // Flexible string type
    pub confidence: Option<f64>,
    pub needs_approval: bool,
    pub rationale: Option<String>,
    pub telemetry_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**Action Structure:**
```rust
pub struct Action {
    pub id: String,
    pub action_type: String,        // Flexible string type
    pub parameters_json: Value,     // Action-specific params
    pub status: ActionStatus,       // Queued, Executing, Completed, etc.
    pub undo_hint_json: Value,      // Reversibility info
    pub trace_id: Option<String>,   // OpenTelemetry trace
    // ... other fields
}
```

**Key Insight:** The system uses **flexible string-based action types** rather than enums. This allows extensibility but means validation must happen at parse time.

#### Thread Context (server/crates/ashford-core/src/threads.rs)

**Thread Structure:**
```rust
pub struct Thread {
    pub id: String,
    pub account_id: String,
    pub provider_thread_id: String,
    pub subject: Option<String>,
    pub snippet: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub metadata_json: Value,
    pub raw_json: Value,
    // ... org_id, user_id, timestamps
}
```

**Note:** Thread context in the current schema is limited. A `ThreadContext` struct may need to be created to hold summarized thread history for the prompt.

### Risks & Constraints

#### Architectural Considerations

1. **Action Type Flexibility vs. Type Safety:**
   - The plan specifies an `ActionType` enum, but existing `Decision.action_type` and `Action.action_type` are strings
   - **Recommendation:** Create the `ActionType` enum for LLM response parsing, but convert to/from strings when persisting to match existing schema
   - This provides compile-time safety for LLM output parsing while maintaining backward compatibility

2. **Decision JSON Contract Alignment:**
   - The plan's `DecisionOutput` struct differs slightly from the existing `Decision` struct
   - `DecisionOutput` is the **LLM response format** (what we parse from the model)
   - `Decision` is the **database record format** (what we persist)
   - Need clear mapping between them

3. **Confidence Type Mismatch:**
   - Plan specifies `confidence: f32`
   - Existing `Decision.confidence` is `Option<f64>`
   - **Decision:** Use `f64` throughout for consistency with existing schema

4. **Thread Context Deferred:**
   - Plan references `ThreadContext` but current `Thread` struct is minimal
   - **Decision:** Defer thread context to a follow-up plan
   - PromptBuilder will accept `Option<ThreadContext>` but implementation will pass `None`
   - Define a placeholder `ThreadContext` struct for API stability

5. **Body Truncation Strategy:**
   - Email bodies can be very large
   - Need configurable max length (suggest 4000-8000 chars for token efficiency)
   - Need HTML sanitization (strip tags, normalize whitespace)
   - Consider extracting plain text from HTML if body_plain is missing

6. **Header Selection:**
   - Not all headers are useful for classification
   - Define a whitelist: From, To, Cc, Subject, List-Id, Return-Path, X-Priority
   - Or blacklist: exclude large/binary headers

#### Testing Considerations

1. **Mock LLM Responses:**
   - `server/crates/ashford-core/src/llm/mock.rs` exists for testing
   - Use `MockChatExecutor` for unit tests of decision parsing

2. **Malformed Response Handling:**
   - LLMs sometimes return partial JSON, extra text before/after JSON, or truncated responses
   - Need robust extraction: find JSON in response, handle missing fields gracefully
   - Consider fallback to "needs_approval: true" on any parse error

3. **Edge Cases for Prompts:**
   - Empty directions list
   - Empty LLM rules list
   - Missing message fields (no subject, no body)
   - Very long bodies requiring truncation

#### Dependencies

- Depends on Plan 13 (rules engine) - already completed based on codebase exploration
- Depends on Plan 15 (assumed to be related to message/thread infrastructure)
- Uses existing `LLMClient` trait and `GenaiLLMClient` implementation
- Uses existing repository pattern for directions and LLM rules

## Implementation Plan

### Expected Behavior/Outcome

The LLM Decision Engine will:
1. Construct a 5-layer prompt from message context, directions, and LLM rules
2. Send the prompt to the configured LLM provider via existing `LLMClient`
3. Parse the JSON response into strongly-typed Rust structs
4. Validate all fields (confidence range, known action types, required fields)
5. Return a `DecisionOutput` that can be converted to a `NewDecision` for persistence

**States:**
- **Success**: Valid JSON response parsed into `DecisionOutput`
- **ParseError**: JSON malformed or missing required fields → fallback to safe decision with `needs_approval: true`
- **ValidationError**: Fields present but invalid (e.g., confidence > 1.0) → reject with specific error
- **LLMError**: Upstream provider error (rate limit, timeout, etc.) → propagate existing `LLMError`

### Key Findings

**Product & User Story:**
- When an email arrives and no deterministic rules match, the system invokes the LLM Decision Engine
- The engine constructs a context-rich prompt and receives a structured decision
- Decisions include action type, confidence, rationale, and undo hints for full auditability

**Design & UX Approach:**
- No direct UX changes - this is backend infrastructure
- Decisions are persisted for future UI display (audit log, approval queue)

**Technical Plan:**
1. Create `decision.rs` with LLM response types (`DecisionOutput`, `ActionType` enum, validation)
2. Create `prompt.rs` with `PromptBuilder` implementing the 5-layer structure
3. Add helper functions for body truncation and header filtering
4. Integrate with existing `LLMClient` for execution
5. Comprehensive unit tests for all parsing and construction scenarios

**Pragmatic Effort Estimate:** Medium complexity - well-scoped with clear requirements

### Acceptance Criteria

- [ ] `ActionType` enum defined with all 15 action types, serializes to snake_case strings
- [ ] `DecisionOutput` struct matches the JSON contract from docs/decision_engine.md
- [ ] `PromptBuilder::build()` produces `Vec<ChatMessage>` with 5 distinct layers
- [ ] Layer 1 (System) contains role definition and JSON schema requirement
- [ ] Layer 2 (Directions) formats all enabled directions as numbered list
- [ ] Layer 3 (LLM Rules) formats each rule with name, description, and rule_text
- [ ] Layer 4 (Message Context) includes from/to/cc/subject/snippet/labels/headers/body
- [ ] Layer 5 (Task) specifies JSON schema, valid actions, and constraints
- [ ] Body text truncated to configurable max length (default 8000 chars)
- [ ] HTML stripped from body if body_plain is missing
- [ ] Confidence validated in range [0.0, 1.0]
- [ ] Unknown action types result in validation error
- [ ] Malformed JSON responses handled gracefully with fallback
- [ ] All new code paths covered by unit tests

### Dependencies & Constraints

**Dependencies:**
- Existing `LLMClient` trait and `GenaiLLMClient` implementation
- `DirectionsRepository::list_enabled()` for loading directions
- `LlmRuleRepository` for loading LLM rules by scope
- `Message` struct with all email fields

**Technical Constraints:**
- Must use `f64` for confidence to match existing `Decision` schema
- Action types stored as strings in database, but validated via enum at parse time
- Thread context deferred - API accepts `Option<ThreadContext>` but always receives `None`

### Implementation Notes

**Recommended Approach:**
1. Start with `decision.rs` types since they define the contract
2. Build `prompt.rs` layer by layer with tests for each
3. Add JSON extraction helper that handles text before/after JSON in LLM responses
4. Use `serde(rename_all = "snake_case")` for action types to match JSON contract

**Potential Gotchas:**
- LLMs may include markdown formatting around JSON (```json blocks)
- Need regex or parser to extract JSON from response
- Empty directions/rules lists should produce valid prompts (just omit those sections)
- Very long subjects should be truncated to prevent prompt bloat

**File Organization:**
```
server/crates/ashford-core/src/llm/
├── mod.rs           # Add exports for new modules
├── decision.rs      # NEW: DecisionOutput, ActionType, parsing, validation
├── prompt.rs        # NEW: PromptBuilder, layer construction, body truncation
├── types.rs         # Existing: ChatMessage, ChatRole, etc.
├── error.rs         # Existing: May need new error variants
├── repository.rs    # Existing: LlmCallRepository
└── mock.rs          # Existing: MockChatExecutor
```

Implemented decision parsing stack for Phase 4.4 (Tasks 1-5). Added new file server/crates/ashford-core/src/llm/decision.rs defining ActionType enum with snake_case serde plus as_str/from_str helpers, and the DecisionOutput contract (MessageRef, DecisionDetails, Explanations, ConsideredAlternative, UndoHint, TelemetryPlaceholder). Implemented validation enforcing non-empty required fields and confidence ranges for primary decision and alternatives. Added DecisionParseError with JSON extraction helpers that handle code fences, surrounding text, and brace balancing while respecting quoted braces; DecisionOutput::parse now extracts, deserializes, and validates. Comprehensive unit tests cover enum round-trips, validation failures, JSON extraction edge cases, and parse error handling/success. Wired module exports in server/crates/ashford-core/src/llm/mod.rs so new types are available to the rest of the crate.

Implemented PromptBuilder and prompt construction pipeline covering Phase 4.4 tasks (Create PromptBuilder struct and configuration; Implement Layer 1 - System Message; Implement Layer 2 - Directions formatting; Implement Layer 3 - LLM Rules formatting; Implement body text processing utilities; Implement header filtering; Implement Layer 4 - Message Context formatting; Implement Layer 5 - Task Directive; Implement PromptBuilder::build() method; Add module exports and integrate with LLM module; Add integration test for full prompt-to-decision flow). Added new module file server/crates/ashford-core/src/llm/prompt.rs defining PromptBuilder with configurable max body/subject lengths plus ThreadContext placeholder. Built five-layer prompt assembly: system message content fixed, numbered DIRECTIONS section, LLM RULE blocks with optional descriptions, rich MESSAGE CONTEXT formatting (from/to/cc/bcc, truncated subject, snippet, whitelisted headers, labels JSON, truncated/sanitized body), and TASK directive describing JSON schema, valid actions (from ActionType enum), confidence bounds, approval hints, and undo expectations. Introduced utilities truncate_text (word-boundary ellipsis), strip_html (html2text width 80, whitespace normalized), get_body_text (plain-first fallback to HTML), and filter_relevant_headers (case-insensitive whitelist). Exported PromptBuilder, PromptBuilderConfig, ThreadContext via llm::mod.rs and added html2text dependency in server/crates/ashford-core/Cargo.toml (Cargo.lock updated). Added integration test tests/llm_prompt_decision_flow.rs to build a prompt with sample message/directions/rules and round-trip DecisionOutput parsing plus malformed response handling. Unit tests inside prompt.rs cover formatting, truncation, HTML stripping, header filtering, message context assembly, and prompt layer composition. Ran cargo fmt and cargo test -p ashford-core to validate all changes.
