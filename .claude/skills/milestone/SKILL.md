---
name: milestone
description: Implement a single milestone from a plan.
user_invocable: true
---

# Milestone Skill

Implement one milestone from an existing plan:

1. Read the plan file to find the target milestone
2. Implement the changes described in the milestone
3. Write/update tests as specified
4. Run verification (tests, lint, typecheck)
5. Update the plan to mark the milestone complete

Use `$ARGUMENTS` to specify which plan and milestone number. If empty, find the most recent plan and its next incomplete milestone.

After implementation, suggest running `/verify` to confirm everything passes.
