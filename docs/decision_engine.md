The LLM Decision Engine is responsible for evaluating emails that were not handled by deterministic rules. It consumes message context, applies global guardrails (directions), evaluates applicable LLM rules, and produces a fully structured decision object.

The engine is implemented in Rust and uses the genai crate (or equivalent abstraction) to support multiple model providers.

⸻

10.1 Responsibilities

The decision engine must:
	1.	Provide deterministic behavior under uncertainty by always applying global “directions” as first-class constraints.
	2.	Interpret enabled LLM rules relevant to the account/domain/scope.
	3.	Generate a structured, machine-verifiable JSON decision containing:
	•	Action type
	•	Action parameters
	•	Confidence score
	•	Approval requirement (advisory)
	•	Rationale
	•	Undo hints
	•	Telemetry
	4.	Behave predictably and safely, applying directions even when model output is ambiguous or inconsistent.
	5.	Attach OpenTelemetry traces to support full pipeline observability.

The LLM Decision Engine is only invoked after:
	•	Deterministic rules were evaluated and none matched, or
	•	A deterministic rule explicitly delegates evaluation to the model (rare, but supported).

⸻

10.2 Inputs to the Decision Engine

The engine receives:
	•	account_id
	•	message_id
	•	Parsed message metadata:
	•	Sender, recipients, subject, snippet
	•	Headers of interest (List-Id, Return-Path, etc.)
	•	Labels
	•	Body plain text + HTML (sanitized)
	•	Thread summary context
	•	Enabled Directions (global guardrails)
	•	Applicable LLM Rules (based on scope)
	•	System-level configuration:
	•	model name
	•	temperature
	•	token limits
	•	safety policy flags (e.g., require high-confidence for certain actions)

⸻

10.3 Prompt Construction

The prompt is constructed from five clearly separated layers.
This structure is critical for auditability, debugging, and predictable behavior.

⸻

Layer 1 — SYSTEM Message

Defines the agent role and output contract. Example:

You are the email classification and action engine.
You MUST call the `record_decision` tool to provide your classification decision.
You MUST follow the DIRECTIONS section strictly.
You MUST NOT hallucinate.
If uncertain, choose a safe and reversible action.

This message also enforces:
	•	Tool call requirement (structured output via tool calling).
	•	Reversibility considerations.
	•	The rule that classification must be grounded in message content and configured rules.

⸻

Layer 2 — DIRECTIONS (Global Guardrails)

All enabled = 1 entries from the directions table are concatenated into a numbered list.

This layer acts as a global, always-on policy constraint, independent of LLM rules.
Directions are not conditions or actions; they define the global behavior philosophy of the system.

Example inserted content:

DIRECTIONS:
1. Never delete or permanently remove email unless a deterministic rule explicitly permits it.
2. When uncertain, prefer labeling or archiving over destructive actions.
3. Never forward or reply to an email unless the rule or user instruction explicitly directs this.
4. Prefer safe, reversible operations when the intent is unclear.

The engine treats directions as hard constraints. If the model attempts an action that violates directions, the Rust side will reject or adjust the decision.

⸻

Layer 3 — LLM Rules (Scoped)

Only LLM rules relevant to the account and message scope are included.

Each rule is presented as:

LLM RULE: <name>
<description>
<rule_text>

These guide model behavior for thematic categories (e.g., invoices, HR email, receipts, school notices).

Key distinction:
LLM rules express situational expectations.
Directions express global invariants.

If an LLM rule conflicts with a direction, the direction wins.

⸻

Layer 4 — Message Context

This contains all structured data about the message:
	•	From / To / CC
	•	Subject
	•	Snippet
	•	Relevant headers
	•	Message body (plain text, compacted)
	•	Current Gmail labels
	•	Thread summary (previous labels, actions, participants)

The engine ensures message content is safe (e.g., stripped of extremely long HTML, rewritten for the model) before insertion.

Example snippet inserted:

MESSAGE CONTEXT:
From: "Amazon" <shipment-tracking@amazon.com>
Subject: Your order has shipped
Snippet: "Your order #123-4567890 has shipped..."
Labels: ["INBOX", "CATEGORY_UPDATES"]
Body (plain): ...


⸻

Layer 5 — TASK Directive

The final section specifies what the model must do:
	•	Call the `record_decision` tool with the decision
	•	Lists valid action types
	•	Constraints (e.g., "confidence must be [0.0, 1.0]", "use None when no action", etc.)
	•	Safety notes
	•	Approvals logic (advisory)

Example:

TASK:
Analyze this email and call the `record_decision` tool with your classification decision.

Valid action types: apply_label, mark_read, mark_unread, archive, delete, ...

Requirements:
- Confidence MUST be between 0.0 and 1.0 inclusive.
- If the action is destructive and confidence is low, set needs_approval to true.
- Ensure undo_hint.inverse_action can reverse the primary decision.
- You MUST call the record_decision tool - do not return plain text.

Note: The JSON schema for the decision is provided via the tool definition, not inline in the prompt.
This uses structured generation via tool calling for more reliable output.


⸻

10.4 JSON Decision Contract

The decision contract remains the same as defined earlier in the spec, represented in strict Rust serde structs.

(Retained, not repeated here.)

⸻

10.5 Post-Processing & Enforcement (Rust-Side)

After receiving the model output:
	1.	Validate JSON strictly (schema validation + semantic validation).
	2.	Apply Safety Enforcement via `SafetyEnforcer`:
	•	Check danger level, confidence, approval_always list, and LLM advisory flag.
	•	Dangerous actions always require approval.
	3.	Persist decisions record with safety telemetry.
	4.	Enqueue next job:
	•	Auto-run safe actions, or
	•	Create an approval request for Discord.

This guarantees deterministic, auditable behavior even when the LLM output is imperfect.

#### SafetyEnforcer

The `SafetyEnforcer` struct validates LLM decisions against policy constraints. It is implemented in `server/crates/ashford-core/src/decisions/safety.rs`.

```rust
use ashford_core::{SafetyEnforcer, PolicyConfig, DecisionOutput};

let enforcer = SafetyEnforcer::new(policy_config);
let result = enforcer.enforce(&decision_output);

if result.requires_approval {
    // Route to Discord approval flow
    for override_reason in &result.overrides_applied {
        println!("Approval required: {}", override_reason);
    }
}
```

#### Action Danger Levels

All action types are classified by danger level (`ActionDangerLevel` enum):

| Level | Actions | Behavior |
|-------|---------|----------|
| **Safe** | ApplyLabel, MarkRead, MarkUnread, Archive, Move, None | Auto-execute |
| **Reversible** | Star, Unstar, Snooze, AddNote, CreateTask | Auto-execute with undo hint |
| **Dangerous** | Delete, Forward, AutoReply, Escalate | Always requires approval |

Access via `ActionType::danger_level()`:

```rust
use ashford_core::ActionType;

let action = ActionType::Delete;
assert_eq!(action.danger_level(), ActionDangerLevel::Dangerous);
```

#### Safety Override Reasons

When the enforcer requires approval, it captures all applicable reasons as `SafetyOverride` variants:

- **DangerousAction**: Action is classified as `ActionDangerLevel::Dangerous`
- **LowConfidence { confidence, threshold }**: Decision confidence is below the configured threshold
- **InApprovalAlwaysList**: Action type is in the `approval_always` config list
- **LlmRequestedApproval**: The LLM's advisory `needs_approval` flag was true

Multiple overrides can apply simultaneously. The logic uses OR semantics—if any condition triggers, approval is required.

#### Enforcement Logic

The enforcer applies these checks in order, collecting all applicable overrides:

1. **Danger Level Check**: If `action.danger_level() == Dangerous` → add `DangerousAction` override
2. **Confidence Threshold**: If `confidence < policy.confidence_default` → add `LowConfidence` override
3. **approval_always List**: If action type string (snake_case) is in `policy.approval_always` → add `InApprovalAlwaysList` override
4. **LLM Advisory Flag**: If `decision.needs_approval == true` → add `LlmRequestedApproval` override

The LLM's advisory flag is always honored—if the LLM requests approval, we respect it even if policy would allow auto-execution.

#### Telemetry Integration

Safety overrides are recorded in decision telemetry for audit purposes:

```rust
let telemetry_json = result.to_telemetry_json();
// {
//   "safety_overrides": ["DangerousAction", "LowConfidence (0.45 < 0.70)"],
//   "requires_approval": true
// }
```

This is stored in the `telemetry_json` field of the decisions table.

#### Configuration

Safety policy is configured via `PolicyConfig` in `config.toml`:

```toml
[policy]
approval_always = ["delete", "forward", "auto_reply", "escalate"]
confidence_default = 0.7
```

- **approval_always**: Action type strings (snake_case) that always require approval regardless of danger level or confidence
- **confidence_default**: Threshold below which approval is required (0.0 to 1.0)

Note: Direction violation detection is deferred to a future plan (LLM-Based Direction Violation Detection).

⸻

10.6 Telemetry

Each classifier run records:
	•	Trace ID (OpenTelemetry)
	•	Model latency
	•	Input/output token counts
	•	Prompt size
	•	Decision result
	•	Safety overrides applied (from `SafetyResult::to_telemetry_json()`)

Safety telemetry includes:
- List of all `SafetyOverride` reasons that triggered approval requirements
- Human-readable descriptions for audit trail (e.g., "Action is dangerous", "Confidence 0.45 below threshold 0.70")
- Final `requires_approval` determination

This allows deep debugging and post-hoc safety audits.

⸻

10.7 Summary of the Role of Directions
	•	Directions are global guardrails.
	•	They are inserted before LLM rules.
	•	They always take precedence.
	•	The model is instructed to obey directions explicitly.
	•	Rust enforces them again after model output.
	•	Directions reduce hallucination, increase safety, and make LLM behavior predictable.


  

### **10.1 Provider & Library**

- Use a Rust abstraction such as genai to integrate with multiple providers.
- Configurable model via config file:

    - provider (e.g., openai, google, etc.).

    - model_name.

    - temperature.

    - max_output_tokens.

  

### **10.2 Decision JSON Contract**

  

    {
      "message_ref": {
        "provider": "gmail",
        "account_id": "string",
        "thread_id": "string|null",
        "message_id": "string"
      },
      "decision": {
        "action": "apply_label|mark_read|mark_unread|archive|delete|move|star|unstar|forward|auto_reply|create_task|snooze|add_note|escalate|none",
        "parameters": {},
        "confidence": 0.0,
        "needs_approval": true,
        "rationale": "string"
      },
      "explanations": {
        "salient_features": ["string"],
        "matched_directions": ["string"],
        "considered_alternatives": [
          {
            "action": "archive",
            "confidence": 0.42,
            "why_not": "string"
          }
        ]
      },
      "undo_hint": {
        "inverse_action": "unapply_label|mark_unread|mark_read|move|restore|unstar|delete_reply|reopen_task|unsnooze|remove_note|deescalate|none",
        "inverse_parameters": {}
      },
      "telemetry": {
        "model": "provider:model@version",
        "latency_ms": 0,
        "input_tokens": 0,
        "output_tokens": 0
      }
    }

Rust side:

- Define strict structs and deserialize with serde_json.
- Validate fields; treat needs_approval as advisory.

### **10.3 Rust Implementation**

The LLM Decision Engine implementation is split into two modules:
- `server/crates/ashford-core/src/llm/decision.rs` - Decision types and parsing
- `server/crates/ashford-core/src/llm/prompt.rs` - 5-layer prompt construction

#### Prompt Builder

The `PromptBuilder` constructs the 5-layer prompt from message context, directions, and LLM rules:

```rust
use ashford_core::llm::{PromptBuilder, PromptBuilderConfig, ThreadContext};

// Create with default configuration
let builder = PromptBuilder::new();

// Or customize limits
let builder = PromptBuilder::with_config(PromptBuilderConfig {
    max_body_length: Some(8000),    // Default: 8000 chars
    max_subject_length: Some(500),  // Default: 500 chars
});

// Build the prompt messages
let messages = builder.build(
    &message,       // &Message - the email to classify
    &directions,    // &[Direction] - enabled global guardrails
    &llm_rules,     // &[LlmRule] - applicable LLM rules
    None,           // Option<&ThreadContext> - reserved for future thread summaries
);
```

The `build()` method returns a `Vec<ChatMessage>` with exactly 2 messages:
1. **System message** (ChatRole::System) - role definition, output contract, safety guidelines
2. **User message** (ChatRole::User) - combined DIRECTIONS, LLM RULES, MESSAGE CONTEXT, and TASK sections

##### Body Text Processing

The prompt builder includes utilities for safely processing email content:

- **`truncate_text(text, max_len)`** - Truncates at word boundaries with "..." suffix
- **`strip_html(html)`** - Uses `html2text` crate for robust HTML-to-text conversion, handling entities, scripts, and tables
- **`get_body_text(message, max_len)`** - Prefers body_plain, falls back to stripped body_html
- **`filter_relevant_headers(headers)`** - Whitelists: List-Id, Return-Path, X-Priority, X-Mailer, Reply-To, Precedence

##### Empty Sections

When directions or LLM rules are empty, those sections are omitted entirely from the prompt (not included as empty sections).

##### Message Context Format

The MESSAGE CONTEXT section includes:
- From/To/CC/BCC with name and email formatting
- Subject (truncated to max_subject_length)
- Snippet
- Filtered headers
- Labels as JSON array
- Body text (truncated, HTML stripped if needed)

##### Thread Context

`ThreadContext` is a placeholder struct for future thread summaries. Currently always pass `None` for this parameter.

#### ActionType Enum

All supported action types are defined as a Rust enum with snake_case serialization:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
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

The enum provides `as_str()` for string conversion and implements `FromStr` for parsing. The `JsonSchema` derive enables automatic JSON Schema generation for tool calling.

#### Decision Structs

The complete decision contract is represented by these Rust structs. All types derive `JsonSchema` from the `schemars` crate for automatic JSON Schema generation:

```rust
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct DecisionOutput {
    pub message_ref: MessageRef,
    pub decision: DecisionDetails,
    pub explanations: Explanations,
    pub undo_hint: UndoHint,
    pub telemetry: TelemetryPlaceholder,
}

pub struct MessageRef {
    pub provider: String,
    pub account_id: String,
    pub thread_id: String,
    pub message_id: String,
}

pub struct DecisionDetails {
    pub action: ActionType,
    pub parameters: Value,  // serde_json::Value for flexibility
    pub confidence: f64,
    pub needs_approval: bool,
    pub rationale: String,
}

pub struct Explanations {
    pub salient_features: Vec<String>,
    pub matched_directions: Vec<String>,
    pub considered_alternatives: Vec<ConsideredAlternative>,
}

pub struct ConsideredAlternative {
    pub action: ActionType,
    pub confidence: f64,
    pub why_not: String,
}

pub struct UndoHint {
    pub inverse_action: ActionType,
    pub inverse_parameters: Value,
}

pub struct TelemetryPlaceholder {}  // Empty; telemetry populated by Rust side
```

#### Validation

`DecisionOutput::validate()` enforces these constraints:

- **Required string fields** must be non-empty: `message_ref.provider`, `message_ref.account_id`, `message_ref.thread_id`, `message_ref.message_id`, `decision.rationale`
- **Confidence** must be in range `[0.0, 1.0]` for both the primary decision and all considered alternatives
- **Considered alternatives** must have non-empty `why_not` explanations

Validation errors are represented by `DecisionValidationError`:

```rust
pub enum DecisionValidationError {
    EmptyField(&'static str),
    InvalidConfidence(f64),
    InvalidAlternativeConfidence { index: usize, confidence: f64 },
}
```

#### JSON Extraction

LLM responses may contain extra text or markdown formatting around the JSON. The `extract_json_from_response()` function handles:

1. **Code fences**: Extracts content from ` ```json ... ``` ` or ` ``` ... ``` ` blocks
2. **Surrounding text**: Finds the first `{` and matches balanced braces
3. **Escaped braces**: Correctly handles `{` and `}` inside JSON strings

Example responses that are handled:

```
Sure, here is the decision:
```json
{"message_ref": {...}, "decision": {...}}
```

The decision object is valid.
```

#### Tool Definition

The `build_decision_tool()` function creates a tool definition with the JSON Schema derived from the Rust types:

```rust
use ashford_core::llm::{build_decision_tool, DECISION_TOOL_NAME};

// Build the tool with auto-generated JSON schema
let tool = build_decision_tool();

// The tool name is "record_decision"
assert_eq!(tool.name, DECISION_TOOL_NAME);

// Add the tool to your completion request
let request = CompletionRequest {
    messages,
    temperature: 0.1,
    max_tokens: 4096,
    json_mode: false,  // Not needed with tool calling
    tools: vec![tool],
};
```

#### Parsing

There are two methods for parsing LLM responses:

**Preferred: Tool Call Parsing**

`DecisionOutput::parse_from_tool_calls()` extracts the decision from tool call results:

```rust
use ashford_core::llm::{DecisionOutput, DECISION_TOOL_NAME};

// After getting the completion response
let tool_calls = response.tool_calls;

match DecisionOutput::parse_from_tool_calls(&tool_calls, DECISION_TOOL_NAME) {
    Ok(decision) => {
        // Use decision.decision.action, decision.decision.confidence, etc.
    }
    Err(DecisionParseError::NoToolCall) => {
        // LLM didn't call the tool - may need to retry or use fallback
    }
    Err(DecisionParseError::WrongToolName { expected, actual }) => {
        // LLM called a different tool
    }
    Err(DecisionParseError::Validation(e)) => {
        // Handle validation failure - may want to require approval
    }
    Err(e) => {
        // Handle other parse errors
    }
}
```

**Legacy: Text Response Parsing**

`DecisionOutput::parse(response: &str)` extracts JSON from text responses (useful for testing or fallback):

1. Extract JSON slice from the response (handles code fences and surrounding text)
2. Deserialize into `DecisionOutput` using serde_json
3. Run validation
4. Return the validated decision or an error

Parse errors are represented by `DecisionParseError`:

```rust
pub enum DecisionParseError {
    NoJsonFound,           // No JSON object in response
    MalformedJson,         // Unbalanced braces
    Json(serde_json::Error), // Deserialization failed
    Validation(DecisionValidationError), // Validation failed
    NoToolCall,            // No tool call in response
    WrongToolName { expected: String, actual: String }, // Wrong tool called
}
```

#### Usage Example

```rust
use ashford_core::llm::{
    build_decision_tool, DecisionOutput, DecisionParseError,
    CompletionRequest, DECISION_TOOL_NAME,
};

// Build the request with the decision tool
let tool = build_decision_tool();
let request = CompletionRequest {
    messages: prompt_builder.build(&message, &directions, &rules, None),
    temperature: 0.1,
    max_tokens: 4096,
    json_mode: false,
    tools: vec![tool],
};

// Call the LLM
let response = llm_client.complete(request, context).await?;

// Parse the tool call response
match DecisionOutput::parse_from_tool_calls(&response.tool_calls, DECISION_TOOL_NAME) {
    Ok(decision) => {
        println!("Action: {:?}, Confidence: {}", decision.decision.action, decision.decision.confidence);
    }
    Err(e) => {
        eprintln!("Failed to parse decision: {}", e);
    }
}
```

