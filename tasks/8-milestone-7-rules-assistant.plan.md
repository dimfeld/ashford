---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Milestone 7: Rules Assistant"
goal: Implement conversational rules assistant with LLM-powered rule creation
id: 8
uuid: cc1de1ae-c5ce-43bb-8529-105936dcb034
generatedBy: agent
status: pending
priority: high
container: false
temp: false
dependencies:
  - 7
parent: 1
references:
  "1": 076d03b1-833c-4982-b0ca-1d8868d40e31
  "7": 5a952985-9ed4-4035-8fef-479f3f7e2010
issue: []
pullRequest: []
docs:
  - docs/rules_assistant.md
  - docs/web_ui.md
  - docs/data_model.md
planGeneratedAt: 2025-11-29T01:23:12.736Z
promptsGeneratedAt: 2025-11-29T01:23:12.736Z
createdAt: 2025-11-29T01:21:27.146Z
updatedAt: 2025-11-29T01:23:12.736Z
progressNotes: []
tasks:
  - title: Design assistant LLM prompt
    done: false
    description: "Create system prompt for rules assistant: explain capabilities,
      output format for rule proposals, how to interpret user intent, schema for
      deterministic vs LLM rules."
  - title: Build assistant chat endpoint
    done: false
    description: "POST /api/rules/assistant/message endpoint: accept message +
      conversationId, call LLM, parse response, return proposed changes +
      human-readable explanation."
  - title: Implement SSE streaming endpoint
    done: false
    description: "POST /api/rules/assistant/stream endpoint: stream LLM tokens via
      SSE. Send token events as they arrive, final event includes proposed
      changes."
  - title: Create conversation storage
    done: false
    description: Store chat history in rules_chat_sessions and rules_chat_messages
      tables. Load history for context on subsequent messages. Support multiple
      sessions.
  - title: Parse rule proposals
    done: false
    description: "Extract structured rule definitions from LLM response. Validate
      against schemas: conditions_json for deterministic, rule_text for LLM
      rules."
  - title: Validate regex safety
    done: false
    description: Check regex patterns for catastrophic backtracking. Reject overly
      complex patterns. Provide user-friendly error messages.
  - title: Validate action parameters
    done: false
    description: Ensure action_type is valid, parameters match expected schema,
      label names are valid, forwarding addresses are reasonable.
  - title: Check priority conflicts
    done: false
    description: Analyze proposed rule priority vs existing rules. Warn if new rule
      would never match due to higher-priority rules. Suggest priority
      adjustments.
  - title: Build apply endpoint
    done: false
    description: "POST /api/rules/assistant/apply endpoint: accept changeIds,
      persist rules to deterministic_rules or llm_rules tables, return
      created/updated rules."
  - title: Create SvelteKit chat UI
    done: false
    description: "Build /rules/assistant page with chat interface: message history,
      input textbox, send button. Scrollable conversation pane."
  - title: Implement SSE client
    done: false
    description: "Client-side SSE handling: open stream, process token events for
      real-time display, handle final event with proposed changes, error
      handling."
  - title: Build proposed changes preview
    done: false
    description: Render proposed rules as human-readable cards. Show 'Show JSON'
      toggle for technical details. Display diff for updates.
  - title: Add Apply/Discard controls
    done: false
    description: Apply button calls apply endpoint, shows success/error feedback.
      Discard clears proposed changes. Navigate to /rules on successful apply.
tags:
  - ai
  - chat
  - llm
  - rust
  - sveltekit
---

Rules assistant for natural language rule creation:
- Backend endpoints for assistant chat
- LLM integration for intent parsing and rule proposal
- Rule schema validation (regex safety, action validity, priority conflicts)
- Conversation context management (rules_chat_sessions, rules_chat_messages tables)
- SvelteKit chat UI (/rules/assistant)
- SSE streaming for real-time LLM responses
- Proposed changes preview with Apply/Discard controls
- Round-trip: message → proposed rules → apply
