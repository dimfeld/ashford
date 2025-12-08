---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: Thread Context Builder for LLM Prompts
goal: Build thread context summarization to enrich LLM decision prompts with
  conversation history
id: 20
uuid: da4dd6ae-fb36-49f3-b153-079eaf9524b0
generatedBy: agent
status: pending
priority: medium
container: false
temp: false
dependencies:
  - 16
parent: 4
references:
  "4": 5cf4cc37-3eb8-4f89-adae-421a751d13a1
  "16": b8c142c5-3335-4b87-9a94-28dbcc96af99
issue: []
pullRequest: []
docs:
  - docs/decision_engine.md
planGeneratedAt: 2025-12-02T07:44:35.561Z
promptsGeneratedAt: 2025-12-02T07:44:35.561Z
createdAt: 2025-12-02T07:44:10.852Z
updatedAt: 2025-12-08T04:27:17.922Z
progressNotes: []
tasks:
  - title: Define ThreadContext and supporting types
    done: false
    description: >-
      Create `server/crates/ashford-core/src/llm/thread_context.rs` with:

      - `ThreadContext` struct with message_count, participants, labels_history,
      prior_actions, conversation_summary, timestamps

      - `Participant` struct with email, name, role, message_count

      - `ParticipantRole` enum (Sender, Recipient, CC)

      - `PriorAction` struct with action_type, message_id, created_at

      - All types derive Serialize, Deserialize, Debug, Clone, PartialEq
  - title: Add method to load thread messages
    done: false
    description: >-
      In `MessageRepository`, add method:

      ```rust

      pub async fn list_by_thread(
          &self,
          org_id: i64,
          user_id: i64,
          thread_id: &str,
          exclude_message_id: Option<&str>,
          limit: Option<i64>,
      ) -> Result<Vec<Message>, MessageError>

      ```

      - Returns messages in thread ordered by received_at DESC (most recent
      first)

      - Optionally excludes the current message being classified

      - Limits to most recent N messages (default 50) for performance

      - Unit tests for thread message retrieval with and without limit
  - title: Add method to load thread actions
    done: false
    description: |-
      In `ActionRepository`, add method:
      ```rust
      pub async fn list_by_thread(
          &self,
          org_id: i64,
          user_id: i64,
          thread_id: &str,
      ) -> Result<Vec<Action>, ActionError>
      ```
      - Returns completed actions for messages in the thread
      - Ordered by created_at
      - Unit tests for action retrieval
  - title: Implement participant extraction
    done: false
    description: |-
      In `thread_context.rs`, add function:
      ```rust
      fn extract_participants(messages: &[Message]) -> Vec<Participant>
      ```
      - Aggregate all senders, recipients, CC across messages
      - Deduplicate by email (case-insensitive)
      - Track message count per participant
      - Determine role (prioritize Sender if they sent any message)
      - Unit tests with various thread configurations
  - title: Implement labels history extraction
    done: false
    description: |-
      In `thread_context.rs`, add function:
      ```rust
      fn extract_labels_history(messages: &[Message]) -> Vec<String>
      ```
      - Collect all unique labels that have appeared on any message in thread
      - Deduplicate and sort alphabetically
      - Unit tests
  - title: Implement heuristic conversation summary
    done: false
    description: >-
      In `thread_context.rs`, add function:

      ```rust

      fn generate_summary(messages: &[Message], participants: &[Participant]) ->
      Option<String>

      ```

      - Generate simple summary like: "Thread with 5 messages between Alice,
      Bob, and 2 others about 'Re: Project Update'"

      - Include message count, key participants, subject

      - Return None if thread has only 1 message

      - Unit tests for various thread sizes
  - title: Implement ThreadContextBuilder
    done: false
    description: >-
      In `thread_context.rs`, create:

      ```rust

      pub struct ThreadContextBuilder {
          message_repo: MessageRepository,
          action_repo: ActionRepository,
      }


      impl ThreadContextBuilder {
          pub fn new(message_repo: MessageRepository, action_repo: ActionRepository) -> Self;
          
          pub async fn build(
              &self,
              org_id: i64,
              user_id: i64,
              thread_id: &str,
              current_message_id: &str,
          ) -> Result<ThreadContext, ThreadContextError>;
      }

      ```

      - Load messages and actions

      - Extract participants, labels, prior actions

      - Generate summary

      - Return assembled ThreadContext
  - title: Add ThreadContextError type
    done: false
    description: |-
      In `thread_context.rs`, add:
      ```rust
      pub enum ThreadContextError {
          Message(MessageError),
          Action(ActionError),
          EmptyThread,
      }
      ```
      - Implement From traits for underlying errors
      - Implement std::error::Error
  - title: Update prompt.rs ThreadContext integration
    done: false
    description: >-
      In `prompt.rs`:

      - Replace placeholder `ThreadContext` struct with import from
      `thread_context.rs`

      - Update `build_message_context()` to format thread context when present:
        - Add "THREAD CONTEXT:" section
        - Include message count, participants summary, labels history
        - Include prior actions summary
        - Include conversation summary if available
      - Unit tests for prompt with and without thread context
  - title: Add module exports
    done: false
    description: >-
      Update `server/crates/ashford-core/src/llm/mod.rs`:

      - Add `mod thread_context;`

      - Export `ThreadContext`, `ThreadContextBuilder`, `ThreadContextError`,
      `Participant`, `ParticipantRole`, `PriorAction`
  - title: Add integration test for thread context flow
    done: false
    description: |-
      Create integration test:
      - Set up thread with multiple messages and actions
      - Build ThreadContext using ThreadContextBuilder
      - Build prompt using PromptBuilder with thread context
      - Verify prompt contains thread context section
      - Verify all expected data is present
  - title: Integrate ThreadContextBuilder into classification job
    done: false
    description: >-
      In `server/crates/ashford-core/src/jobs/classify.rs`:


      - Import `ThreadContextBuilder` from the llm module

      - In `run_llm_classification()`, before building the prompt:
        - Create `ThreadContextBuilder` with message and action repositories
        - Call `builder.build()` to get thread context (returns `Option<ThreadContext>`)
        - Pass thread context to `PromptBuilder::build()` instead of `None`
      - Handle the case where thread context building fails gracefully (log
      warning, proceed with `None`)

      - The repositories are already available via `Dispatcher`


      This completes the integration so thread context is actually used during
      classification.
tags: []
---

Implement the `ThreadContext` struct and builder to provide conversation history context for LLM classification prompts.

## Background

Plan 16 (Prompt Construction) deferred thread context to keep scope manageable. The `PromptBuilder` already accepts `Option<ThreadContext>` but always receives `None`. This plan implements the thread context building.

## Key Components

### ThreadContext Struct
```rust
pub struct ThreadContext {
    pub message_count: usize,
    pub participants: Vec<Participant>,
    pub labels_history: Vec<String>,      // Labels that have appeared on thread
    pub prior_actions: Vec<PriorAction>,  // Actions taken on previous messages
    pub conversation_summary: Option<String>, // Brief summary of thread
    pub first_message_at: Option<DateTime<Utc>>,
    pub latest_message_at: Option<DateTime<Utc>>,
}

pub struct Participant {
    pub email: String,
    pub name: Option<String>,
    pub role: ParticipantRole,  // Sender, Recipient, CC
    pub message_count: usize,
}

pub struct PriorAction {
    pub action_type: String,
    pub message_id: String,
    pub created_at: DateTime<Utc>,
}
```

### ThreadContextBuilder
```rust
pub struct ThreadContextBuilder {
    message_repo: MessageRepository,
    action_repo: ActionRepository,
}

impl ThreadContextBuilder {
    pub async fn build(
        &self,
        org_id: i64,
        user_id: i64,
        thread_id: &str,
        current_message_id: &str,  // Exclude current message
    ) -> Result<ThreadContext, ThreadContextError>;
}
```

### Conversation Summary Options
1. **Heuristic summary** - Extract key info: "3 messages between Alice and Bob about 'Project Update'"
2. **LLM-generated summary** - Use LLM to summarize thread (more expensive, better quality)
3. **No summary** - Just structured data, let classification LLM interpret

## Integration

The `ThreadContextBuilder` will be called before `PromptBuilder::build()` when processing messages that are part of existing threads.

## Scope

- Focus on structured data extraction first
- Conversation summary can be simple heuristic initially
- LLM-based summarization can be a future enhancement

## Research

### Summary

The ThreadContext feature is well-positioned for implementation. The codebase already has a placeholder `ThreadContext` struct in `prompt.rs`, the `PromptBuilder.build()` method accepts `Option<&ThreadContext>`, and the classification flow passes `None` for this parameter. All necessary source data (messages, actions, participants) exists in the database with proper repository access patterns. The main work involves:

1. Defining the full `ThreadContext` struct with participant, action history, and summary fields
2. Adding a `list_by_thread_id` method to `MessageRepository` (currently missing)
3. Adding a `list_by_thread_id` method to `ActionRepository` (currently only supports `list_by_message_id`)
4. Building the extraction logic for participants, labels, and heuristic summary
5. Integrating the `ThreadContextBuilder` into the classification flow

### Findings

#### Prompt System Architecture

**File:** `server/crates/ashford-core/src/llm/prompt.rs`

The prompt system uses a 6-layer architecture for LLM classification:
1. SYSTEM message (role, tool requirements, safety guidelines)
2. DIRECTIONS section (global guardrails)
3. LLM RULES section (scoped natural-language rules)
4. MESSAGE CONTEXT section (structured message data)
5. AVAILABLE LABELS section (labels for classification)
6. TASK directive (action instructions)

**Current ThreadContext Implementation (lines 9-11):**
```rust
/// Placeholder for future thread context summaries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThreadContext {}
```

**Integration Point (lines 151-153):**
```rust
if let Some(_ctx) = thread_context {
    // Reserved for future thread summaries.
}
```

The `build_message_context()` method already has a slot for thread context insertion, making integration straightforward.

**Classification Flow (lines 45-52):**
The `PromptBuilder::build()` method signature already accepts `Option<&ThreadContext>`, but it's always called with `None` in production.

#### Message Repository

**File:** `server/crates/ashford-core/src/messages.rs`

**Existing Methods:**
- `upsert(new_msg)` - Create/update message with conflict handling
- `get_by_provider_id(org_id, user_id, account_id, provider_message_id)` - Fetch by Gmail ID
- `get_by_id(org_id, user_id, message_id)` - Fetch by internal ID
- `exists(org_id, user_id, account_id, provider_message_id)` - Check existence

**Missing Method Needed:**
```rust
pub async fn list_by_thread_id(
    &self,
    org_id: i64,
    user_id: i64,
    thread_id: &str,
    exclude_message_id: Option<&str>,
) -> Result<Vec<Message>, MessageError>
```

**Message Structure (lines 21-45):**
Contains all participant information needed:
- `from_email`, `from_name` - Sender
- `to: Vec<Mailbox>` - Recipients
- `cc: Vec<Mailbox>` - CC recipients
- `bcc: Vec<Mailbox>` - BCC recipients
- `labels: Vec<String>` - Message labels
- `received_at` - Timestamp for ordering

**Database Index:** `messages_thread_idx` exists for efficient thread queries.

#### Action Repository

**File:** `server/crates/ashford-core/src/decisions/repositories.rs`

**Existing Methods (lines 327-351):**
- `list_by_message_id(org_id, user_id, message_id)` - Actions for single message
- `list_by_status(org_id, user_id, status, account_id)` - Actions by status
- `list_filtered(...)` - Complex filtering with joins

**Missing Method Needed:**
```rust
pub async fn list_by_thread_id(
    &self,
    org_id: i64,
    user_id: i64,
    thread_id: &str,
) -> Result<Vec<Action>, ActionError>
```

This requires joining with the messages table since actions link to `message_id`, not `thread_id`.

**Action Structure (lines in types.rs):**
```rust
Action {
    id, message_id, action_type, status,
    created_at, executed_at, ...
}
```

**ActionStatus Enum:**
- Queued, Executing, Completed, Failed, Canceled, Rejected, ApprovedPending

Only actions with status `Completed` should be included in prior actions context.

#### Thread Repository

**File:** `server/crates/ashford-core/src/threads.rs`

**Thread Structure:**
```rust
Thread {
    id, account_id, provider_thread_id,
    subject, snippet, last_message_at,
    metadata_json, raw_json,
    created_at, updated_at, org_id, user_id
}
```

**Existing Methods:**
- `upsert(...)` - Create/update thread
- `get_by_id(org_id, user_id, thread_id)` - Fetch by ID
- `get_by_provider_id(org_id, user_id, account_id, provider_thread_id)` - Fetch by Gmail thread ID
- `update_last_message_at(...)` - Update timestamp

#### Classification Job

**File:** `server/crates/ashford-core/src/jobs/classify.rs`

**Current LLM Classification Flow (lines 434-510):**
1. Load message from database
2. Build prompt with `PromptBuilder`
3. Call `PromptBuilder::build()` with `thread_context: None`
4. Send to LLM client

**Integration Point (line 467):**
```rust
let messages = prompt_builder.build(
    message,
    &directions,
    &llm_rules,
    None,  // <-- ThreadContext always None
    &available_labels
);
```

**LlmCallContext (lines 482-491):**
Already contains `thread_id` field - the thread ID is available during classification.

#### Error Handling Patterns

**File:** Various repository files

All error types follow a consistent pattern using `thiserror`:
```rust
#[derive(Debug, Error)]
pub enum ComponentError {
    #[error("database error: {0}")]
    Database(#[from] DbError),
    #[error("sql error: {0}")]
    Sql(#[from] libsql::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("datetime parse error: {0}")]
    DateTimeParse(#[from] chrono::ParseError),
    #[error("not found: {0}")]
    NotFound(String),
}
```

**ThreadContextError should follow this pattern:**
```rust
#[derive(Debug, Error)]
pub enum ThreadContextError {
    #[error("message error: {0}")]
    Message(#[from] MessageError),
    #[error("action error: {0}")]
    Action(#[from] ActionError),
    #[error("empty thread")]
    EmptyThread,
}
```

#### Database Schema

**File:** `server/migrations/001_initial.sql`

**threads table:**
- Primary key: `id TEXT`
- Unique constraint: `(account_id, provider_thread_id)`
- Indexes: `threads_account_thread_idx`, `threads_last_message_idx`

**messages table:**
- Primary key: `id TEXT`
- Foreign key: `thread_id` → `threads(id)`
- Index: `messages_thread_idx` on `(thread_id)`

**actions table:**
- Primary key: `id TEXT`
- Foreign key: `message_id` → `messages(id)`
- Index: `actions_message_idx` on `(message_id, created_at)`

The `messages_thread_idx` index enables efficient queries for `list_by_thread_id`.

#### Decision Engine Documentation

**File:** `docs/decision_engine.md`

Documents that MESSAGE CONTEXT should include:
- Thread summary (previous labels, actions, participants)
- This aligns with the ThreadContext design

The 6-layer prompt architecture explicitly reserves space for thread context in Layer 4 (MESSAGE CONTEXT).

### Risks & Constraints

1. **Query Performance**: Loading all messages in a thread could be slow for very long threads. Consider adding a limit (e.g., last 50 messages) or using summary statistics from the thread table.

2. **Participant Deduplication**: Email addresses may appear with different name variations. Need case-insensitive email matching for deduplication.

3. **Action Joining**: The `list_by_thread_id` for actions requires a JOIN with messages table since actions only have `message_id`, not `thread_id` directly.

4. **Empty Thread Handling**: The current message being classified might be the first in a thread. Return `None` or empty context gracefully.

5. **Thread Context Placement in Prompt**: The current placeholder is at the end of `build_message_context()`. Need to decide the best position and formatting for thread context data.

6. **Circular Dependencies**: `ThreadContextBuilder` will depend on both `MessageRepository` and `ActionRepository`. These are already available in the classification job context.

7. **Testing**: Need integration tests that set up realistic thread data with multiple messages, participants, and actions.

8. **Prompt Token Budget**: Thread context adds to prompt length. Consider summarizing rather than including all details for very active threads.

## Expected Behavior/Outcome

When classifying an email that is part of an existing thread with prior messages:
- The LLM prompt includes a "THREAD CONTEXT" section with conversation history
- Participants are extracted and deduplicated across all thread messages
- Prior completed actions on thread messages are summarized
- Labels that have appeared on any thread message are listed
- A heuristic summary describes the thread (e.g., "5 messages between Alice, Bob, and 2 others about 'Project Update'")
- First and last message timestamps are included

For single-message threads or the first message in a thread:
- Thread context is minimal or omitted (returns `None`)
- Classification proceeds normally without thread history

## Key Findings

**Product & User Story:**
Thread context enables smarter classification by giving the LLM visibility into conversation history. For example:
- If previous messages were archived, the LLM can follow the same pattern
- If a thread has high-priority labels, new messages inherit that context
- Multi-party threads are handled with awareness of all participants

**Design & UX Approach:**
This is a backend-only feature. The thread context enriches LLM prompts without any UI changes. The feature is transparent to users - they simply get better classification decisions.

**Technical Plan & Risks:**
- Core risk is query performance for long threads - mitigated by limiting to recent messages
- Participant deduplication requires case-insensitive email matching
- Action retrieval needs a JOIN since actions link to messages, not threads directly

**Pragmatic Effort Estimate:**
The existing task breakdown is well-structured. Key implementation areas:
1. Type definitions (ThreadContext, Participant, PriorAction) - straightforward
2. Repository methods - two new queries needed
3. Extraction logic - moderate complexity for participant deduplication
4. Integration - minimal changes to classify.rs and prompt.rs

## Acceptance Criteria

- [ ] `ThreadContext` struct defined with all specified fields (message_count, participants, labels_history, prior_actions, conversation_summary, timestamps)
- [ ] `MessageRepository::list_by_thread_id()` returns all messages in a thread ordered by `received_at`, with optional exclusion of current message
- [ ] `ActionRepository::list_by_thread_id()` returns completed actions for all messages in a thread
- [ ] Participant extraction correctly deduplicates by email (case-insensitive) and tracks role (Sender/Recipient/CC)
- [ ] Labels history collects unique labels from all thread messages
- [ ] Heuristic summary generates human-readable thread description
- [ ] `ThreadContextBuilder::build()` assembles complete `ThreadContext` from repositories
- [ ] `PromptBuilder::build_message_context()` formats thread context into prompt when present
- [ ] Classification job calls `ThreadContextBuilder` for messages with thread history
- [ ] Unit tests cover participant extraction, labels extraction, and summary generation
- [ ] Integration test verifies complete flow from thread setup through prompt generation
- [ ] All new code paths are covered by tests

## Dependencies & Constraints

**Dependencies:**
- Plan 16 (Prompt Construction) - already complete, provides `PromptBuilder` infrastructure
- Existing `MessageRepository` and `ActionRepository` for data access
- `threads` and `messages` database tables with `thread_id` relationship

**Technical Constraints:**
- Limit thread context to most recent 50 messages for performance and prompt size
- Participant emails must be deduplicated case-insensitively
- Actions query must JOIN with messages table to filter by thread_id
- Thread context should not significantly increase prompt token count

## Implementation Notes

**Recommended Approach:**
1. Create `thread_context.rs` as a new module under `src/llm/`
2. Define types first (`ThreadContext`, `Participant`, `ParticipantRole`, `PriorAction`, `ThreadContextError`)
3. Add repository methods to existing files (not new files)
4. Implement extraction functions as private functions in `thread_context.rs`
5. Build `ThreadContextBuilder` that composes repository calls and extraction
6. Update `prompt.rs` to format `ThreadContext` in the "THREAD CONTEXT" section
7. Wire up in `classify.rs` - build thread context before prompt building

**Potential Gotchas:**
- The `Message` struct stores participants in `to`, `cc`, `bcc` as `Vec<Mailbox>` but `from_email`/`from_name` as separate fields. Handle both formats in participant extraction.
- Empty threads (message being classified is the only message) should return `None` rather than error.
- The `exclude_message_id` parameter is important - we don't want the current message included in its own thread context.
- Action status filtering: only include `Completed` actions in prior_actions, not queued/failed/etc.

**Thread Context Formatting for Prompt:**
Suggested format to add after the existing message context:
```
THREAD CONTEXT:
Messages in thread: 5
Participants: Alice <alice@example.com> (Sender, 3 messages), Bob <bob@example.com> (Recipient, 2 messages)
Labels seen: INBOX, IMPORTANT, Project/Alpha
Prior actions: archive (2), apply_label (1)
Summary: Thread with 5 messages between Alice and Bob about "Re: Project Update"
First message: 2024-01-15T10:30:00Z
Latest message: 2024-01-17T14:45:00Z
```
