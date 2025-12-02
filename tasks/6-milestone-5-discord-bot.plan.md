---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Milestone 5: Discord Bot"
goal: Implement Discord bot for action logging and approval workflow
id: 6
uuid: 70ff7f0a-6830-49c0-91d1-e7ed93e09bbc
generatedBy: agent
status: pending
priority: high
container: false
temp: false
dependencies:
  - 5
parent: 1
issue: []
docs:
  - docs/discord.md
  - docs/data_model.md
planGeneratedAt: 2025-11-29T01:23:12.400Z
promptsGeneratedAt: 2025-11-29T01:23:12.400Z
createdAt: 2025-11-29T01:21:26.968Z
updatedAt: 2025-11-29T01:23:12.400Z
tasks:
  - title: Set up Discord bot client
    done: false
    description: Initialize Discord client using serenity or poise crate. Configure
      bot token from config. Connect to configured channel.
  - title: Implement action logging embeds
    done: false
    description: "Create rich embeds for all actions: subject, sender, snippet,
      action type, confidence, rationale, status (auto-executed/pending). Post
      on action queue/complete."
  - title: Build approval.notify job handler
    done: false
    description: Create embed for dangerous actions with Approve/Reject buttons.
      Include action details, confidence, rationale. Post to Discord channel.
  - title: Implement button interaction handler
    done: false
    description: Listen for button clicks. Verify user in whitelist. Dispatch to
      approve/reject/undo handlers based on custom_id.
  - title: Create approve handler
    done: false
    description: "On Approve click: verify whitelist, mark action status='approved',
      enqueue action.gmail job, update Discord message with approval status."
  - title: Create reject handler
    done: false
    description: "On Reject click: verify whitelist, mark action status='rejected',
      update Discord message with rejection status. No further jobs enqueued."
  - title: Create undo button handler
    done: false
    description: "On Undo click: verify whitelist, enqueue undo job, update Discord
      message with undo pending status."
  - title: Implement signed payloads
    done: false
    description: Sign button custom_ids with HMAC including action_id and expiry
      timestamp. Verify signature on interaction. Reject expired or invalid
      signatures.
  - title: Manage discord_whitelist
    done: false
    description: API endpoints to add/remove users from whitelist. Store Discord
      user_id and username. Check whitelist on all approval interactions.
  - title: Add trace ID to Discord messages
    done: false
    description: Include OpenTelemetry trace ID in embed footer. Enable correlation
      between Discord messages and backend traces.
tags:
  - approvals
  - discord
  - rust
---

Discord integration for approvals and logging:
- Discord bot client setup (serenity or similar)
- Action logging embeds for all actions
- approval.notify job handler
- Interactive approval embeds with Approve/Reject/Undo buttons
- Button interaction handler with whitelist verification
- Signed payload validation with expiry
- discord_whitelist table management
- Trace ID correlation in Discord messages

## Future Enhancement: Safer Action Alternatives

When presenting dangerous actions for approval, consider offering safer alternatives as additional options. For example:

- **Delete** action could show: `[Approve Delete] [Archive Instead] [Reject]`
- **Forward** action could show: `[Approve Forward] [Reject]`

This would require:
1. Metadata on ActionType indicating a safer fallback action (e.g., Delete → Archive)
2. Additional button in the approval embed for "Do safer action instead"
3. Handler that creates a new action with the safer alternative

This keeps the safety enforcement layer (Plan 17) simple - it only gates with `needs_approval = true` - while putting the downgrade decision in the user's hands at approval time.
