---
name: review
description: Review recent changes for correctness, security, and quality.
user_invocable: true
---

# Review Skill

Review code changes using the reviewer agent pattern:

1. Identify what changed (git diff, or specific files from `$ARGUMENTS`)
2. Review each change for: correctness, edge cases, security, performance
3. Output a structured review with severity levels

If `$ARGUMENTS` specifies files or a commit range, scope the review to that. Otherwise, review unstaged + staged changes.
