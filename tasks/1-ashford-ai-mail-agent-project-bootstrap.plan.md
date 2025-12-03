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
references:
  "2": 85389e56-6e82-4b14-b6ab-153a10439a6e
  "3": b93a0b33-fccb-4f57-8c97-002039917c44
  "4": 5cf4cc37-3eb8-4f89-adae-421a751d13a1
  "5": 66785b19-e85d-4135-bbca-9d061a0394c7
  "6": 70ff7f0a-6830-49c0-91d1-e7ed93e09bbc
  "7": 5a952985-9ed4-4035-8fef-479f3f7e2010
  "8": cc1de1ae-c5ce-43bb-8529-105936dcb034
  "9": 62073031-b34a-4c7d-bfcf-d28c4f1695e7
issue: []
pullRequest: []
docs:
  - docs/overview.md
  - docs/data_model.md
  - docs/configuration.md
createdAt: 2025-11-29T01:20:48.558Z
updatedAt: 2025-11-29T01:38:01.973Z
progressNotes: []
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
