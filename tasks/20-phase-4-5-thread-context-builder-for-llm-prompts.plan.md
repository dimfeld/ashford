---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Thread Context Builder for LLM Prompts"
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
references: {}
issue: []
pullRequest: []
docs:
  - docs/decision_engine.md
planGeneratedAt: 2025-12-02T07:44:35.561Z
promptsGeneratedAt: 2025-12-02T07:44:35.561Z
createdAt: 2025-12-02T07:44:10.852Z
updatedAt: 2025-12-02T07:44:35.561Z
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
    description: |-
      In `MessageRepository`, add method:
      ```rust
      pub async fn list_by_thread(
          &self,
          org_id: i64,
          user_id: i64,
          thread_id: &str,
          exclude_message_id: Option<&str>,
      ) -> Result<Vec<Message>, MessageError>
      ```
      - Returns all messages in thread ordered by received_at
      - Optionally excludes the current message being classified
      - Unit tests for thread message retrieval
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
