# ADR Process

This project uses Architecture Decision Records (ADRs) to capture significant technical and architectural decisions.

## When to Write an ADR

Create a new ADR when you:

- Introduce a new subsystem or major dependency
- Change core architectural patterns
- Make a trade-off that impacts long-term maintenance
- Reject a significant alternative

## How to Create an ADR

1. Copy the template:

   ```bash
   cp docs/ADR/template.md docs/ADR/NNNN-title.md
   ```

2. Choose the next sequential number (`NNNN`), and use a short, hyphenated title.

3. Fill in the template sections:

   - **Status** (Proposed/Accepted/Deprecated/Superseded)
   - **Context**
   - **Decision**
   - **Consequences**

4. Add any relevant diagrams or links.

5. Reference the ADR in related code changes or documentation.

## Updating Existing ADRs

- If a decision changes, create a new ADR and mark the old one as **Superseded**.
- Never rewrite history; keep the original record for traceability.

## Review Expectations

- ADRs should be concise and focused.
- Tie the decision to measurable requirements when possible.
- Include at least one alternative that was considered.
