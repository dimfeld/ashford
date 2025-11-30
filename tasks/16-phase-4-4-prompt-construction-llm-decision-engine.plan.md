---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Phase 4.4: Prompt Construction & LLM Decision Engine"
goal: Build the 5-layer prompt construction system and LLM decision
  parsing/validation
id: 16
uuid: b8c142c5-3335-4b87-9a94-28dbcc96af99
status: pending
priority: high
container: false
temp: false
dependencies:
  - 13
  - 15
parent: 4
issue: []
docs:
  - docs/decision_engine.md
  - docs/rules_engine.md
createdAt: 2025-11-30T01:14:19.216Z
updatedAt: 2025-11-30T01:14:19.216Z
tasks: []
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
