---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Gmail Snooze Management: Reschedule and Cancel"
goal: Allow users to change the unsnooze time or immediately unsnooze a snoozed
  email by canceling/rescheduling the pending unsnooze job
id: 26
uuid: f66cf496-965b-4654-9fec-81593d25e48d
generatedBy: agent
status: pending
priority: maybe
container: false
temp: false
dependencies:
  - 23
parent: 5
references:
  "5": 66785b19-e85d-4135-bbca-9d061a0394c7
  "23": 7300ce7b-c38b-4fe6-ae96-ead50a3f1f05
issue: []
pullRequest: []
docs: []
planGeneratedAt: 2025-12-04T10:11:32.103Z
promptsGeneratedAt: 2025-12-04T10:11:32.103Z
createdAt: 2025-12-04T10:11:21.639Z
updatedAt: 2025-12-04T10:11:32.104Z
progressNotes: []
tasks:
  - title: Add reschedule_job method to JobQueue
    done: false
    description: "Add reschedule_job(job_id, new_not_before: DateTime<Utc>) method
      to JobQueue. Updates not_before for a queued job. Returns error if job is
      not in 'queued' state."
  - title: Add helper to find pending unsnooze job for message
    done: false
    description: "Create helper function that takes message_id and returns the
      pending unsnooze job if one exists. Flow: query snooze actions for message
      → get most recent completed → extract unsnooze_job_id from undo_hint →
      fetch job and verify state is 'queued'."
  - title: Implement change_snooze_time action
    done: false
    description: "New action type that finds the pending unsnooze job for a message
      and reschedules it to the new time. Parameters: {message_id, new_time}
      where new_time uses same format as snooze (until or amount+units)."
  - title: Implement unsnooze_now action
    done: false
    description: "New action type that immediately unsnoozes a message: cancel
      pending unsnooze job, add INBOX label, remove snooze label. Parameters:
      {message_id}. Handle case where job already ran (just restore labels)."
  - title: Add tests for snooze management
    done: false
    description: "Tests for: reschedule_job method, change_snooze_time action,
      unsnooze_now action, edge cases (job already ran, message deleted, no
      pending snooze)."
tags:
  - actions
  - gmail
  - rust
---

Enable management of snoozed emails after the initial snooze action.

## Scope
- Add reschedule_job method to JobQueue for changing not_before time
- Implement "change snooze time" action that finds and reschedules the pending unsnooze job
- Implement "unsnooze now" action that cancels the pending job and immediately restores to inbox
- Query flow: find snooze action by message_id → get unsnooze job ID from undo_hint → cancel/reschedule

## Out of Scope
- UI for snooze management (separate frontend work)
- Batch operations on multiple snoozed emails

## Prerequisites from Plan 23
- Snooze action stores unsnooze job ID in undo_hint_json
- ActionRepository::list_by_message_id can find snooze actions
- JobQueue::cancel already exists

## Design Notes
- To find the unsnooze job for a message:
  1. Query actions by message_id and action_type='snooze' and status='completed'
  2. Get most recent one (by executed_at)
  3. Extract unsnooze_job_id from undo_hint_json
  4. Check job state - if still 'queued', can cancel/reschedule
- "Unsnooze now" should: cancel job + add INBOX label + remove snooze label
- "Change snooze time" should: update job's not_before to new time
- Edge case: if unsnooze job already ran, these operations are no-ops
