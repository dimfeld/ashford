---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: LLM-Based Direction Violation Detection
goal: Implement LLM-assisted detection of direction violations in email
  classification decisions
id: 21
uuid: 5fd30460-6ccf-4ca9-8e19-15f7888f47a8
status: pending
priority: medium
container: false
temp: false
dependencies:
  - 17
parent: 4
references: {}
issue: []
pullRequest: []
docs:
  - docs/decision_engine.md
  - docs/rules_engine.md
createdAt: 2025-12-02T18:34:52.627Z
updatedAt: 2025-12-02T18:34:52.627Z
progressNotes: []
tasks: []
tags: []
---

After the safety enforcement layer (Plan 17) is in place with danger levels, confidence thresholds, and approval_always checks, add a second-pass LLM evaluation to detect when decisions violate natural language directions.

## Context

Directions are natural language instructions like:
- "Never delete newsletters"
- "Always archive receipts from Amazon"
- "Do not forward emails containing confidential information"

These cannot be reliably enforced with pattern matching. An LLM can evaluate whether a proposed action violates the intent of each direction.

## Proposed Approach

1. After initial decision is made, if action is potentially risky (e.g., delete, forward), invoke a lightweight LLM call
2. Provide the decision, the email context, and all enabled directions
3. Ask the LLM to identify any direction violations
4. If violations detected, override to `needs_approval = true` or downgrade action

## Considerations

- **Latency**: Adds a second LLM call to the pipeline
- **Cost**: Additional token usage per decision
- **Caching**: Could cache direction violation checks for similar email patterns
- **Selective invocation**: Only invoke for dangerous actions or low-confidence decisions to minimize overhead

## Alternative: Structured Direction Metadata

Could also add optional structured metadata to directions (e.g., `blocked_actions: ["delete"]`) for deterministic enforcement of simple rules, reserving LLM evaluation for complex natural language constraints.
