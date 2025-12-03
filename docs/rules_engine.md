The Rule System defines how the AI Mail Agent determines actions for incoming emails.
It consists of three coordinated layers:
	1.	Deterministic Rules — explicit, structured filters and actions.
	2.	Directions — global natural-language behavioral guardrails.
	3.	LLM Rules — situational natural-language classification logic used when deterministic rules do not match.

Together, these ensure predictable and safe behavior while giving users flexible natural-language control.

⸻

3.1 Overview

When a new email is ingested:
	1.	The system evaluates Deterministic Rules first and with highest precedence.
	2.	If none match, the system invokes the LLM Decision Engine.
	3.	During LLM evaluation:
	•	Directions are always applied as global constraints.
	•	LLM Rules provide category-level behavior.
	4.	The resulting decision then flows through:
	•	Safety enforcement
	•	“Dangerous Action” policy
	•	Discord approval
	•	Action execution

This hybrid architecture guarantees that where users want explicit behavior, they get it, and where they want flexibility, they get intelligent automation under strong safety constraints.

⸻

3.2 Deterministic Rules

Deterministic rules are structured filters with explicit matching conditions and explicit actions.
They are evaluated before any LLM involvement.

3.2.1 Rule Structure

Each deterministic rule includes:
	•	id
	•	name
	•	description
	•	scope (global | account | sender | domain)
	•	scope_ref (account_id, domain name, or sender email)
	•	priority (lower = earlier evaluation)
	•	enabled flag
	•	disabled_reason — explains why a rule was auto-disabled (e.g., "Label 'Work' was deleted from Gmail")
	•	conditions_json — structured boolean condition tree:
	•	Sender email exact / wildcard match
	•	Sender domain
	•	Subject regex or substring
	•	Header regex
	•	Gmail label presence
	•	action_type (archive | apply_label | delete | snooze | forward | …)
	•	action_parameters_json
	•	safe_mode:
	•	default
	•	always_safe
	•	dangerous_override (explicitly allow dangerous actions)

3.2.2 Execution Semantics
	•	Deterministic rules are evaluated in ascending priority.
	•	You can choose (configurable) to:
	•	Apply first match, or
	•	Apply all matches in priority order (actions merged).
	•	Actions generated from deterministic rules:
	•	Are considered strong, explicit intent.
	•	May bypass LLM entirely.
	•	Still pass through safety gating:
	•	If safe_mode='dangerous_override' → these actions are considered safe.
	•	Otherwise → dangerous actions require Discord approval.

Deterministic rules give the user explicit, stable behavior—ideal for high-volume or predictable senders.

3.2.3 Auto-Disabled Rules

Rules can be automatically disabled by the system when their dependencies become invalid:

- **Deleted Labels**: When a Gmail label referenced by a rule is deleted, the rule is soft-disabled with `disabled_reason` set (e.g., "Label 'Work' was deleted from Gmail")
- **Preservation**: Disabled rules are not deleted, allowing users to review and fix them
- **Re-enabling**: Users can update the rule to reference a valid label and re-enable it

The `disabled_reason` field provides clear feedback about why a rule stopped working.

⸻

3.3 Directions (Global Guardrails)

Directions are global, always-on natural-language instructions that shape the LLM’s behavioral boundaries.
They are not rules, are not conditional, and do not produce actions directly.

3.3.1 Purpose of Directions

Directions act as:
	•	Safety constraints
	•	Behavioral invariants
	•	User-defined global policies
	•	Model steering heuristics
	•	Overrides when LLM rules would behave too aggressively

Examples:
	•	“Never delete or permanently remove email unless explicitly allowed.”
	•	“When uncertain, choose a safe, reversible action.”
	•	“Do not forward emails unless explicitly instructed via a rule.”

3.3.2 Application

Directions are always included in LLM classification through:
	•	The “DIRECTIONS” layer of the LLM prompt (Section 10)
	•	Post-processing enforcement in Rust after receiving the model output

If an LLM rule contradicts a direction, the direction always wins.

3.3.3 Persistence

Directions are stored in the directions table:
	•	id
	•	content (NL instruction)
	•	enabled flag
	•	timestamps

Users typically modify directions infrequently.

⸻

3.4 LLM Rules (Situational Natural-Language Rules)

LLM Rules are natural-language rule definitions that describe case-specific or category-specific behavior the user wants.
They are evaluated only when deterministic rules do not match.

3.4.1 Rule Structure

Each LLM rule includes:
	•	id
	•	name
	•	description
	•	scope (global | account | sender | domain)
	•	scope_ref (optional)
	•	rule_text — natural-language behavioral rule
	•	enabled flag
	•	metadata_json — additional hints

Examples:
	•	“Invoices should be labeled Finance/Invoices and archived after filing.”
	•	“Email related to HR must stay in the inbox and be marked important.”

LLM rules do not directly output actions; instead, they influence LLM decision-making through prompt inclusion.

⸻

3.5 Rule Evaluation Order

The following is the definitive evaluation pipeline:

Step 1 — Deterministic Rules
	1.	Load all enabled deterministic rules relevant to the account/domain/sender.
	2.	Sort by priority.
	3.	Evaluate conditions.
	4.	If any rules match:
	•	Produce deterministic actions.
	•	Apply safety gating (dangerous overrides).
	•	Terminate rule evaluation (no LLM).

Step 2 — Directions

If deterministic rules do not match:
	•	Load all enabled = 1 directions.
	•	Directions are injected as a mandatory global policy layer into the LLM prompt.
	•	Directions constrain every possible LLM output.

Step 3 — LLM Rules
	•	Load all enabled LLM rules relevant to the message.
	•	Insert them into the prompt context after directions.
	•	LLM evaluates the message under:
	•	System instructions
	•	Directions (hard constraints)
	•	LLM rules (soft, situational guidance)
	•	Message context

Step 4 — LLM Decision Generation

The model generates:
	•	Action
	•	Parameters
	•	Confidence
	•	Needs approval
	•	Rationale
	•	Undo hints

Step 5 — Rust Enforcement

Rust validates and post-processes the decision using `SafetyEnforcer`:
	1.	**Danger Level Check**: Dangerous actions (Delete, Forward, AutoReply, Escalate) always require approval
	2.	**Confidence Threshold**: If confidence < `policy.confidence_default`, require approval
	3.	**approval_always List**: Actions in `policy.approval_always` always require approval
	4.	**LLM Advisory**: Honor LLM's `needs_approval` flag if set to true
	5.	Persist decision with safety telemetry (overrides applied)
	6.	Enqueue next job (auto-run safe actions or create Discord approval request)

See `server/crates/ashford-core/src/decisions/safety.rs` for implementation details.

⸻

3.6 Deterministic Rules vs Directions vs LLM Rules

Layer	Purpose	Example	Who Edits It	Priority
Deterministic Rules	Explicit, structured logic	“If from *@amazon.com → label Receipts”	User via UI/editor	Highest (first)
Directions	Global guardrails	“Never delete unless deterministic rule says so”	User (rarely)	Always applied (overrules LLM rules)
LLM Rules	Situational NL rules	“HR email → keep in inbox”	User via rules assistant	Only after deterministic rules

Enforcement hierarchy:

Deterministic Rules > Directions > LLM Rules

⸻

3.7 Benefits of the Three-Layer Model
	•	Predictability: deterministic rules override everything else.
	•	Safety: directions prevent unwanted destructive actions.
	•	Flexibility: LLM rules provide intelligent behavior for complex or ambiguous categories.
	•	Auditability: every layer is visible and persists in storage.
	•	Reduced hallucination: directions anchor the model’s reasoning.
	•	Separation of concerns: rules define “when X happens do Y,” directions define “how the system must behave.”


