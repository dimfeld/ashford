---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Gmail Actions: Snooze"
goal: Implement snooze action with configurable label and scheduled restore job
id: 23
uuid: 7300ce7b-c38b-4fe6-ae96-ead50a3f1f05
generatedBy: agent
status: pending
priority: medium
container: false
temp: false
dependencies:
  - 22
parent: 5
references: {}
issue: []
pullRequest: []
docs:
  - docs/gmail_integration.md
planGeneratedAt: 2025-12-03T02:21:58.926Z
promptsGeneratedAt: 2025-12-03T02:21:58.926Z
createdAt: 2025-12-03T02:21:14.729Z
updatedAt: 2025-12-03T02:21:58.926Z
progressNotes: []
tasks:
  - title: Add snooze configuration
    done: false
    description: "Add snooze_label field to config (default: 'Ashford/Snoozed'). Add
      to Config struct in config.rs. Ensure label is created on first use if it
      doesn't exist."
  - title: Create unsnooze job type
    done: false
    description: "Create new job type 'unsnooze.gmail' with handler in
      jobs/unsnooze_gmail.rs. Payload: {account_id, message_id, action_id}.
      Export JOB_TYPE_UNSNOOZE_GMAIL constant."
  - title: Implement snooze action in action_gmail
    done: false
    description: "Implement snooze: remove INBOX label, add snooze label. Extract
      snooze_until from parameters_json (datetime or duration). Enqueue unsnooze
      job with not_before set to snooze_until. Store original labels in
      undo_hint."
  - title: Implement unsnooze job handler
    done: false
    description: "Implement handle_unsnooze_gmail: add INBOX label, remove snooze
      label (preserve other labels). Update original action's undo_hint to
      reflect completion. Handle edge cases: message deleted, label already
      removed."
  - title: Add snooze parameter validation
    done: false
    description: "Validate snooze parameters: snooze_until must be in the future,
      reasonable maximum duration (e.g., 1 year). Return clear error messages
      for invalid parameters."
  - title: Add tests for snooze functionality
    done: false
    description: "Add unit tests for snooze action and unsnooze job handler. Test:
      successful snooze/unsnooze cycle, edge cases (message deleted while
      snoozed, invalid duration), job scheduling with correct not_before."
tags:
  - actions
  - gmail
  - rust
---

Implement Gmail snooze functionality:

## Scope
- Add snooze label configuration (configurable with default "Ashford/Snoozed")
- Implement snooze action: archive message + apply snooze label
- Create unsnooze job type for scheduled restoration
- Unsnooze behavior: add INBOX label, remove snooze label, preserve other labels

## Out of Scope
- Snooze management UI (future milestone)

## Design Notes
- Gmail doesn't have native snooze, so we implement via labels + scheduled jobs
- Snooze parameters should include: duration or target datetime
- Need to handle edge cases: message deleted while snoozed, label removed externally
