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
Your task is to produce a single JSON decision object following the required schema.
You MUST follow the DIRECTIONS section strictly.
You MUST NOT hallucinate.
If uncertain, choose a safe and reversible action.

This message also enforces:
	•	Required output JSON schema.
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

The final section specifies what the model must output:
	•	The JSON structure (the decision contract)
	•	Constraints (e.g., “confidence must be [0.0, 1.0]”, “use None when no action”, etc.)
	•	Safety notes
	•	Approvals logic (advisory)

Example:

TASK:
Based on the DIRECTIONS and LLM RULES, evaluate the email and output a single JSON object with:
- decision.action
- decision.parameters
- decision.confidence
- decision.needs_approval
- rationale
- explanations
- undo_hint
- telemetry placeholder


⸻

10.4 JSON Decision Contract

The decision contract remains the same as defined earlier in the spec, represented in strict Rust serde structs.

(Retained, not repeated here.)

⸻

10.5 Post-Processing & Enforcement (Rust-Side)

After receiving the model output:
	1.	Validate JSON strictly (schema validation + semantic validation).
	2.	Apply Directions on the Rust side as a safety override:
	•	If model suggests a forbidden action → downgrade to safe fallback (e.g., mark_unread) or mark as requiring approval.
	3.	Apply “Dangerous Action Policy”:
	•	If action is destructive and not whitelisted by deterministic rules → require Discord approval.
	4.	Calculate effective confidence thresholds:
	•	If model confidence < global threshold → convert to “needs approval”.
	5.	Persist decisions record.
	6.	Enqueue next job:
	•	Auto-run safe actions, or
	•	Create an approval request for Discord.

This guarantees deterministic, auditable behavior even when the LLM output is imperfect.

⸻

10.6 Telemetry

Each classifier run records:
	•	Trace ID (OpenTelemetry)
	•	Model latency
	•	Input/output token counts
	•	Prompt size
	•	Decision result
	•	Safety overrides applied

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

