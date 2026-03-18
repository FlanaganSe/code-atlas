---
name: prd
description: Create a Product Requirements Document for a feature or initiative.
user_invocable: true
---

# PRD Skill

Create a PRD in `.claude/plans/prd-{name}.md` with:

1. **Problem**: What problem are we solving? Who has it?
2. **Goals**: What does success look like? (measurable)
3. **Non-goals**: What are we explicitly NOT doing?
4. **User stories**: As a [role], I want [thing], so that [benefit]
5. **Requirements**: Functional and non-functional
6. **Open questions**: Things to resolve before implementation

Ask the user what feature/initiative they want to spec. If `$ARGUMENTS` is provided, use that as the starting point.
