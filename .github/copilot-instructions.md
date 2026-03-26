# Copilot Instructions for agent-switch

Follow the rules in `AGENTS.md` at the repository root. Key points:

- `AGENTS.md` is the single authoritative index for this repo; shared-baseline chapters live in `ai-rules/*.md` and are linked from `AGENTS.md`.
- When adding or renaming files under `ai-rules/`, update the baseline chapter index in `AGENTS.md` in the same edit session.
- Commits follow Conventional Commits; prefer one Minimum Viable Change per commit when automating (see `ai-rules/commit-each-minimum-viable-change.md`).
- Build / verify commands are defined under **Repository Guidelines** in `AGENTS.md` (npm + Rust paths as applicable).
