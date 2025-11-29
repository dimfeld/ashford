
- **Resource attributes**:

    - service.name=ai-mail-agent-rust

    - service.version

    - deployment.environment

    - host.arch, os.type
- **Spans**:

    - email.receive

    - email.classify

    - email.action

    - email.approval

    - queue.job
- **Propagation**:

    - Rust service attaches trace IDs to log lines.

    - SvelteKit can accept trace IDs from Rust responses for correlation (optional but nice).

