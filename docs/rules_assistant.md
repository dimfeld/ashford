The rules assistant is a conversational layer that helps the user create and adjust rules.

  

### **4.1 UX Flow**

- User opens /rules/assistant in the SvelteKit UI.
- Types natural-language instructions like:

    - "All receipts from amazon.com → label 'Receipts/Amazon' and archive."

    - "If an email is about payroll or salary, mark as important and keep in the inbox."
- SvelteKit calls a **remote function** that:

    1. Sends user message + current rule set to the Rust backend via REST.

    2. Rust backend uses the LLM to:

        - Parse intent (create/update/delete rules).

        - Propose deterministic and/or LLM rules to satisfy the request.

    3. Rust backend returns:

        - Proposed structured rule objects.

        - A human-readable explanation / diff message.
- UI renders:

    - A preview of the rule(s) to be created/changed.

    - Controls for "Apply changes" or "Discard."
- On apply:

    - SvelteKit calls "apply rules change" endpoint on Rust backend, which persists changes in deterministic_rules / llm_rules tables.

  

### **4.2 Backend Responsibilities**

- Maintain conversation context for the rules assistant (optional, but helpful for multi-step rule refinements).
- Ask LLM to output machine-readable rule definitions matching internal schema.
- Validate:

    - Regex safety.

    - Action validity.

    - Priority/Fallthrough semantics (e.g., does a new rule need to be placed above some existing rules?).

