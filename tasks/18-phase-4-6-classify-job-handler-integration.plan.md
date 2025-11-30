---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Phase 4.6: Classify Job Handler & Integration"
goal: Create the classify job handler that orchestrates the full decision
  pipeline and integrate with existing job system
id: 18
uuid: 9def82bc-4c74-4945-882a-81a674f25cf1
status: pending
priority: high
container: false
temp: false
dependencies:
  - 14
  - 16
  - 17
parent: 4
issue: []
docs:
  - docs/rules_engine.md
  - docs/decision_engine.md
  - docs/job_queue.md
createdAt: 2025-11-30T01:14:19.538Z
updatedAt: 2025-11-30T01:14:19.538Z
tasks: []
tags: []
---

Final integration layer that wires together all components into the classify job and connects to the existing ingestion pipeline.

## Key Components

### Classify Job Handler
**Job type**: `classify`
**Payload**:
```json
{
  "account_id": "string",
  "message_id": "string"
}
```

### Orchestration Flow
```rust
pub async fn handle_classify(
    dispatcher: &JobDispatcher,
    job: Job,
) -> Result<(), JobError> {
    // 1. Parse payload
    let payload: ClassifyPayload = ...;
    
    // 2. Load message and account
    let message = message_repo.get_by_id(...).await?;
    let account = account_repo.get_by_id(...).await?;
    
    // 3. Try deterministic rules (fast path)
    let deterministic_result = rule_engine
        .evaluate_deterministic(&message)
        .await?;
    
    if let Some(matched_rules) = deterministic_result {
        // Create decision from deterministic rules
        // Apply safety gating
        // Persist decision
        // Enqueue action job
        return Ok(());
    }
    
    // 4. LLM path (slow path)
    // Load directions and LLM rules
    let directions = directions_repo.get_all_enabled().await?;
    let llm_rules = llm_rules_repo.get_by_scope(&message).await?;
    
    // 5. Build prompt and call LLM
    let prompt = prompt_builder.build(&message, &directions, &llm_rules);
    let llm_response = llm_provider.complete(prompt).await?;
    
    // 6. Parse decision
    let mut decision = parse_decision(llm_response)?;
    
    // 7. Apply safety enforcement
    let safety_result = safety_enforcer.enforce(&mut decision);
    
    // 8. Persist decision
    let decision_id = decision_repo.create(&decision).await?;
    
    // 9. Enqueue next job
    if decision.needs_approval {
        // Enqueue approval request job
        queue.enqueue("approval.request", ...).await?;
    } else {
        // Enqueue action execution job
        queue.enqueue("action.execute", ...).await?;
    }
    
    Ok(())
}
```

### Error Handling
- Message not found → `JobError::Fatal`
- Account not found → `JobError::Fatal`
- LLM provider error → Map per provider (retryable vs fatal)
- Decision parse error → `JobError::Fatal` (log for debugging)
- Database error → `JobError::Retryable`

### Integration with Ingest Job
Wire classify job into `handle_ingest_gmail`:
```rust
// After message is persisted...
let idempotency_key = format!("gmail:{}:{}:classify", account_id, message_id);
queue.enqueue(
    "classify",
    json!({ "account_id": account_id, "message_id": message_id }),
    Some(idempotency_key),
    0,  // priority
).await?;
```

### JobDispatcher Update
Add to dispatcher match:
```rust
match job.job_type.as_str() {
    "ingest.gmail" => handle_ingest_gmail(self, job).await,
    "history.sync.gmail" => handle_history_sync_gmail(self, job).await,
    "backfill.gmail" => handle_backfill_gmail(self, job).await,
    "classify" => handle_classify(self, job).await,  // NEW
    _ => Err(JobError::Fatal(...)),
}
```

### Dependency Injection
Ensure JobDispatcher has access to:
- LLM provider instance
- Rule engine components
- Safety enforcer

### File Organization
```
ashford-core/src/jobs/
├── classify.rs      # Classify job handler
├── mod.rs           # Update dispatcher
```

### Testing
- End-to-end classify flow with deterministic match
- End-to-end classify flow with LLM path
- Safety override application in context
- Error handling for various failure modes
- Idempotency key prevents duplicate classification

### Performance Considerations
- LLM latency tracking in telemetry
- Database query optimization (indices already exist)
- Consider batching for high-volume scenarios (future enhancement)
