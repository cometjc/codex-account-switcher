# Governance

- 本 `AGENTS.md` 為本 repo 的治理權威入口。
- 與組織 shared baseline 對齊之正式章節見下方 [共同 baseline 章節](#共同-baseline-章節)（`ai-rules/*.md`）；後續同步請依 `skills/new-rule/SKILL.md` 執行。

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
- **多 subagent / parallel-lane（PLD）**：本 repo 使用 **`plugins/parallel-lane-dev/`**（`scripts` / `skills` 預設 symlink 至同層 **`agent-plugins`** 內套件；安裝見 **`scripts/install-pld-plugin.sh`** 與該目錄 `README.md`）。真相來源：**`.pld/executor.sqlite`**。口令 **pld-go** → **`npm run pld:executor:go`**。技能：**`plugins/parallel-lane-dev/skills/parallel-lane-dev/SKILL.md`**。**

---

### 3. Self-Improvement Loop
- After ANY correction from the user: update `tasks/lessons.md` with the pattern
- Write rules for yourself that prevent the same mistake
- Ruthlessly iterate on these lessons until mistake rate drops
- Review lessons at session start for relevant project
- 若此次請求本身是規則新增或規則修正，且正式規則文件已更新並完成驗證，main agent 應預設直接 commit；不要停在「這批還沒 commit」等待額外提醒
- 這條自動 commit 規則適用於 main agent 的本地治理變更，不自動授權平行 lane subagent 在 lane worktree 內自行 `git commit`（見 `parallel-lane-dev` 倉庫規則）
- 若 main agent 已處於使用者明確授權的 `proceed` 收斂流程中，且本地 commit 成功後下一步只有單一、低風險、可逆的 finishing 動作，應預設自動接續，不要再多停一次等待額外 `proceed`
- 上一條只適用於沒有分支策略歧義的情況；例如已在 `main` 且唯一自然下一步是 `git push origin main`，可以直接做。若仍存在 merge / PR / push / release 等多條路徑，就必須先停下來對齊

---

### 4. Verification Before Done
- Never mark a task complete without proving it works
- Diff behavior between main and your changes when relevant
- Ask yourself: "Would a staff engineer approve this?"
- Run tests, check logs, demonstrate correctness
- 若問題提出時附有 tmux app 即時畫面可對照，且 `make test` 成功後會讓 app hot reload 到新版本，則完成驗證前必須重新查看同一個 tmux 畫面，確認 UI 問題已在 live app 中實際消失
- 若重看 tmux live 畫面後仍有落差，不能只以測試綠燈結案；必須繼續修正，並把該 live 誤差補進回歸測試涵蓋範圍

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
1. **Dirty Tree Gate**: Before writing session `plan.md` or implementation files, inspect `git status`; if the current tree is dirty and the dirty paths are not the exact work you are continuing, create and switch to a dedicated worktree first, then do planning and follow-up work there.
2. **Code-change flow**: 凡是會修改程式、規則、腳本、文件結構或其他 tracked 檔案的工作，都不應再直接靠 `todo.md` 開工；入口改成 `$superpower-dev:do`。
3. **Local todo scope**: `tasks/todo.md` is local-only scratchpad for non-code housekeeping; do not use it as the authoritative plan for code changes.
4. **AUQ confirmation gate**: 走 `$superpower-dev:do` 時，若需要在執行器或候選 plan 間做確認／歧義消解，必須使用 AUQ（`mcp__ask_user_questions__ask_user_questions`），不要改用純文字提問。
5. **Track Progress**: Mark checklist items complete as you go in the active spec/plan surfaces.
6. **Explain Changes**: High-level summary at each step.
7. **Document Results**: Add review/verification outcomes to the active spec/plan surfaces.
8. **Capture Lessons**: Update `tasks/lessons.md` after corrections.

---

## Core Principles
- **Simplicity First**: Make every change as simple as possible. Impact minimal code
- **No Laziness**: Find root causes. No temporary fixes. Senior developer standards

## 共同 baseline 章節

以下檔案由組織 shared baseline 採納至本 repo；與上游的採納錨點由 baseline 倉庫之 `adopted/agent-switch` 分支標示（由維護者在每次完成同步後更新）。

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
- **`plugins/parallel-lane-dev/`**：PLD scoreboard / executions（專案專用）；`scripts` 為上游 **`parallel-lane-dev`** 腳本目錄的 symlink（見該目錄 `README.md`）。
- `tests/*.test.js`: `node:test` contract tests (no npm `dependencies` / `devDependencies` for the product).
- `package.json`: npm metadata and `files` publish whitelist; legacy TypeScript CLI and `node_modules` product deps are removed.

## Build, Test, and Development Commands
- **Rust**: `make test`（預設驗證指令；會先 `cargo test`，成功後自動 `make install`）。
- **npm**: `npm run build` → `cargo build --bin agent-switch` (also `prepublishOnly`).
- **npm install / ci**: lockfile has no transitive deps; installs are effectively no-ops aside from npm metadata.
- **PLD**：`npm run pld:executor:audit`、`npm run pld:executor:go` 等（完整列表見根 `package.json` 的 `pld:*`）；需本機 **`sqlite3`** CLI。
- **Node tests**: `node --test tests/entrypoints.test.js` (and other `tests/*.test.js` as needed).

## Coding Style & Naming Conventions
- **Rust**: follow existing modules and patterns in `src/`.
- **Node (contract tests)**: CommonJS; 2-space indent; prefer built-in `node:*` modules.

## Testing Guidelines
- Rust: `make test`（若只想本地快速測且不安裝，才使用 `cargo test`）。
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
### 開發流程

所有 plan 的實作流程固定為：

```
plan[1..n] -> split to lanes[1..m] -> 4a parallel-lane flow（`plugins/parallel-lane-dev/`、`npm run pld:*`）
```

- 收到實作任務時，先確認 plan 拆分完成
- 將 plan 切分為獨立 lanes
- 以 4a parallel-lane flow 執行各 lane（腳本 symlink 自 `agent-plugins` 套件；技能 `plugins/parallel-lane-dev/skills/parallel-lane-dev/SKILL.md`）
- 不可跳過 lane 拆分直接實作
- 若使用者提供 tmux app 即時畫面作為 UI 問題參考，則在 `make test` 成功並觸發 hot reload 後，必須回看同一個 tmux pane；若 live UI 仍有偏差，該偏差要在繼續修正時一併補進測試

### Build Rule

Always use `make test` as the default verification command for Rust changes.  
Use `cargo test` only for explicit quick local checks or when the user explicitly requests it.
