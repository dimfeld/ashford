---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Gmail Actions: Outbound Email & Undo"
goal: Implement forward/auto-reply actions via outbound.send job and the undo
  job handler
id: 24
uuid: e3c7d618-82e3-4835-9f9c-441d596c2fc1
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
  - docs/data_model.md
planGeneratedAt: 2025-12-03T02:21:59.017Z
promptsGeneratedAt: 2025-12-03T02:21:59.017Z
createdAt: 2025-12-03T02:21:14.787Z
updatedAt: 2025-12-03T02:21:59.017Z
progressNotes: []
tasks:
  - title: Add send_message method to GmailClient
    done: false
    description: Implement GmailClient::send_message(raw_message) calling POST
      /messages/send with base64url-encoded RFC 2822 message. Add
      SendMessageRequest and SendMessageResponse types.
  - title: Create MIME message builder
    done: false
    description: "Create MimeBuilder utility for constructing RFC 2822 email
      messages. Support: To/From/Subject headers, plain text and HTML body,
      In-Reply-To and References headers for threading, attachments."
  - title: Create outbound.send job type
    done: false
    description: "Create new job type 'outbound.send' with handler in
      jobs/outbound_send.rs. Payload: {account_id, action_id, message_type:
      'forward'|'reply', to, subject, body, original_message_id, attachments}.
      Export JOB_TYPE_OUTBOUND_SEND constant."
  - title: Implement forward action
    done: false
    description: "In action_gmail, implement forward: extract recipients and
      optional note from parameters. Enqueue outbound.send job with
      message_type='forward'. Include original message body (inline or as
      attachment based on config)."
  - title: Implement auto_reply action
    done: false
    description: "Implement auto_reply: extract reply content from parameters (may
      be LLM-generated). Enqueue outbound.send job with message_type='reply'.
      Set proper threading headers to keep in same thread."
  - title: Implement outbound.send job handler
    done: false
    description: "Implement handle_outbound_send: build MIME message using
      MimeBuilder, call send_message API. Update action status on completion.
      Store sent message ID in action result for reference."
  - title: Create undo job type
    done: false
    description: "Create new job type 'undo.action' with handler in
      jobs/undo_action.rs. Payload: {account_id, original_action_id}. Export
      JOB_TYPE_UNDO_ACTION constant."
  - title: Implement undo job handler
    done: false
    description: "Implement handle_undo_action: load original action, validate it's
      undoable (completed status, has valid undo_hint). Derive inverse action
      from undo_hint_json. Execute inverse via Gmail API. Create new action
      record for the undo. Create action_link with relation_type='undo_of'."
  - title: Implement inverse action derivation
    done: false
    description: "Create derive_inverse_action(action, undo_hint) function. Map:
      archive→restore labels, apply_label→remove_label, trash→restore,
      star→unstar, etc. Return error for non-undoable actions (delete, forward,
      auto_reply)."
  - title: Add tests for outbound email
    done: false
    description: Add tests for MimeBuilder (headers, body, attachments, threading).
      Add integration tests for outbound.send job with mocked Gmail API. Verify
      correct MIME structure.
  - title: Add tests for undo system
    done: false
    description: "Add tests for undo job handler: successful undo of various action
      types, rejection of non-undoable actions, action_link creation. Test edge
      cases: action already undone, original action failed."
tags:
  - actions
  - gmail
  - rust
---

Implement email sending and undo functionality:

## Scope - Outbound Email
- Add send_message method to GmailClient
- Create outbound.send job handler
- MIME message construction with proper threading headers (In-Reply-To, References)
- Implement forward action (include original as attachment or inline)
- Implement auto_reply action (template or content from decision)

## Scope - Undo System
- Create undo job handler
- Load original action and derive inverse action from undo_hint_json
- Execute inverse action via Gmail API
- Create action_link with relation_type='undo_of'
- Handle non-undoable actions gracefully

## Design Notes
- Forward needs to handle attachments from original message
- Auto-reply content comes from decision parameters (may be LLM-generated)
- Some actions cannot be undone (delete, forward, auto_reply) - undo handler should validate
