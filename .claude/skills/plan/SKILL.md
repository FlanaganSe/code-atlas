---
name: plan
description: Create an implementation plan from a PRD or research doc.
user_invocable: true
---

# Plan Skill

Create an implementation plan in `.claude/plans/plan-{name}.md`:

1. Read the relevant PRD and/or research doc
2. Break down into milestones (each independently shippable)
3. For each milestone, list:
   - **Files to create/modify** (with paths)
   - **Key changes** (what, not how — leave implementation to the milestone)
   - **Tests needed**
   - **Acceptance criteria**
4. Identify risks and dependencies between milestones

Use `$ARGUMENTS` as the plan name/scope. If empty, look for recent PRDs.
