# Plot Mode Reactivation Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Re-open `plot-mode` as an honest NLSDD execution by turning the remaining product gap into dispatchable lane work, with Rust rendering upgraded from scaffold text into a visible 7d/5h plot and runtime interaction kept coherent across chart/panels.

**Architecture:** Keep the existing Node snapshot handoff intact and treat the Rust viewer as the only surface that needs new implementation. Re-open only the lanes that still map to real unfinished behavior: runtime interaction/state propagation, chart rendering, and panel refresh richness. Tracking/docs should be updated first so `nlsdd-go` can dispatch truthful work instead of seeing an all-parked execution.

**Tech Stack:** TypeScript CLI handoff, Rust `ratatui` viewer, NLSDD scoreboard/lane plans, `node --test`, `cargo test`, `cargo check`.

---

## File Structure

- Modify: `NLSDD/scoreboard.md`
  - Re-open only the plot-mode lanes that correspond to real unfinished work.
- Modify: `NLSDD/executions/plot-mode/overview.md`
  - Replace the stale parked-everywhere picture with the new 4a execution truth.
- Modify: `NLSDD/executions/plot-mode/lane-2.md`
  - Re-scope Lane 2 around runtime interaction and shared render-state propagation.
- Modify: `NLSDD/executions/plot-mode/lane-3.md`
  - Re-scope Lane 3 around real chart rendering, band geometry, and chart-local regression coverage.
- Modify: `NLSDD/executions/plot-mode/lane-4.md`
  - Re-scope Lane 4 around summary/compare refresh against the richer runtime/chart state.
- Modify: `NLSDD/executions/plot-mode/lane-5.md`
  - Keep docs parked by default, but record when it should wake up after visible behavior changes.
- Modify: `tasks/todo.md`
  - Track the replan decision and the runtime truth that triggered it.
- Modify: `rust/plot-viewer/src/app.rs`
  - If execution proceeds, keep selected profile and focus transitions coherent for every render surface.
- Modify: `rust/plot-viewer/src/render/mod.rs`
  - If execution proceeds, expose shared render state that chart/panels can consume without duplicating runtime logic.
- Modify: `rust/plot-viewer/src/render/chart.rs`
  - If execution proceeds, replace placeholder paragraphs with real chart rendering.
- Modify: `rust/plot-viewer/src/render/panels.rs`
  - If execution proceeds, refresh summary/compare content from real focused/current state.
- Test: `tests/plot-mode-shell.test.js`
- Test: `tests/plot-handoff.test.js`
- Test: `tests/plot-readme.test.js`
- Test: `tests/plot-rust-model-contract.test.js`
- Test: `tests/plot-viewer-scaffold.test.js`

## Chunk 1: Re-open Plot-Mode Tracking Truth

### Task 1: Rewrite plot-mode execution tracking so honest lane work exists again

**Files:**
- Modify: `NLSDD/scoreboard.md`
- Modify: `NLSDD/executions/plot-mode/overview.md`
- Modify: `NLSDD/executions/plot-mode/lane-2.md`
- Modify: `NLSDD/executions/plot-mode/lane-3.md`
- Modify: `NLSDD/executions/plot-mode/lane-4.md`
- Modify: `NLSDD/executions/plot-mode/lane-5.md`
- Modify: `tasks/todo.md`

- [x] **Step 1: Write the tracking delta before editing**

Capture the runtime truth that triggered the replan:
- `plot-mode` product gap remains open because Rust viewer still renders scaffold text rather than the real chart
- current coordinator truth is `no-dispatchable-lane`
- the remaining work belongs to Lane 2 / 3 / 4, not Lane 1

- [x] **Step 2: Update tracked execution surfaces**

Rewrite the plot-mode tracking docs so they say:
- Lane 1 stays parked unless Node handoff or launch UX regresses
- Lane 2 is queued for runtime interaction/state propagation
- Lane 3 is queued for real chart rendering
- Lane 4 is queued behind Lane 2/3 for panel refresh/richer compare output
- Lane 5 stays parked until visible behavior changes enough to require docs updates

- [x] **Step 3: Run scoreboard/tracking verification**

Run:
- `npm run nlsdd:scoreboard:refresh`
- `node NLSDD/scripts/nlsdd-build-dispatch-plan.cjs --execution plot-mode --dry-run --json`

Expected:
- tracked/runtime scoreboards stay coherent
- `plot-mode` now reports honest queued work instead of `no-dispatchable-lane`

- [x] **Step 4: Commit**

```bash
git add NLSDD/scoreboard.md NLSDD/executions/plot-mode/overview.md NLSDD/executions/plot-mode/lane-2.md NLSDD/executions/plot-mode/lane-3.md NLSDD/executions/plot-mode/lane-4.md NLSDD/executions/plot-mode/lane-5.md tasks/todo.md
git commit -m "chore(plot): 重啟 plot-mode 執行規劃"
```

## Chunk 2: Runtime Interaction And Shared State

### Task 2: Re-open Lane 2 for interaction/state coherence

**Files:**
- Modify: `rust/plot-viewer/src/app.rs`
- Modify: `rust/plot-viewer/src/render/mod.rs`
- Test: `tests/plot-viewer-scaffold.test.js`

- [x] **Step 1: Write the failing test**

Add coverage that proves:
- left/right moves the selected profile
- tab/shift-tab changes focus
- the render boundary exposes the same selected/current/focus truth to downstream renderers

- [x] **Step 2: Run test to verify coverage exercises the seam**

Run: `cargo test --manifest-path rust/plot-viewer/Cargo.toml`
Expected: the new coverage exercises the richer runtime/render-state contract.

- [x] **Step 3: Write minimal implementation**

Tighten `app.rs` + `render/mod.rs` so:
- selected/current/focus state stays centralized
- render consumers do not need to re-derive runtime state ad hoc
- later chart/panel tasks can trust a single render-state view

- [x] **Step 4: Run lane verification**

Run:
- `cargo test --manifest-path rust/plot-viewer/Cargo.toml`
- `cargo check --manifest-path rust/plot-viewer/Cargo.toml`

Expected: PASS

- [x] **Step 5: Commit**

```bash
git add rust/plot-viewer/src/app.rs rust/plot-viewer/src/render/mod.rs tests/plot-viewer-scaffold.test.js
git commit -m "feat(plot): 收斂 viewer runtime 互動狀態"
```

## Chunk 3: Real Chart Rendering

### Task 3: Re-open Lane 3 for the first real 7d/5h plot

**Files:**
- Modify: `rust/plot-viewer/src/render/chart.rs`
- Test: chart-focused Rust regression coverage under `rust/plot-viewer`

- [x] **Step 1: Write the failing test**

Add regression coverage for:
- visible 7d curve output derived from `sevenDayPoints`
- 5h band output when `fiveHourBand.available` is true
- fallback messaging when the band is unavailable
- axis/legend text staying aligned with the rendered chart surface

- [x] **Step 2: Run test to verify coverage exercises visible chart output**

Run:
- `cargo test --manifest-path rust/plot-viewer/Cargo.toml render::chart -- --nocapture`

Expected: the new coverage proves the renderer no longer relies on scaffold copy like `pending Canvas plot`.

- [x] **Step 3: Write minimal implementation**

Replace the current placeholder paragraph renderer with a real chart implementation that:
- draws a visible 7d shape from snapshot points
- draws the 5h band when data is available
- keeps the focused/current labels visible
- preserves a stable fallback when a profile has no usable band

- [x] **Step 4: Run lane verification**

Run:
- `cargo test --manifest-path rust/plot-viewer/Cargo.toml render::chart -- --nocapture`
- `cargo check --manifest-path rust/plot-viewer/Cargo.toml`

Expected: PASS

- [x] **Step 5: Commit**

```bash
git add rust/plot-viewer/src/render/chart.rs
git commit -m "feat(plot): 補上可見的 7d 與 5h 圖表"
```

## Chunk 4: Panels Against The Real Viewer State

### Task 4: Re-open Lane 4 for summary/compare refresh against the richer runtime/chart state

**Files:**
- Modify: `rust/plot-viewer/src/render/panels.rs`
- Test: panel-focused Rust regression coverage under `rust/plot-viewer`

- [x] **Step 1: Write the failing test**

Add regression coverage that proves:
- summary content follows the focused profile
- compare content distinguishes selected vs current profile correctly after profile cycling
- panel copy uses real runtime state instead of scaffold placeholders

- [x] **Step 2: Run test to verify coverage exercises panel refresh behavior**

Run:
- `cargo test render_panels_builds_visible_summary_and_compare_blocks --manifest-path rust/plot-viewer/Cargo.toml`
- `cargo test render_panels_locks_visible_summary_compare_copy_and_shape --manifest-path rust/plot-viewer/Cargo.toml`

Expected: the new coverage proves panel output no longer relies on placeholder routing copy and limited state detail.

- [x] **Step 3: Write minimal implementation**

Upgrade `panels.rs` so:
- summary/compare content refreshes from the richer shared state
- selected/current/focus semantics stay coherent after navigation
- panel wording remains truthful to the now-visible chart/runtime behavior

- [x] **Step 4: Run lane verification**

Run:
- `cargo test render_panels_builds_visible_summary_and_compare_blocks --manifest-path rust/plot-viewer/Cargo.toml`
- `cargo test render_panels_locks_visible_summary_compare_copy_and_shape --manifest-path rust/plot-viewer/Cargo.toml`
- `cargo check --manifest-path rust/plot-viewer/Cargo.toml`

Expected: PASS

- [x] **Step 5: Commit**

```bash
git add rust/plot-viewer/src/render/panels.rs
git commit -m "feat(plot): 讓 summary 與 compare 面板跟隨 viewer 狀態"
```

## Chunk 5: Docs Wake-Up Only If Behavior Changed

### Task 5: Re-open Lane 5 only when visible operator behavior changed

**Files:**
- Modify: `README.md`
- Modify: `tests/plot-readme.test.js`
- Modify: `tests/plot-mode-shell.test.js`

- [x] **Step 1: Check whether user-visible behavior changed enough to require docs edits**

Only proceed if:
- the viewer now renders a real chart
- operator run/build instructions need updated expectations

- [x] **Step 2: If needed, write the failing doc regression**

Add/update README or shell regression coverage for the new plot-mode behavior.

- [x] **Step 3: If needed, update docs minimally**

Keep docs honest:
- plot-mode is still phase 1
- Rust owns rendering
- the local recovery/build/run path matches real behavior

- [x] **Step 4: Run doc verification**

Run:
- `npm run build`
- `node --test tests/plot-readme.test.js`
- `node --test tests/plot-mode-shell.test.js`

Expected: PASS

- [x] **Step 5: Commit**

```bash
git add README.md tests/plot-readme.test.js tests/plot-mode-shell.test.js
git commit -m "docs(plot): 更新 plot-mode 操作說明"
```
