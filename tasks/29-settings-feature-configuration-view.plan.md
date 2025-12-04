---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Settings Feature: Configuration View"
goal: Build the read-only settings page displaying system configuration with
  redacted secrets, including the Rust API endpoint
id: 29
uuid: 1a24434a-f5be-45ff-97bd-d50026e0869e
generatedBy: agent
status: pending
priority: medium
container: false
temp: false
dependencies:
  - 30
parent: 7
references:
  "7": 5a952985-9ed4-4035-8fef-479f3f7e2010
  "30": 64c00252-4c84-4b02-8fc2-68559edf27a9
issue: []
pullRequest: []
docs:
  - docs/web_ui.md
planGeneratedAt: 2025-12-03T10:13:27.977Z
promptsGeneratedAt: 2025-12-03T10:13:27.977Z
createdAt: 2025-12-03T09:46:54.862Z
updatedAt: 2025-12-03T10:13:27.977Z
progressNotes: []
tasks:
  - title: Create settings API endpoint in Rust
    done: false
    description: "Create server/crates/ashford-server/src/api/settings.rs with GET
      /api/settings handler. Read config and return sanitized version with
      secrets redacted (OAuth tokens, API keys shown as '••••••••'). Include:
      accounts (email, display_name), model config, policy config, Discord
      config."
  - title: Create settings TS types
    done: false
    description: "Add #[derive(TS)] to a new SettingsResponse struct that contains
      the sanitized config shape. Export to generated types."
  - title: Create settings remote function
    done: false
    description: Create web/src/lib/api/settings.remote.ts with getSettings query
      that fetches /api/settings.
  - title: Build settings page
    done: false
    description: "Create web/src/routes/settings/+page.svelte with card-based
      layout. Sections: Accounts (list email accounts with sync status), Model
      Settings (model name, confidence thresholds), Policy (approval_always
      actions, confidence_default), Discord (channel info if configured). Use
      shadcn Card components. All read-only with redacted secrets displayed."
tags:
  - backend
  - frontend
  - settings
---

Settings display feature spanning Rust API and SvelteKit UI:

**Rust API Endpoint:**
- GET /api/settings - Return sanitized configuration (secrets redacted)

**SvelteKit Page:**
- /settings - Read-only display of:
  - Configured email accounts
  - Model selection and confidence thresholds
  - Discord channel configuration
  - Gmail configuration
  - Policy settings (approval_always list, confidence_default)

**Features:**
- Secrets shown as redacted (e.g., "••••••••")
- Clean card-based layout for each config section
- No edit functionality (read-only for now)
