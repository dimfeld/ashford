---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Gmail Actions: Undo System"
goal: Implement undo job handler that derives and executes inverse actions from
  undo_hint_json
id: 31
uuid: cc4bd313-ff47-4c58-87f2-30999a6058e2
generatedBy: agent
status: pending
priority: medium
container: false
temp: false
dependencies: []
parent: 5
references:
  "5": 66785b19-e85d-4135-bbca-9d061a0394c7
issue: []
pullRequest: []
docs:
  - docs/gmail_integration.md
  - docs/data_model.md
planGeneratedAt: 2025-12-04T20:30:11.590Z
promptsGeneratedAt: 2025-12-04T20:30:11.590Z
createdAt: 2025-12-04T20:29:59.381Z
updatedAt: 2025-12-04T20:31:48.510Z
progressNotes: []
tasks:
  - title: Create undo job type
    done: false
    description: "Create new job type 'undo.action' with handler in
      jobs/undo_action.rs. Payload: {account_id, original_action_id}. Export
      JOB_TYPE_UNDO_ACTION constant and register in mod.rs dispatcher."
  - title: Implement inverse action derivation
    done: false
    description: "Create derive_inverse_action(action, undo_hint) function. Map:
      archive→apply_label INBOX, apply_label→remove_label, trash→restore,
      star→unstar, etc. Return error for non-undoable actions (delete, forward,
      auto_reply marked with irreversible=true)."
  - title: Implement undo job handler
    done: false
    description: "Implement handle_undo_action: load original action, validate it's
      undoable (completed status, not irreversible, not already undone). Derive
      inverse action from undo_hint_json. Execute inverse via Gmail API. Create
      new action record for the undo. Create action_link with
      relation_type='undo_of'."
  - title: Implement snooze undo handling
    done: false
    description: "Special case for snooze undo: cancel the scheduled unsnooze.gmail
      job using cancel_unsnooze_job_id from undo_hint. Restore INBOX label and
      remove snooze label. Handle case where unsnooze job already ran."
  - title: Add tests for undo system
    done: false
    description: "Add tests for undo job handler: successful undo of various action
      types (archive, apply_label, star, trash), rejection of non-undoable
      actions (delete, forward, auto_reply), action_link creation verification.
      Test edge cases: action already undone, original action failed, message
      deleted externally."
tags:
  - actions
  - gmail
  - rust
---

Implement the undo system for reversing Gmail actions:

## Scope
- Create undo.action job type and handler
- Load original action and validate it's undoable
- Derive inverse action from undo_hint_json
- Execute inverse action via Gmail API
- Create action_link with relation_type='undo_of'
- Handle non-undoable actions gracefully (delete, forward, auto_reply)

## Design Notes
- Undo hint structure already stored by existing actions contains inverse_action and inverse_parameters
- Some actions are irreversible (delete, forward, auto_reply) - undo handler should reject these
- Snooze undo requires canceling the scheduled unsnooze job
- Check for existing action_links to prevent double-undo

<!-- rmplan-generated-start -->
## Research

### Summary
- The undo system leverages the existing `undo_hint_json` infrastructure already populated by action handlers
- `action_links` table exists with `undo_of` relation type ready for use
- Most actions already store inverse action info in their undo hints
- Special handling needed for snooze (cancel scheduled job) and irreversible actions (delete, forward, auto_reply)

### Findings

#### Undo Hint Structure
Existing actions store undo hints in `undo_hint_json` column with this structure:
```json
{
    "pre_labels": ["INBOX", "UNREAD"],
    "pre_unread": true,
    "pre_starred": false,
    "pre_in_inbox": true,
    "pre_in_trash": false,
    "action": "archive",
    "inverse_action": "apply_label",
    "inverse_parameters": {"label": "INBOX"}
}
```

Irreversible actions use:
```json
{
    "action": "delete",
    "inverse_action": "none",
    "inverse_parameters": {"note": "cannot undo delete - message permanently deleted"},
    "irreversible": true
}
```

#### Action Links Table
**Schema:**
```sql
CREATE TABLE action_links (
  id TEXT PRIMARY KEY,
  cause_action_id TEXT NOT NULL,
  effect_action_id TEXT NOT NULL,
  relation_type TEXT NOT NULL CHECK (
    relation_type IN ('undo_of','approval_for','spawned','related')
  )
);
```

**For undo:** When undoing action A, create action B for the inverse, then link: `{cause: B, effect: A, relation_type: "undo_of"}` meaning "B is the undo of A"

#### Inverse Action Mapping
| Original Action | Inverse Action | Inverse Parameters |
|-----------------|----------------|-------------------|
| archive | apply_label | `{"label": "INBOX"}` |
| apply_label | remove_label | `{"label": "<same>"}` |
| remove_label | apply_label | `{"label": "<same>"}` |
| mark_read | mark_unread | `{}` |
| mark_unread | mark_read | `{}` |
| star | unstar | `{}` |
| unstar | star | `{}` |
| trash | restore | `{}` |
| restore | trash | `{}` |
| snooze | (special) | Cancel job, restore INBOX, remove snooze label |
| delete | ✗ irreversible | |
| forward | ✗ irreversible | |
| auto_reply | ✗ irreversible | |

#### Snooze Undo Special Case
Snooze undo hint contains:
```json
{
    "snooze_until": "2024-12-25T09:00:00Z",
    "snooze_label": "Label_123",
    "unsnooze_job_id": "job-uuid",
    "cancel_unsnooze_job_id": "job-uuid",
    "inverse_parameters": {
        "add_labels": ["INBOX"],
        "remove_labels": ["Label_123"]
    }
}
```

Handler must:
1. Cancel the scheduled `unsnooze.gmail` job via `JobQueue::cancel()`
2. Apply labels: add INBOX, remove snooze label
3. Handle case where unsnooze job already ran (graceful no-op)

#### Key Files
- `server/crates/ashford-core/src/jobs/action_gmail.rs` - Contains `PreImageState`, undo hint building
- `server/crates/ashford-core/src/decisions/repositories.rs` - `ActionRepository`, `ActionLinkRepository`
- `server/crates/ashford-core/src/queue.rs` - `JobQueue::cancel()` method

### Risks & Constraints

1. **Double-Undo Prevention**
   - Check for existing `action_link` with `relation_type = "undo_of"` before processing
   - Return clear error if action already undone

2. **Race Conditions**
   - User may request undo while original action is still executing
   - Only allow undo of actions in `Completed` status

3. **External State Changes**
   - Message may be deleted externally before undo
   - Handle 404 from Gmail API gracefully (action becomes non-undoable)

4. **Snooze Job Timing**
   - Unsnooze job may have already run when user requests undo
   - Need to detect this case and skip job cancellation
<!-- rmplan-generated-end -->
