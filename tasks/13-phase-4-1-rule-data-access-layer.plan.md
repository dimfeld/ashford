---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Phase 4.1: Rule Data Access Layer"
goal: Create repository classes and domain types for deterministic rules, LLM
  rules, directions, decisions, and actions
id: 13
uuid: 38766c9f-5711-40f8-b264-292a865ef49e
status: pending
priority: high
container: false
temp: false
dependencies: []
parent: 4
issue: []
docs:
  - docs/data_model.md
  - docs/rules_engine.md
createdAt: 2025-11-30T01:14:18.743Z
updatedAt: 2025-11-30T01:14:18.743Z
tasks: []
tags: []
---

Foundation layer that provides data access for all rule types. This must be completed before any rule evaluation logic can be implemented.

## Key Components

### Domain Types
- `DeterministicRule` struct matching schema (id, name, scope, priority, conditions_json, action_type, etc.)
- `LLMRule` struct (id, name, scope, rule_text, metadata_json, etc.)
- `Direction` struct (id, content, enabled)
- `Decision` struct matching the JSON contract in decision_engine.md
- `Action` struct with status tracking

### Repositories
- `DeterministicRuleRepository` - load by scope (global, account, sender, domain), list all, create
- `LLMRuleRepository` - load by scope, list all, create
- `DirectionsRepository` - load all enabled, list all, create
- `DecisionRepository` - create, get by id, get by message_id
- `ActionRepository` - create, update status, get by decision_id

### Patterns to Follow
- Match existing repository patterns (AccountRepository, MessageRepository)
- Use `Database` wrapper with async methods
- Return `Result<T, XyzError>` with appropriate error types
- JSON serialization for complex fields using serde_json

### File Organization
```
ashford-core/src/
├── rules/
│   ├── mod.rs
│   ├── types.rs          # Domain structs
│   └── repositories.rs   # All rule repositories
├── decisions/
│   ├── mod.rs
│   ├── types.rs          # Decision & Action structs
│   └── repositories.rs   # Decision & Action repositories
```

### Testing
- Unit tests with in-memory database
- Test CRUD operations for each repository
- Test scope filtering for rules
