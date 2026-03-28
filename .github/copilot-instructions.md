# Copilot Instructions for agent-switch

Follow the rules in `AGENTS.md` at the repository root. Key points:

- `AGENTS.md` is the single authoritative index for this repo; shared-baseline chapters live in `ai-rules/*.md` and are linked from `AGENTS.md`.
- Do not duplicate full rule chapters in this file; keep pointers only (see `ai-rules/shared-rules-entry-and-thin-adapters.md`).
- When adding or renaming files under `ai-rules/`, update the baseline chapter index in `AGENTS.md` and any tool adapters (`CLAUDE.md`, `.cursor/rules/*.mdc`) in the same edit session per `ai-rules/shared-rules-entry-and-thin-adapters.md`.
- Commits follow Conventional Commits; prefer one Minimum Viable Change per commit when automating (see `ai-rules/commit-each-minimum-viable-change.md`). Before starting a new MVC, assess existing dirty tree and unknown changes per `ai-rules/before-new-mvc-assess-existing-dirty-tree.md`.
- Build / verify commands are defined under **Repository Guidelines** in `AGENTS.md` (npm + Rust paths as applicable).
