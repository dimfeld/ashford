### **7.1 Capabilities**

- **Channel**: Single configured channel.
- **Whitelist**: discord_whitelist table with allowed usernames or IDs.
- **Messages**:

    - For **every action** (queued or completed):

        - Post or update an embed summarizing:

            - Subject, sender, snippet.

            - Derived action(s), confidence, rationale.

            - Whether it is auto-executed or pending approval.

    - For **dangerous actions requiring approval**:

        - Embed with buttons:

            - Approve.

            - Reject.

            - Open in Web.

  

### **7.2 Interaction Handling**

- On **Approve**:

    - Verify user in whitelist.

    - Mark action status='approved'.

    - Enqueue corresponding action.gmail (or outbound.send) job.
- On **Reject**:

    - Mark action status='rejected'.

    - No further jobs for that action.
- On **Undo** (if exposed as button or command):

    - Enqueue undo job using action_links mapping.

  

### **7.3 Security**

- All interactive Discord components include:

    - Signed payload with action_id and expiry.
- Only whitelisted users may approve/reject/undo.
- All decisions logged and correlated with OpenTelemetry trace IDs.

