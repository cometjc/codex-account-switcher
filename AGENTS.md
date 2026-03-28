# Governance

- 本 `AGENTS.md` 為本 repo 的治理權威入口。
- 與組織 shared baseline 對齊之正式章節見下方 [共同 baseline 章節](#共同-baseline-章節)（`ai-rules/*.md`）；後續同步請依該目錄內之 adoption workflow 執行。

---

### 0. Response Language
- 預設以繁體中文（zhtw）回覆使用者。
- 若使用者在當前對話中明確指定其他語言，才針對該次請求改用指定語言。

---

### 1. Plan Node Default
- Enter plan mode for ANY non-trivial task (3+ steps or architectural decisions)
- If something goes sideways, STOP and re-plan immediately - don't keep pushing
- Use plan mode for verification steps, not just building
- Write detailed specs upfront to reduce ambiguity

---

### 2. Subagent Strategy
- Use subagents liberally to keep main context window clean
- Offload research, exploration, and parallel analysis to subagents
- For complex problems, throw more compute at it via subagents
- One task per subagent for focused execution
- 若使用者輸入 `nlsdd-go`，將其視為固定口令：透過中央 executor 持續推進所有 plans，直到 executor 內的相關 plan 都完成。
- `nlsdd-go` 的正式入口是 repo-local executor（`.nlsdd/executor.sqlite` + `node NLSDD/scripts/nlsdd-executor.cjs`），不是手動掃描 `plan/`、scoreboard markdown 或 lane journal。
- `plan/` 不應保留 live plan；主控端在 `nlsdd-go` 前應先確認 `plan/` 已清空，必要時先用 executor import/cleanup。
- subagent 與主控端之間只允許透過 executor + worktree/result branch 交換狀態；不要再把自由文字、lane markdown、scoreboard row 當成正式交換介面。

---

### 3. Self-Improvement Loop
- After ANY correction from the user: update `tasks/lessons.md` with the pattern
- Write rules for yourself that prevent the same mistake
- Ruthlessly iterate on these lessons until mistake rate drops
- Review lessons at session start for relevant project
- 若此次請求本身是規則新增或規則修正，且正式規則文件已更新並完成驗證，main agent 應預設直接 commit；不要停在「這批還沒 commit」等待額外提醒
- 這條自動 commit 規則適用於 main agent 的本地治理變更，不自動授權 NLSDD subagent 在 lane worktree 內自行 `git commit`
- 若 main agent 已處於使用者明確授權的 `proceed` 收斂流程中，且本地 commit 成功後下一步只有單一、低風險、可逆的 finishing 動作，應預設自動接續，不要再多停一次等待額外 `proceed`
- 上一條只適用於沒有分支策略歧義的情況；例如已在 `main` 且唯一自然下一步是 `git push origin main`，可以直接做。若仍存在 merge / PR / push / release 等多條路徑，就必須先停下來對齊

---

### 4. Verification Before Done
- Never mark a task complete without proving it works
- Diff behavior between main and your changes when relevant
- Ask yourself: "Would a staff engineer approve this?"
- Run tests, check logs, demonstrate correctness

---

### 5. Demand Elegance (Balanced)
- For non-trivial changes: pause and ask "is there a more elegant way?"
- If a fix feels hacky: "Knowing everything I know now, implement the elegant solution"
- Skip this for simple, obvious fixes - don't over-engineer
- Challenge your own work before presenting it

---

### 6. Autonomous Bug Fixing
- When given a bug report: just fix it. Don't ask for hand-holding
- Point at logs, errors, failing tests - then resolve them
- Zero context switching required from the user
- Go fix failing CI tests without being told how

---

## Task Management
1. **Plan First**: Write plan to `tasks/todo.md` with checkable items
2. **Verify Plan**: Check in before starting implementation
3. **Track Progress**: Mark items complete as you go
4. **Explain Changes**: High-level summary at each step
5. **Document Results**: Add review section to `tasks/todo.md`
6. **Capture Lessons**: Update `tasks/lessons.md` after corrections

---

## Core Principles
- **Simplicity First**: Make every change as simple as possible. Impact minimal code
- **No Laziness**: Find root causes. No temporary fixes. Senior developer standards

## 共同 baseline 章節

以下檔案由組織 shared baseline 採納至本 repo；與上游的採納錨點由 baseline 倉庫之 `adopted/agent-switch` 分支標示（由維護者在每次完成同步後更新）。

- [ai-rules/adoption-workflow.md](ai-rules/adoption-workflow.md)
- [ai-rules/shared-baseline-sync-and-local-adoption.md](ai-rules/shared-baseline-sync-and-local-adoption.md)
- [ai-rules/shared-rules-entry-and-thin-adapters.md](ai-rules/shared-rules-entry-and-thin-adapters.md)
- [ai-rules/commit-each-minimum-viable-change.md](ai-rules/commit-each-minimum-viable-change.md)
- [ai-rules/before-new-mvc-assess-existing-dirty-tree.md](ai-rules/before-new-mvc-assess-existing-dirty-tree.md)
- [ai-rules/choose-agents-skill-script-or-runbook.md](ai-rules/choose-agents-skill-script-or-runbook.md)
- [ai-rules/verify-third-party-module-interface-before-integration.md](ai-rules/verify-third-party-module-interface-before-integration.md)
- [ai-rules/distinguish-rule-suggestions-from-established-process-state.md](ai-rules/distinguish-rule-suggestions-from-established-process-state.md)
- [ai-rules/surface-stale-untracked-governance-files-at-stop.md](ai-rules/surface-stale-untracked-governance-files-at-stop.md)

# Repository Guidelines

## Project Structure & Module Organization
- `src/**`: Rust crate (`agent-switch`) — TUI, auth snapshots, usage/plot.
- `Cargo.toml`, `Cargo.lock`, `build.rs`, `build-number`: Rust build and embedded version metadata.
- `bin/agent-switch.cjs`: npm-published shim; runs a built `target/*/agent-switch` if present, otherwise `cargo run --bin agent-switch`.
- `scripts/link-dev-bin.cjs`, `scripts/unlink-dev-bin.cjs`: optional dev symlink into global npm `bin`.
- `NLSDD/scripts/**`: NLSDD automation (Node, CommonJS).
- `tests/*.test.js`: `node:test` contract tests (no npm `dependencies` / `devDependencies` for the product).
- `package.json`: npm metadata and `files` publish whitelist; legacy TypeScript CLI and `node_modules` product deps are removed.

## Build, Test, and Development Commands
- **Rust**: `cargo test` (preferred to validate changes; see `CLAUDE.md` build rule).
- **npm**: `npm run build` → `cargo build --bin agent-switch` (also `prepublishOnly`).
- **npm install / ci**: lockfile has no transitive deps; installs are effectively no-ops aside from npm metadata.
- **NLSDD**: `npm run nlsdd:*` or `node NLSDD/scripts/...`.
- **Node tests**: `node --test tests/entrypoints.test.js` (and other `tests/*.test.js` as needed).

## Coding Style & Naming Conventions
- **Rust**: follow existing modules and patterns in `src/`.
- **Node (NLSDD, tests)**: CommonJS; 2-space indent; prefer built-in `node:*` modules.

## Testing Guidelines
- Rust: `cargo test`.
- Node: `node --test` over `tests/*.test.js`; keep assertions aligned with `bin/` shim and `package.json` scripts.

## Commit & Pull Request Guidelines
- Follow Conventional Commit style used in history (`chore: ...`, `init`): prefer `feat:`, `fix:`, `chore:`, `docs:`.
- Keep commits focused and runnable (build passes before commit).
- PRs should include:
  - concise problem/solution summary
  - manual verification steps and outcomes
  - linked issue (if applicable)
  - CLI output snippets for behavior changes.

## Agent-Specific Notes
- If Python tooling is introduced for repo automation, use `uv` workflows (`uv run`, `uv pip`) instead of `pip`.
