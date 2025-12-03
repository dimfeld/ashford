---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Milestone 3: Rule Engine & Decision Pipeline"
goal: Implement the three-layer rule system (deterministic rules, directions,
  LLM rules) and decision pipeline
id: 4
uuid: 5cf4cc37-3eb8-4f89-adae-421a751d13a1
generatedBy: agent
status: done
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
  - 21
  - 20
parent: 1
references:
  "1": 076d03b1-833c-4982-b0ca-1d8868d40e31
  "3": b93a0b33-fccb-4f57-8c97-002039917c44
  "13": 38766c9f-5711-40f8-b264-292a865ef49e
  "14": 4faa40e3-cbc5-4d8c-a596-225ab64a50d9
  "15": 01e10898-4dba-4343-902f-cd5ab57178eb
  "16": b8c142c5-3335-4b87-9a94-28dbcc96af99
  "17": 85737737-8826-483b-9a82-87e7c0098c90
  "18": 9def82bc-4c74-4945-882a-81a674f25cf1
  "20": da4dd6ae-fb36-49f3-b153-079eaf9524b0
  "21": 5fd30460-6ccf-4ca9-8e19-15f7888f47a8
issue: []
pullRequest: []
docs:
  - docs/rules_engine.md
  - docs/data_model.md
planGeneratedAt: 2025-11-29T01:23:12.069Z
promptsGeneratedAt: 2025-11-29T01:23:12.069Z
createdAt: 2025-11-29T01:21:26.793Z
updatedAt: 2025-12-02T18:34:52.629Z
progressNotes: []
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
