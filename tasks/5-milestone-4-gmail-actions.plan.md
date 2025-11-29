---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Milestone 4: Gmail Actions"
goal: Implement Gmail action execution with undo support
id: 5
uuid: 66785b19-e85d-4135-bbca-9d061a0394c7
generatedBy: agent
status: pending
priority: high
container: false
temp: false
dependencies:
  - 4
parent: 1
issue: []
docs:
  - docs/gmail_integration.md
  - docs/data_model.md
planGeneratedAt: 2025-11-29T01:23:12.234Z
promptsGeneratedAt: 2025-11-29T01:23:12.234Z
createdAt: 2025-11-29T01:21:26.875Z
updatedAt: 2025-11-29T01:23:12.234Z
tasks:
  - title: Define action types enum
    done: false
    description: "Create ActionType enum: archive, apply_label, remove_label,
      mark_read, mark_unread, delete, trash, restore, star, unstar, snooze,
      forward, auto_reply."
  - title: Implement archive action
    done: false
    description: Gmail API call to remove INBOX label. Store undo_hint with original
      labels. Mark action completed.
  - title: Implement label actions
    done: false
    description: "apply_label: add label (create if needed). remove_label: remove
      label. Store original label state in undo_hint."
  - title: Implement read state actions
    done: false
    description: "mark_read: remove UNREAD label. mark_unread: add UNREAD label.
      Track original state for undo."
  - title: Implement delete/trash actions
    done: false
    description: "trash: move to trash. delete: permanent delete (dangerous,
      requires approval). Store pre-delete state for trash undo."
  - title: Implement star actions
    done: false
    description: "star: add STARRED label. unstar: remove STARRED label. Simple
      toggle with undo."
  - title: Implement snooze action
    done: false
    description: Move to snooze folder (configurable), schedule restore job for
      snooze end time. Store original location for undo.
  - title: Implement forward action
    done: false
    description: Create outbound.send job with forward parameters. Include original
      message as attachment or inline. Store in actions table.
  - title: Implement auto_reply action
    done: false
    description: Create outbound.send job with reply parameters. Generate reply
      using template or LLM. Track reply in actions table.
  - title: Build outbound.send job handler
    done: false
    description: Send email via Gmail API (messages.send). Handle drafts,
      attachments, threading (In-Reply-To, References headers).
  - title: Create undo job handler
    done: false
    description: Load original action, derive inverse action from undo_hint_json,
      execute inverse, create action_link with relation_type='undo_of'.
  - title: Implement action.gmail job handler
    done: false
    description: Route to appropriate action implementation based on action_type.
      Update action status, capture errors, record executed_at.
  - title: Capture pre-images for undo
    done: false
    description: Before executing action, fetch current message state (labels,
      folder, read status). Store in undo_hint_json for reliable undo.
tags:
  - actions
  - gmail
  - rust
---

Gmail action execution:
- action.gmail job handler
- Action types: archive, apply_label, remove_label, mark_read, mark_unread, delete, trash, star, unstar, snooze
- Forward and auto-reply actions (outbound.send job)
- Undo job handler with inverse action derivation
- undo_hint_json storage for reversibility
- actions and action_links tables
- Pre-image capture for undo operations
