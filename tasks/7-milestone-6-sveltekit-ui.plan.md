---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Milestone 6: SvelteKit UI"
goal: Build SvelteKit web application with actions history, rules management,
  and settings pages
id: 7
uuid: 5a952985-9ed4-4035-8fef-479f3f7e2010
generatedBy: agent
status: pending
priority: high
container: false
temp: false
dependencies:
  - 6
parent: 1
references:
  "1": 076d03b1-833c-4982-b0ca-1d8868d40e31
  "6": 70ff7f0a-6830-49c0-91d1-e7ed93e09bbc
issue: []
pullRequest: []
docs:
  - docs/web_ui.md
planGeneratedAt: 2025-11-29T01:23:12.568Z
promptsGeneratedAt: 2025-11-29T01:23:12.568Z
createdAt: 2025-11-29T01:21:27.056Z
updatedAt: 2025-11-29T01:23:12.568Z
progressNotes: []
tasks:
  - title: Initialize SvelteKit project
    done: false
    description: Create SvelteKit project with Bun. Configure TypeScript, Valibot
      for validation, Tailwind CSS for styling. Set up project structure.
  - title: Create Rust API client
    done: false
    description: Build typed HTTP client for Rust backend at localhost:17801. Create
      fetch wrapper with error handling. Define TypeScript types matching Rust
      API responses.
  - title: Implement remote functions pattern
    done: false
    description: Set up query() and command() helpers in *.remote.ts files.
      Configure SvelteKit for transparent client-server RPC.
  - title: Build actions list page
    done: false
    description: "Create /actions route with filterable table: time window, account,
      sender, action type, status, confidence. Paginated results with periodic
      polling refresh."
  - title: Add action filters UI
    done: false
    description: "Filter controls: date range picker, account dropdown,
      sender/domain search, action type multi-select, status checkboxes,
      confidence slider."
  - title: Build action detail page
    done: false
    description: "Create /actions/[id] route showing: decision JSON, rationale,
      before/after state, approval status, trace link, Gmail link. Add undo
      button."
  - title: Implement undo action
    done: false
    description: Undo button calls undoAction command. Show loading state, handle
      errors, refresh action detail on completion.
  - title: Build rules list page
    done: false
    description: Create /rules route with tabs for Deterministic and LLM rules. Show
      name, scope, enabled state, conditions summary. Enable/disable toggle.
  - title: Add rule reordering
    done: false
    description: Drag-and-drop or up/down buttons to change rule priority. Persist
      new priorities via PATCH endpoint.
  - title: Create rule edit forms
    done: false
    description: Forms for creating/editing deterministic rules (conditions builder,
      action selector) and LLM rules (rule_text textarea). Validate before save.
  - title: Build settings page
    done: false
    description: "Create /settings route showing read-only configuration: accounts,
      model settings, Discord channel, Gmail config. Redact secrets."
  - title: Add layout and navigation
    done: false
    description: "Create app shell with sidebar navigation: Actions, Rules, Rules
      Assistant, Settings. Add breadcrumbs, page titles."
tags:
  - frontend
  - sveltekit
  - ui
---

SvelteKit web application:
- Project setup with Bun
- Remote functions pattern for Rust API communication
- Actions history page (/actions) with filters
- Action detail page (/actions/:id) with undo support
- Rules list page (/rules) with tabbed deterministic/LLM rules
- Rule create/edit forms
- Settings page (/settings) with redacted secrets view
- Periodic polling for updates
