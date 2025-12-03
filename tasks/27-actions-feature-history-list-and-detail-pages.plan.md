---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Actions Feature: History List and Detail Pages"
goal: Build the actions history page with filters and action detail page with
  undo support, including all required Rust API endpoints
id: 27
uuid: 18da2db8-7523-467d-9f54-5f8a73896df7
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
planGeneratedAt: 2025-12-03T10:13:27.735Z
promptsGeneratedAt: 2025-12-03T10:13:27.735Z
createdAt: 2025-12-03T09:46:54.701Z
updatedAt: 2025-12-03T10:13:27.735Z
progressNotes: []
tasks:
  - title: Create actions API module in Rust
    done: false
    description: Create server/crates/ashford-server/src/api/mod.rs and
      server/crates/ashford-server/src/api/actions.rs. Set up module structure
      and add /api routes to the main router.
  - title: Implement GET /api/actions endpoint
    done: false
    description: "Create list_actions handler with query params: time_window,
      account_id, sender, action_type, status, min_confidence, max_confidence,
      limit, offset. Use ActionRepository methods. Return
      PaginatedResponse<Action>."
  - title: Implement GET /api/actions/{id} endpoint
    done: false
    description: "Create get_action handler that returns action with joined decision
      data, message subject/sender, and undo_hint. Include computed fields:
      can_undo, gmail_link."
  - title: Implement POST /api/actions/{id}/undo endpoint
    done: false
    description: Create undo_action handler that validates action can be undone,
      creates undo job in queue, and returns job ID. Handle errors if action
      already undone or not undoable.
  - title: Create actions remote functions
    done: false
    description: Create web/src/lib/api/actions.remote.ts with getActions query
      (with filter params), getAction query (by ID), and undoAction command. Use
      generated TS types.
  - title: Build actions list page
    done: false
    description: "Create web/src/routes/actions/+page.svelte with table showing:
      timestamp, subject, sender, action type, confidence (as percentage),
      status badge. Use shadcn Table component. Link rows to detail page."
  - title: Add action filters UI
    done: false
    description: "Add filter controls above table: date range picker (or preset
      buttons: 24h, 7d, 30d), account dropdown, sender search input, action type
      multi-select, status checkboxes, confidence range slider. Sync filters to
      URL search params."
  - title: Add pagination controls
    done: false
    description: Add pagination below table using shadcn Pagination component. Show
      total count, current page, items per page selector (10/25/50). Handle page
      changes via URL params.
  - title: Implement periodic polling
    done: false
    description: Add $effect that refreshes action list every 10 seconds using
      getActions.refresh(). Pause polling when tab is not visible. Show subtle
      refresh indicator.
  - title: Build action detail page
    done: false
    description: "Create web/src/routes/actions/[id]/+page.svelte showing: action
      type badge, status, confidence, timestamp, message subject and sender,
      rationale text, decision JSON (collapsible), before/after state diff,
      Gmail link button, trace link if available."
  - title: Implement undo button
    done: false
    description: Add Undo Action button to detail page (shown only if can_undo is
      true). Show loading spinner during request. Display success toast and
      refresh data on completion. Show error toast on failure.
tags:
  - actions
  - backend
  - frontend
---

Complete actions feature spanning Rust API and SvelteKit UI:

**Rust API Endpoints:**
- GET /api/actions - List actions with filters (timeWindow, account, sender, actionType, status, confidence range, pagination)
- GET /api/actions/{id} - Get action detail with decision, message info, and undo hint
- POST /api/actions/{id}/undo - Enqueue undo job

**SvelteKit Pages:**
- /actions - Filterable table with columns: timestamp, subject, sender, action type, confidence, status
- /actions/[id] - Detail view showing decision JSON, rationale, before/after state, Gmail link, undo button

**Features:**
- Pagination for large result sets
- Periodic polling for updates (10s interval)
- Filter persistence in URL params
- Loading and error states
