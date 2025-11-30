---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Phase 4.5: Safety Enforcement & Post-Processing"
goal: Implement Rust-side safety enforcement, dangerous action policy, and
  confidence thresholds
id: 17
uuid: 85737737-8826-483b-9a82-87e7c0098c90
status: pending
priority: high
container: false
temp: false
dependencies:
  - 13
  - 16
parent: 4
issue: []
docs:
  - docs/decision_engine.md
  - docs/rules_engine.md
createdAt: 2025-11-30T01:14:19.376Z
updatedAt: 2025-11-30T01:14:19.376Z
tasks: []
tags: []
---

Critical safety layer that validates LLM output against directions and policy constraints. This ensures safe behavior even with imperfect model output.

## Key Components

### Direction Enforcement
After receiving LLM decision:
1. Check if action violates any enabled direction
2. If violation detected:
   - Downgrade to safe fallback action (e.g., `mark_unread`, `archive`)
   - OR mark as requiring approval
3. Log safety override in telemetry

Example enforcement:
- Direction: "Never delete unless deterministic rule allows"
- LLM output: `action: delete`
- Enforcement: Override to `needs_approval: true` or downgrade to `archive`

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
- Dangerous: require approval unless:
  - Deterministic rule with `safe_mode = 'dangerous_override'`
  - High confidence AND in approval whitelist

### Confidence Thresholds
Use config `PolicyConfig`:
```rust
pub struct PolicyConfig {
    pub approval_always: Vec<String>,  // Actions always needing approval
    pub confidence_default: f32,       // Threshold below which approval needed
}
```

Logic:
- If `confidence < confidence_default` → `needs_approval = true`
- If action in `approval_always` → `needs_approval = true`
- LLM's `needs_approval` is advisory, Rust policy overrides

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
    pub original_action: Option<ActionType>,
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
