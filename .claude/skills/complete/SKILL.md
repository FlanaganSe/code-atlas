---
name: complete
description: Finalize a plan — verify, review, and summarize what was done.
user_invocable: true
---

# Complete Skill

Wrap up an implementation plan:

1. Run full verification (`/verify`)
2. Run a review of all changes since the plan started
3. Summarize what was implemented, any deviations from the plan, and known limitations
4. Suggest next steps (follow-up tasks, docs to update, etc.)

Use `$ARGUMENTS` to specify which plan to complete. If empty, find the most recent active plan.
