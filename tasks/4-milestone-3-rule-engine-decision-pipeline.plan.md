---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Milestone 3: Rule Engine & Decision Pipeline"
goal: Implement the three-layer rule system (deterministic rules, directions,
  LLM rules) and decision pipeline
id: 4
uuid: 5cf4cc37-3eb8-4f89-adae-421a751d13a1
generatedBy: agent
status: in_progress
priority: high
container: false
temp: false
dependencies:
  - 3
  - 13
  - 14
  - 15
  - 16
  - 17
  - 18
parent: 1
issue: []
docs:
  - docs/rules_engine.md
  - docs/data_model.md
planGeneratedAt: 2025-11-29T01:23:12.069Z
promptsGeneratedAt: 2025-11-29T01:23:12.069Z
createdAt: 2025-11-29T01:21:26.793Z
updatedAt: 2025-11-30T01:25:26.781Z
tasks:
  - title: Define condition tree schema
    done: false
    description: "Design JSON schema for conditions_json: AND/OR/NOT operators, leaf
      conditions (sender_email, sender_domain, subject_regex, header_match,
      label_present, etc.)."
  - title: Build condition evaluator
    done: false
    description: Implement recursive evaluator for condition trees. Support wildcard
      matching, regex, substring, exact match. Handle all leaf condition types.
  - title: Implement deterministic rule loader
    done: false
    description: Load enabled deterministic rules for a message's
      account/domain/sender. Sort by priority. Support scope filtering (global,
      account, sender, domain).
  - title: Create deterministic rule executor
    done: false
    description: Evaluate rules in priority order. On match, produce action with
      parameters. Respect first-match vs all-matches configuration.
  - title: Build directions loader
    done: false
    description: Load all enabled directions from database. Format for LLM prompt
      inclusion. Cache for performance.
  - title: Implement LLM rules loader
    done: false
    description: Load enabled LLM rules relevant to message scope. Format rule_text
      for prompt inclusion.
  - title: Design LLM prompt template
    done: false
    description: "Create prompt structure with sections: system instructions,
      DIRECTIONS (global constraints), LLM RULES (situational guidance), EMAIL
      CONTEXT (headers, body snippet), OUTPUT FORMAT (JSON schema)."
  - title: Integrate LLM provider
    done: false
    description: Use genai crate or build provider abstraction. Support Gemini,
      OpenAI, Anthropic. Implement retry with backoff, timeout handling, token
      counting.
  - title: Parse LLM decision output
    done: false
    description: "Define decision JSON schema: action_type, parameters, confidence,
      needs_approval, rationale, undo_hints. Validate and parse LLM response."
  - title: Implement safety enforcement
    done: false
    description: "Post-process LLM decisions: enforce directions (e.g., never delete
      unless allowed), apply dangerous action policy, check confidence
      thresholds."
  - title: Create classify job handler
    done: false
    description: "Orchestrate classification: load rules, try deterministic first,
      fall back to LLM, enforce safety, persist decision, enqueue action or
      approval job."
  - title: Store telemetry data
    done: false
    description: "Record in telemetry_json: model used, prompt/completion tokens,
      latency, temperature, any errors or retries."
tags:
  - ai
  - llm
  - rules
  - rust
---

Rule evaluation and LLM classification:
- Deterministic rule evaluation engine with condition tree parsing
- Directions loading and enforcement
- LLM rules integration with prompt construction
- LLM decision engine (via genai crate or similar)
- Decision JSON schema and validation
- Dangerous action policy implementation
- Confidence thresholds and auto-approval logic
- decisions table storage
