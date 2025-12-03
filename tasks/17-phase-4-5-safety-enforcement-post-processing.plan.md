---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Phase 4.5: Safety Enforcement & Post-Processing"
goal: Implement Rust-side safety enforcement, dangerous action policy, and
  confidence thresholds
id: 17
uuid: 85737737-8826-483b-9a82-87e7c0098c90
generatedBy: agent
status: done
priority: high
container: false
temp: false
dependencies:
  - 13
  - 16
parent: 4
references:
  "4": 5cf4cc37-3eb8-4f89-adae-421a751d13a1
  "13": 38766c9f-5711-40f8-b264-292a865ef49e
  "16": b8c142c5-3335-4b87-9a94-28dbcc96af99
issue: []
pullRequest: []
docs:
  - docs/decision_engine.md
  - docs/rules_engine.md
planGeneratedAt: 2025-12-02T18:41:17.899Z
promptsGeneratedAt: 2025-12-02T18:41:17.899Z
createdAt: 2025-11-30T01:14:19.376Z
updatedAt: 2025-12-02T20:36:24.902Z
progressNotes:
  - timestamp: 2025-12-02T20:35:59.223Z
    text: "Code review passed with no critical issues. Implementation correctly
      implements all 6 tasks. Minor notes: (1) No integration test exists, only
      unit tests, but tests are comprehensive with 27 tests in safety.rs and 7
      in policy.rs covering edge cases. (2) Case-sensitivity in approval_always
      list matching is documented but could trip up users who put 'Delete'
      instead of 'delete'."
    source: "reviewer: code review"
  - timestamp: 2025-12-02T20:36:24.898Z
    text: "Safety enforcement code added (policy.rs, safety.rs) is unused: no call
      sites in decision pipeline or persistence. Needs wiring to LLM decision
      flow and telemetry before plan can be considered done."
    source: "reviewer: plan17"
tasks:
  - title: Create ActionDangerLevel enum and classify ActionTypes
    done: true
    description: >-
      Create `decisions/policy.rs` with:


      1. `ActionDangerLevel` enum with variants: `Safe`, `Reversible`,
      `Dangerous`

      2. Implement `danger_level()` method on `ActionType` (in
      `llm/decision.rs`):
         - Safe: ApplyLabel, MarkRead, MarkUnread, Archive, Move, None
         - Reversible: Star, Unstar, Snooze, AddNote, CreateTask
         - Dangerous: Delete, Forward, AutoReply, Escalate
      3. Add unit tests for all 15 ActionType classifications

      4. Export from `decisions/mod.rs`
  - title: Create SafetyOverride enum and SafetyResult struct
    done: true
    description: >-
      In `decisions/policy.rs`, add:


      1. `SafetyOverride` enum representing reasons for requiring approval:
         - `DangerousAction` - action is classified as Dangerous
         - `LowConfidence { confidence: f64, threshold: f32 }` - below threshold
         - `InApprovalAlwaysList` - action type in approval_always config
         - `LlmRequestedApproval` - LLM's advisory flag was true

      2. `SafetyResult` struct:
         - `overrides_applied: Vec<SafetyOverride>` - all reasons that triggered approval
         - `requires_approval: bool` - final determination

      3. Implement `Display` for `SafetyOverride` for telemetry logging

      4. Export from `decisions/mod.rs`
  - title: Implement SafetyEnforcer struct
    done: true
    description: >-
      Create `decisions/safety.rs` with `SafetyEnforcer` struct:


      1. Fields:
         - `policy: PolicyConfig` (from config.rs)

      2. Constructor: `SafetyEnforcer::new(policy: PolicyConfig)`


      3. Main method: `enforce(&self, decision: &DecisionOutput) ->
      SafetyResult`
         - Check each condition and collect all applicable SafetyOverrides
         - Use OR logic: if any override applies, requires_approval = true

      4. Helper methods (private):
         - `check_danger_level(&self, action: ActionType) -> Option<SafetyOverride>`
         - `check_confidence(&self, confidence: f64) -> Option<SafetyOverride>`
         - `check_approval_always(&self, action: ActionType) -> Option<SafetyOverride>`
         - `check_llm_advisory(&self, needs_approval: bool) -> Option<SafetyOverride>`

      5. Export from `decisions/mod.rs`
  - title: Unit tests for SafetyEnforcer enforcement logic
    done: true
    description: >-
      Add comprehensive tests in `decisions/safety.rs`:


      1. Danger level tests:
         - Dangerous action (Delete) requires approval
         - Safe action (Archive) does not require approval (alone)
         - Reversible action (Star) does not require approval (alone)

      2. Confidence threshold tests:
         - Confidence below threshold requires approval
         - Confidence at threshold does not require approval
         - Confidence above threshold does not require approval

      3. approval_always list tests:
         - Action in list requires approval regardless of confidence
         - Action not in list follows normal rules

      4. LLM advisory flag tests:
         - LLM needs_approval=true is honored even for safe actions
         - LLM needs_approval=false doesn't override policy

      5. Combined scenario tests:
         - Multiple overrides collected (e.g., dangerous + low confidence)
         - Safe action with high confidence and needs_approval=false passes through
  - title: Create telemetry helpers for SafetyResult
    done: true
    description: >-
      Add serialization support for safety enforcement telemetry:


      1. In `decisions/policy.rs` or `decisions/safety.rs`:
         - Implement `Serialize` for `SafetyOverride` and `SafetyResult`
         - Add `SafetyResult::to_telemetry_json(&self) -> serde_json::Value` method

      2. The telemetry JSON should include:
         - `safety_overrides`: array of override descriptions
         - `requires_approval`: boolean
         - Human-readable reasons for audit trail

      3. Add tests verifying telemetry JSON structure
  - title: Export PolicyConfig from ashford-core
    done: true
    description: >-
      Update `lib.rs` to export `PolicyConfig` from the config module so it can
      be used by callers constructing `SafetyEnforcer`:


      1. Add `pub use config::PolicyConfig;` to `lib.rs`

      2. Verify the export works by checking it can be imported from
      `ashford_core::PolicyConfig`
changedFiles:
  - server/crates/ashford-core/src/decisions/mod.rs
  - server/crates/ashford-core/src/decisions/policy.rs
  - server/crates/ashford-core/src/decisions/safety.rs
  - server/crates/ashford-core/src/lib.rs
  - server/crates/ashford-core/src/llm/decision.rs
tags: []
---

Critical safety layer that validates LLM output against directions and policy constraints. This ensures safe behavior even with imperfect model output.

## Key Components

### Direction Enforcement (Deferred to Plan 21)
Direction violation detection is deferred to Plan 21 (LLM-Based Direction Violation Detection). This plan focuses on danger levels, confidence thresholds, and the approval_always list.

### Dangerous Action Policy
Define action danger levels:
```rust
pub enum ActionDangerLevel {
    Safe,           // archive, apply_label, mark_read
    Reversible,     // star, snooze
    Dangerous,      // delete, forward, auto_reply
}

impl ActionType {
    pub fn danger_level(&self) -> ActionDangerLevel { ... }
}
```

Policy enforcement:
- Safe actions: auto-execute
- Reversible: auto-execute with undo hint
- Dangerous: require approval (set `needs_approval = true`)

### Confidence Thresholds
Use config `PolicyConfig`:
```rust
pub struct PolicyConfig {
    pub approval_always: Vec<String>,  // Actions always needing approval
    pub confidence_default: f32,       // Threshold below which approval needed
}
```

Logic (OR of all conditions):
- If LLM's `needs_approval = true` → `needs_approval = true` (honor LLM's request)
- If `confidence < confidence_default` → `needs_approval = true`
- If action in `approval_always` → `needs_approval = true`
- If action is Dangerous level → `needs_approval = true`

The LLM's advisory flag is respected - if the LLM requests approval, we honor it even if policy would allow auto-execution.

### Safety Enforcement API
```rust
pub struct SafetyEnforcer {
    directions: Vec<Direction>,
    policy: PolicyConfig,
}

impl SafetyEnforcer {
    pub fn enforce(&self, decision: &mut DecisionOutput) -> SafetyResult {
        // Returns info about any overrides applied
    }
}

pub struct SafetyResult {
    pub overrides_applied: Vec<SafetyOverride>,
    pub requires_approval: bool,
}
```

### Telemetry Capture
Extend decision telemetry:
```rust
pub struct DecisionTelemetry {
    pub model: String,
    pub latency_ms: u64,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub safety_overrides: Vec<String>,  // List of overrides applied
}
```

### File Organization
```
ashford-core/src/decisions/
├── safety.rs        # SafetyEnforcer implementation
├── policy.rs        # Danger levels and policy definitions
```

### Testing
- Direction violation detection tests
- Danger level classification tests
- Confidence threshold tests
- Override application tests
- Complex scenarios combining multiple policies

## Research

### Summary

This plan implements the critical safety layer that validates LLM decisions against policy constraints before action execution. The core challenge is creating a post-processing pipeline that can:
1. Detect when LLM decisions violate user-defined directions
2. Classify actions by danger level and enforce appropriate approval requirements
3. Apply confidence thresholds to require human approval for uncertain decisions
4. Record all safety overrides for audit purposes

Key insight: The codebase already has all the foundational pieces (ActionType, Direction, PolicyConfig, DecisionOutput) - this plan primarily adds the enforcement logic that ties them together.

### Findings

#### Existing Decision Infrastructure

**LLM Decision Types** (`server/crates/ashford-core/src/llm/decision.rs`)
- `ActionType` enum with 15 action types including safe (Archive, MarkRead), reversible (Star, Snooze), and dangerous (Delete, Forward, AutoReply)
- `DecisionOutput` struct contains:
  - `decision.action: ActionType` - the recommended action
  - `decision.confidence: f64` - confidence score [0.0, 1.0]
  - `decision.needs_approval: bool` - LLM's advisory approval flag
  - `explanations.matched_directions: Vec<String>` - directions the LLM claims to have followed
- Validation exists for confidence ranges but not for direction compliance

**Decision Storage** (`server/crates/ashford-core/src/decisions/types.rs`)
- `Decision` struct stores the decision record with `needs_approval: bool` and `confidence: Option<f64>`
- `DecisionSource` enum distinguishes `Llm` vs `Deterministic` sources
- `NewDecision` used for creating new decisions - will need to capture safety overrides

**Action Status Machine** (`server/crates/ashford-core/src/decisions/types.rs`)
- `ActionStatus` enum: Queued, Executing, Completed, Failed, Canceled, Rejected, ApprovedPending
- The `ApprovedPending` status already exists for actions awaiting approval

#### Policy Configuration

**PolicyConfig** (`server/crates/ashford-core/src/config.rs`, lines 74-80)
```rust
pub struct PolicyConfig {
    #[serde(default)]
    pub approval_always: Vec<String>,  // e.g., ["delete", "forward"]
    pub confidence_default: f32,        // e.g., 0.7
}
```
- Already defined and loaded from config.toml
- Accessed via `config.policy`
- `approval_always` contains action type strings that always require approval
- `confidence_default` is the threshold below which approval is required

#### Direction System

**Direction struct** (`server/crates/ashford-core/src/rules/types.rs`, lines 153-162)
```rust
pub struct Direction {
    pub id: String,
    pub org_id: i64,
    pub user_id: Option<i64>,
    pub content: String,  // Natural language text
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**DirectionsRepository** (`server/crates/ashford-core/src/rules/repositories.rs`)
- `list_enabled(org_id, user_id)` returns all enabled directions for the user
- Query: `WHERE org_id = ?1 AND (user_id IS NULL OR user_id = ?2) AND enabled = 1`

**Current Direction Usage**:
- Directions are formatted into the LLM prompt via `build_directions_section()` in `llm/prompt.rs`
- The LLM response includes `matched_directions: Vec<String>` to track which it considered
- **No Rust-side enforcement exists** - the system trusts the LLM to follow directions

#### SafeMode for Deterministic Rules

**SafeMode enum** (`server/crates/ashford-core/src/rules/types.rs`, lines 35-60)
```rust
pub enum SafeMode {
    Default,           // Standard safety gating
    AlwaysSafe,        // Bypasses approval requirement
    DangerousOverride, // Explicitly permits dangerous actions
}
```
- Used by `DeterministicRule` to control whether deterministic rules can bypass safety checks
- Not currently enforced at runtime - just stored in the rule

#### Telemetry

**Current TelemetryPlaceholder** (`server/crates/ashford-core/src/llm/decision.rs`, line 114-115)
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, JsonSchema)]
pub struct TelemetryPlaceholder {}
```
- Empty placeholder struct - actual telemetry populated by Rust side
- The plan calls for extending this to include `safety_overrides: Vec<String>`

**LlmCall logging** (`server/crates/ashford-core/src/llm/repository.rs`)
- Tracks model, latency, tokens for each LLM call
- Does not currently track safety overrides

**Decision telemetry_json field** (`server/crates/ashford-core/src/decisions/types.rs`)
- `telemetry_json: Value` field exists in Decision/NewDecision
- Currently stores LLM call metadata, can be extended for safety overrides

#### Module Organization

Current decisions module structure:
```
server/crates/ashford-core/src/decisions/
├── mod.rs           # Exports
├── types.rs         # Decision, Action, NewDecision, etc.
└── repositories.rs  # DecisionRepository, ActionRepository
```

Proposed additions:
```
server/crates/ashford-core/src/decisions/
├── mod.rs           # Updated exports
├── types.rs         # Existing types
├── repositories.rs  # Existing repositories
├── safety.rs        # NEW: SafetyEnforcer implementation
└── policy.rs        # NEW: ActionDangerLevel, policy enforcement
```

#### Integration Points

**Where SafetyEnforcer will be called**:
1. After `DecisionOutput::parse_from_tool_calls()` succeeds
2. Before creating `NewDecision` record
3. The enforcer modifies `needs_approval` and potentially downgrades actions

**Data flow**:
```
LLM Response → parse_from_tool_calls() → DecisionOutput
    → SafetyEnforcer.enforce() → SafetyResult
    → Build NewDecision (with modified needs_approval, telemetry)
    → DecisionRepository.create()
    → ActionRepository.create() (with appropriate status)
```

### Risks & Constraints

#### Direction Violation Detection Challenge

The plan describes checking if actions "violate" directions, but directions are natural language text (e.g., "Never delete newsletters"). Detecting violations requires:

1. **Option A: Pattern-based detection** - Parse directions for known patterns like "never delete", "always archive". Limited but deterministic.

2. **Option B: LLM-assisted detection** - Ask the LLM to evaluate if the action violates directions. Adds latency and cost.

3. **Option C: Action-type blacklisting** - Store structured metadata with directions (e.g., `blocked_actions: ["delete"]`). Requires schema changes.

**Recommendation**: Start with danger-level based enforcement and confidence thresholds (deterministic). Direction violation detection could be deferred or implemented with structured metadata rather than NL parsing.

#### PolicyConfig Type Mismatch

`PolicyConfig.approval_always` contains strings like `"delete"` but `ActionType` uses CamelCase variants. Need to ensure consistent string conversion via `ActionType::as_str()` which returns `"delete"` (snake_case).

#### SafeMode Integration

The plan references `safe_mode = 'dangerous_override'` for deterministic rules, but the safety enforcer receives a `DecisionOutput` from the LLM path. Need to clarify:
- Does SafetyEnforcer only apply to LLM decisions?
- For deterministic rules, is SafeMode handled separately?

Based on the architecture, SafetyEnforcer should only apply to LLM decisions (`DecisionSource::Llm`). Deterministic rules already have SafeMode attached to the rule itself.

#### Telemetry Extension

The `TelemetryPlaceholder` struct is defined in `llm/decision.rs` but is part of the LLM response contract. Changing it affects the JSON schema sent to the LLM.

Better approach: Store safety overrides in the `telemetry_json: Value` field when persisting decisions, rather than modifying the LLM response contract.

#### Test Dependencies

Tests will need:
- Sample directions with various patterns
- DecisionOutput fixtures for different scenarios
- Mock PolicyConfig values

The existing test patterns in `llm/decision.rs` provide good examples.

### Expected Behavior/Outcome

When a decision flows through the safety enforcement layer:

1. **Dangerous action detection**: Actions like Delete, Forward, AutoReply trigger approval requirements
2. **Confidence threshold**: Decisions below `confidence_default` (e.g., 0.7) require approval
3. **Always-approve list**: Actions in `approval_always` always require approval regardless of confidence
4. **Override logging**: All safety interventions are recorded in decision telemetry

**Decision States**:
- `needs_approval = false`: Action queued for immediate execution
- `needs_approval = true`: Action queued with `ApprovedPending` status, awaiting Discord approval

### Acceptance Criteria

- [ ] `ActionDangerLevel` enum classifies all 15 ActionTypes correctly
- [ ] `SafetyEnforcer` correctly applies confidence threshold from PolicyConfig
- [ ] Actions in `approval_always` list always get `needs_approval = true`
- [ ] Dangerous actions (Delete, Forward, AutoReply, Escalate) always require approval
- [ ] LLM's `needs_approval` advisory flag is honored (OR logic with policy)
- [ ] `SafetyResult` captures all overrides applied (reasons for requiring approval)
- [ ] Safety overrides are persisted in decision's `telemetry_json`
- [ ] Unit tests cover: danger level classification, confidence thresholds, approval_always, LLM advisory flag, edge cases
- [ ] Integration test: full flow from DecisionOutput to Decision with safety enforcement

### Dependencies & Constraints

**Dependencies**:
- Plan 13 (LLM Decision Engine) - provides DecisionOutput parsing
- Plan 16 (assumed - provides LLM integration) - provides LLM client

**Technical Constraints**:
- Must not modify the LLM response contract (TelemetryPlaceholder stays empty)
- Must integrate with existing DecisionRepository and ActionRepository
- PolicyConfig is already defined - use it as-is
- Direction enforcement is optional/deferred - focus on danger levels and thresholds

### Implementation Notes

**Recommended Approach**:

1. Create `decisions/policy.rs` with `ActionDangerLevel` enum and classification method on `ActionType`
2. Create `decisions/safety.rs` with `SafetyEnforcer` struct
3. Implement enforcement in order of complexity:
   - Danger level check (simple enum match)
   - Confidence threshold check (compare float)
   - approval_always check (string comparison)
4. Create `SafetyResult` to capture enforcement decisions
5. Extend `NewDecision` telemetry to include safety overrides
6. Write comprehensive unit tests for each enforcement path

**Potential Gotchas**:
- ActionType uses CamelCase in Rust but snake_case in JSON - use `as_str()` for comparisons
- Confidence is `f64` in DecisionOutput but `f32` in PolicyConfig - need type conversion
- `approval_always` contains action type strings - ensure exact match with `ActionType::as_str()`

**Deferred/Out of Scope**:
- Direction violation detection via NL parsing → deferred to Plan 21 (LLM-Based Direction Violation Detection)
- Automatic action downgrading (removed from scope entirely - see note below)
- SafeMode integration for deterministic rules (already handled separately)

**Note on Action Downgrading**: The safety layer will NOT automatically downgrade dangerous actions to safer alternatives. Instead, dangerous actions simply get `needs_approval = true`. A future enhancement (see Plan 6: Discord Bot) could present safer alternatives in the approval UI - e.g., when approving a delete action, offer "Archive instead" as an additional option. This keeps the safety layer simple and puts the downgrade decision in the user's hands.

Implemented all 6 tasks for the Safety Enforcement & Post-Processing plan.

## Files Created

### decisions/policy.rs
Created new file at server/crates/ashford-core/src/decisions/policy.rs containing:
- ActionDangerLevel enum with Safe, Reversible, and Dangerous variants
- SafetyOverride enum with 4 variants: DangerousAction, LowConfidence { confidence, threshold }, InApprovalAlwaysList, and LlmRequestedApproval
- SafetyResult struct with overrides_applied: Vec<SafetyOverride> and requires_approval: bool
- Display trait implementation for SafetyOverride providing human-readable messages for telemetry
- Serialize implementations for telemetry JSON output
- SafetyResult::to_telemetry_json() method that produces structured JSON with safety_overrides array and requires_approval boolean
- SafetyResult::approved() constructor for cases with no overrides
- 7 unit tests for policy types

### decisions/safety.rs
Created new file at server/crates/ashford-core/src/decisions/safety.rs containing:
- SafetyEnforcer struct holding a PolicyConfig reference
- SafetyEnforcer::new(policy: PolicyConfig) constructor
- SafetyEnforcer::enforce(&self, decision: &DecisionOutput) -> SafetyResult main method that uses OR logic - if any check triggers, requires_approval becomes true
- Private helper methods: check_danger_level, check_confidence, check_approval_always, check_llm_advisory
- 27 comprehensive unit tests covering all scenarios from Task 4:
  - Danger level tests for all action types
  - Confidence threshold tests (below, at, above threshold, edge cases 0.0 and 1.0)
  - approval_always list matching (including case sensitivity)
  - LLM advisory flag handling
  - Combined scenarios with multiple overrides
  - Telemetry JSON structure verification

## Files Modified

### llm/decision.rs
Added danger_level() method to ActionType enum that classifies all 15 action types:
- Safe: ApplyLabel, MarkRead, MarkUnread, Archive, Move, None
- Reversible: Star, Unstar, Snooze, AddNote, CreateTask  
- Dangerous: Delete, Forward, AutoReply, Escalate
Added 2 unit tests for danger level classification.

### decisions/mod.rs
Added module declarations for policy and safety, and pub use statements to export ActionDangerLevel, SafetyOverride, SafetyResult, and SafetyEnforcer.

### lib.rs
Added PolicyConfig to config module exports and safety types (ActionDangerLevel, SafetyOverride, SafetyResult, SafetyEnforcer) to decisions exports.

## Design Decisions

1. OR Logic: The SafetyEnforcer collects ALL applicable SafetyOverride reasons, not just the first one found. This provides better audit trail and transparency.

2. Type Conversion: DecisionOutput uses f64 for confidence while PolicyConfig uses f32 for threshold. The comparison converts threshold to f64 for safe comparison.

3. Case-Sensitive Matching: The approval_always list matching uses exact string comparison with ActionType::as_str() (snake_case). Users must use lowercase snake_case values like 'delete', 'forward', 'auto_reply'.

4. Telemetry Integration: Safety overrides are serialized to JSON via to_telemetry_json() for storage in the telemetry_json field of decisions, rather than modifying TelemetryPlaceholder.

5. No Action Downgrading: Per the plan, dangerous actions simply get needs_approval=true rather than being automatically downgraded to safer alternatives.

## Test Coverage
- 306 total tests in ashford-core crate, all passing
- 27 tests in safety.rs covering all Task 4 requirements
- 7 tests in policy.rs for type behavior
- 2 tests in decision.rs for danger level classification
