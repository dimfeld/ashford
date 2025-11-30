---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Phase 4.2: Deterministic Rule Engine"
goal: Implement condition tree parsing and evaluation, and deterministic rule
  execution
id: 14
uuid: 4faa40e3-cbc5-4d8c-a596-225ab64a50d9
status: pending
priority: high
container: false
temp: false
dependencies:
  - 13
parent: 4
issue: []
docs:
  - docs/rules_engine.md
createdAt: 2025-11-30T01:14:18.896Z
updatedAt: 2025-11-30T01:14:18.896Z
tasks: []
tags: []
---

Core deterministic rule evaluation engine that provides the "fast path" for email classification. This handles explicit, structured rules before LLM involvement.

## Key Components

### Condition Tree Schema & Parser
Define JSON schema for condition trees supporting:
- **Logical operators**: AND, OR, NOT
- **Leaf conditions**:
  - `sender_email` - exact match or wildcard (e.g., `*@amazon.com`)
  - `sender_domain` - domain match
  - `subject_contains` - substring match
  - `subject_regex` - regex pattern match
  - `header_match` - specific header value check
  - `label_present` - Gmail label exists

Example condition tree:
```json
{
  "op": "AND",
  "children": [
    { "type": "sender_domain", "value": "amazon.com" },
    { "op": "OR", "children": [
      { "type": "subject_contains", "value": "shipped" },
      { "type": "subject_contains", "value": "delivered" }
    ]}
  ]
}
```

### Condition Evaluator
- Recursive tree evaluation
- Pattern matching for different condition types
- Efficient regex caching if needed
- Returns `bool` for match result

### Rule Loader
- Load rules by scope (global → account → domain → sender)
- Sort by priority (ascending)
- Filter to enabled rules only

### Rule Executor
- Evaluate rules against message metadata
- Support first-match vs all-matches mode (configurable)
- Return list of matched rules with actions
- Handle `safe_mode` field for dangerous action policy

### File Organization
```
ashford-core/src/rules/
├── conditions.rs    # Condition types and evaluator
├── deterministic.rs # Rule loader and executor
```

### Testing
- Unit tests for each condition type
- Complex condition tree evaluation tests
- Priority ordering tests
- First-match vs all-matches mode tests
