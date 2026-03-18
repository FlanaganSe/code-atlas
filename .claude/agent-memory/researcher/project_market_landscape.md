---
name: market_landscape_code_visualization
description: Code/architecture visualization tool market landscape as of March 2026 — what exists, what's dead, key gaps, and recommended product angle
type: project
---

Market landscape research completed March 2026 for code architecture visualization desktop app.

**Why:** User asked for a full competitive scan before deciding on product direction.

**Key facts:**
- CodeSee was acquired by GitKraken (May 2024) after near-shutdown; its visualization product is not yet shipping under GitKraken
- Sourcetrail is archived (2021) but has active community forks; misses modern languages (no Rust, no TS)
- SciTools Understand is the most complete tool but enterprise/legacy-market only, opaque pricing
- dependency-cruiser and Madge are both CLI-only, JS/TS only, no interactive UI
- Nx Graph / Turborepo Graph are locked to their respective build systems
- CodeViz (YC S24) is the closest new entrant: VS Code extension with AI-generated call graphs + C4 diagrams, $19/month; hard blockers = code leaves machine (enterprise dealbreaker) + web-dev-biased categorization + free tier too limited
- No tool does: hierarchical zoom (system → module → file → function), always-current (file-watch), local-only, multi-language — all four together

**The highest-value gap:** Local-first desktop app with hierarchically-zoomable nested dependency graph that stays current via file-watching. No existing tool delivers this.

**Recommended approach:** tree-sitter as the language-agnostic parsing layer (used by GitHub/Neovim/Zed), with Rust + TypeScript as the launch languages. Ship expand/collapse hierarchical nesting with file-watch update first — that single feature is the differentiator.

**How to apply:** When the user asks about product direction, feature priorities, or competitive positioning, reference this landscape. The "local-first + always-current + hierarchical zoom" framing is the core value proposition.
