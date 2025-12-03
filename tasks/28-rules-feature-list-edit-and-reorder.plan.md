---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Rules Feature: List, Edit, and Reorder"
goal: Build the rules management pages with tabbed list view, create/edit forms,
  and priority reordering, including all required Rust API endpoints
id: 28
uuid: 22994868-219e-4c67-affd-53b36c2248f7
generatedBy: agent
status: pending
priority: high
container: false
temp: false
dependencies:
  - 26
parent: 7
references: {}
issue: []
pullRequest: []
docs:
  - docs/web_ui.md
planGeneratedAt: 2025-12-03T10:13:27.848Z
promptsGeneratedAt: 2025-12-03T10:13:27.848Z
createdAt: 2025-12-03T09:46:54.782Z
updatedAt: 2025-12-03T10:13:27.848Z
progressNotes: []
tasks:
  - title: Create rules API module in Rust
    done: false
    description: Create server/crates/ashford-server/src/api/rules.rs. Add
      /api/rules routes to the main router.
  - title: Implement GET /api/rules/deterministic endpoint
    done: false
    description: Create list_deterministic_rules handler using
      DeterministicRuleRepository.list_all(). Return array sorted by priority
      descending.
  - title: Implement GET /api/rules/llm endpoint
    done: false
    description: Create list_llm_rules handler using LlmRuleRepository.list_all().
      Return array of LLM rules.
  - title: Implement POST /api/rules/deterministic endpoint
    done: false
    description: Create create_deterministic_rule handler. Validate request body
      (name required, conditions_json valid, action_type valid). Use repository
      to create. Return created rule.
  - title: Implement PATCH /api/rules/deterministic/{id} endpoint
    done: false
    description: Create update_deterministic_rule handler. Accept partial updates
      (name, description, enabled, priority, conditions_json, action_type,
      action_parameters_json, safe_mode). Return updated rule.
  - title: Implement POST /api/rules/llm endpoint
    done: false
    description: Create create_llm_rule handler. Validate request body (name and
      rule_text required). Use repository to create. Return created rule.
  - title: Implement PATCH /api/rules/llm/{id} endpoint
    done: false
    description: Create update_llm_rule handler. Accept partial updates (name,
      description, enabled, rule_text, scope, scope_ref, metadata_json). Return
      updated rule.
  - title: Create rules remote functions
    done: false
    description: "Create web/src/lib/api/rules.remote.ts with queries:
      getDeterministicRules, getLlmRules, getDeterministicRule(id),
      getLlmRule(id). Add commands: createDeterministicRule,
      updateDeterministicRule, createLlmRule, updateLlmRule."
  - title: Build rules list page with tabs
    done: false
    description: "Create web/src/routes/rules/+page.svelte with Tabs component:
      Deterministic Rules, LLM Rules. Each tab shows list with: name, scope
      badge, enabled toggle, conditions summary (for deterministic) or rule_text
      preview (for LLM). Add New Rule button per tab."
  - title: Implement enable/disable toggle
    done: false
    description: "Add Switch component to each rule row. On toggle, call
      updateDeterministicRule or updateLlmRule with enabled: true/false. Show
      optimistic update, revert on error."
  - title: Implement priority reordering
    done: false
    description: Add up/down arrow buttons to deterministic rules list. On click,
      swap priorities with adjacent rule and call updateDeterministicRule for
      both affected rules. Disable up on first item, down on last.
  - title: Build deterministic rule form
    done: false
    description: "Create web/src/routes/rules/deterministic/[id]/+page.svelte (and
      /new). Form fields: name, description, scope dropdown, scope_ref
      (conditional), enabled switch, safe_mode dropdown, action_type dropdown,
      action_parameters (dynamic based on action_type). Include condition
      builder component."
  - title: Build condition builder component
    done: false
    description: "Create web/src/lib/components/ConditionBuilder.svelte. Top toggle:
      Match ALL/ANY conditions. List of condition rows with: type dropdown
      (sender_email, sender_domain, subject_contains, subject_regex,
      header_match, label_present), value input(s). Add/remove condition
      buttons. Output conditions_json."
  - title: Build LLM rule form
    done: false
    description: "Create web/src/routes/rules/llm/[id]/+page.svelte (and /new). Form
      fields: name, description, scope dropdown, scope_ref (conditional),
      enabled switch, rule_text textarea (large, with placeholder example). Save
      and Cancel buttons."
tags:
  - backend
  - frontend
  - rules
---

Complete rules management feature spanning Rust API and SvelteKit UI:

**Rust API Endpoints:**
- GET /api/rules/deterministic - List all deterministic rules
- GET /api/rules/llm - List all LLM rules
- POST /api/rules/deterministic - Create deterministic rule
- PATCH /api/rules/deterministic/{id} - Update deterministic rule
- POST /api/rules/llm - Create LLM rule
- PATCH /api/rules/llm/{id} - Update LLM rule

**SvelteKit Pages:**
- /rules - Tabbed view (Deterministic | LLM rules) showing name, scope, enabled state, conditions summary
- /rules/deterministic/new and /rules/deterministic/[id] - Form with condition builder and action selector
- /rules/llm/new and /rules/llm/[id] - Form with rule_text textarea

**Features:**
- Enable/disable toggle per rule
- Priority reordering via up/down arrow buttons
- Basic condition builder for deterministic rules:
  - Top-level AND/OR toggle ("Match ALL conditions" vs "Match ANY condition")
  - Flat list of leaf conditions (no nested groups)
  - Condition types: sender email, sender domain, subject contains, subject regex, header match, label present
  - Add/remove condition buttons
- Validation before save
