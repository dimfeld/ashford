---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: Ashford AI Mail Agent - Project Bootstrap
goal: Bootstrap the Ashford AI Mail Agent with Rust backend, libsql queue, Gmail
  integration, Discord bot, and SvelteKit UI
id: 1
uuid: 076d03b1-833c-4982-b0ca-1d8868d40e31
status: in_progress
priority: high
container: true
temp: false
dependencies:
  - 2
  - 3
  - 4
  - 5
  - 6
  - 7
  - 8
  - 9
issue: []
docs:
  - docs/overview.md
  - docs/data_model.md
  - docs/configuration.md
createdAt: 2025-11-29T01:20:48.558Z
updatedAt: 2025-11-29T01:38:01.973Z
tasks: []
tags:
  - ai
  - discord
  - gmail
  - rust
  - sveltekit
---

This is the top-level container for the Ashford AI Mail Agent project.

The project consists of:
- **Rust Agent Service**: Core backend with libsql queue, Gmail integration, LLM classification, rule engine, and Discord bot
- **SvelteKit Web App**: UI for viewing actions, managing rules, and using the rules assistant

See child plans for detailed implementation phases.
