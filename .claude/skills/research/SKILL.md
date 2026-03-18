---
name: research
description: Research a topic before planning. Gathers context from codebase and web.
user_invocable: true
---

# Research Skill

Research the given topic using the researcher agent pattern:

1. Search the codebase for relevant code, patterns, and prior art
2. Search the web for best practices, library docs, or solutions
3. Write findings to `.claude/plans/research-{topic}.md`

Structure the output as:
- **Question**: What we're investigating
- **Codebase findings**: What exists today (with file:line refs)
- **External findings**: Best practices, library options, etc.
- **Recommendation**: What approach to take and why

Use `$ARGUMENTS` as the research topic. If empty, ask the user.
