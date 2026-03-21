# Ratatui Plot Mode Implementation Plan

> **For agentic workers:** REQUIRED: Use the repo's `NLSDD` workflow and the plot-mode execution lane plans under `NLSDD/executions/plot-mode/` when running this plan with subagents. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a new Rust-based `plot` mode that visualizes per-profile 7d usage curves plus current 5h projection bands, and establish the first migration step from the current Node CLI toward a Rust TUI architecture.

**Architecture:** Keep the existing TypeScript CLI as the source of truth for auth/account/cache/API access in Phase 1. Add a Rust workspace app that consumes a normalized snapshot JSON exported by the Node CLI and renders the chart UI with `ratatui` `Canvas`, because the 5h projected parallel-band geometry is custom drawing rather than a standard dataset chart.

**Tech Stack:** TypeScript, Node.js, Rust, Cargo, `ratatui`, `crossterm`, JSON snapshot interchange

## NLSDD Execution Status

- [x] Lane 1 landed real-binary handoff verification against the Rust viewer and passed spec + quality review.
- [x] Lane 2 landed the drawable panel boundary seam and passed spec + quality review.
- [x] Lane 3 landed a local `ChartViewModel` extraction in `chart.rs` and passed spec + quality review.
- [x] Lane 4 landed the first visible Summary/Compare panel surface and passed spec + quality review.
- [x] Lane 1 second-round shell confidence messaging implementer commit landed and is now tracked in `NLSDD/scoreboard.md`.
- [x] Lane 2 second-round boundary API stabilization implementer commit landed and is now tracked in `NLSDD/scoreboard.md`.
- [x] Lane 3 second-round ASCII 7d curve surface implementer commit landed and is now tracked in `NLSDD/scoreboard.md`.
- [x] Lane 4 second-round panel docs / README regression implementer commit landed and is now tracked in `NLSDD/scoreboard.md`.
- [x] Re-cut the next 4-active-lane plan from current NLSDD runtime/manual state instead of blindly reusing the original second-round queue
- [x] Keep Lane 1 / Lane 3 / Lane 4 as the preferred post-correction refill lanes
- [x] Mark Lane 2 as conditional: keep it active only while the current correction loop is open, then park it unless another lane proves the stronger decode path is needed
- [x] Narrow the next lane-local refill items to:
  - Lane 1: tighten plot snapshot builder semantics for real 7d history and 5h band math
  - Lane 3: add 5h band, axis labels, and unavailable-band fallback
  - Lane 4: add panel-specific regression coverage for the visible Summary / Compare structure
- [x] Land the third-round Lane 1 refill (`baa7b8e`) and pass spec + quality review
- [x] Close Lane 2's current correction as a no-op and park the lane unless another lane proves nested `usage` decoding is necessary
- [x] Land the third-round Lane 3 refill (`585317d`) and pass spec + quality review
- [x] Land the third-round Lane 4 refill (`abd8b10`) and pass spec + quality review
- [x] Land the fourth-round Lane 3 wording refinement (`35c8351`) and keep it chart-local
- [x] Land the fourth-round Lane 4 field-mapping refactor (`b24f12a`) without changing visible panel output
- [x] Build a clean integration branch from shared baseline `d19d319` and merge the accepted Lane 1 / 3 / 4 stacks there

---

## File Structure

- Create: `rust/plot-viewer/Cargo.toml`
- Create: `rust/plot-viewer/src/main.rs`
- Create: `rust/plot-viewer/src/app.rs`
- Create: `rust/plot-viewer/src/model.rs`
- Create: `rust/plot-viewer/src/render/mod.rs`
- Create: `rust/plot-viewer/src/render/chart.rs`
- Create: `rust/plot-viewer/src/render/panels.rs`
- Create: `rust/plot-viewer/src/input.rs`
- Create: `src/lib/plot/plot-snapshot.ts`
- Create: `src/lib/plot/index.ts`
- Modify: `src/commands/root.ts`
- Modify: `package.json`
- Create: `plan/2026-03-20-ratatui-plot-mode-implementation-plan.md`

## Phase Breakdown

### Task 1: Define the cross-runtime snapshot contract

**Files:**
- Create: `src/lib/plot/plot-snapshot.ts`
- Create: `src/lib/plot/index.ts`

- [ ] **Step 1: Define TypeScript snapshot interfaces**

Create TS types for:
- `PlotSnapshot`
- `PlotProfile`
- `PlotWindowPoint`
- `PlotFiveHourBand`

Required fields:
- profile id/name/current marker
- 7d window start/end unix seconds
- ordered 7d usage points in normalized `% used`
- active 5h window start/end unix seconds
- 5h projected band lower/upper bounds expressed in 7d-plot Y units
- summary labels already shown by the current CLI (`Time to reset`, `Usage Left`, `Drift`, `Pacing Status`)

- [ ] **Step 2: Add a pure builder function**

Add a single exported function that converts the current `UsageResponse` / saved-profile menu data into `PlotSnapshot`.

Rules:
- X coordinates are profile-local window time, not absolute shared time.
- Y coordinates are 7d used percent.
- The 5h band height is computed from the user-defined projection ratio:
  - `bandHeight = (delta7dPercent / delta5hPercent) * 100`
- If `delta5hPercent <= 0`, omit the band and mark it unavailable.

- [ ] **Step 3: Add serialization helper**

Add a function that returns stable pretty JSON for the Rust viewer.

- [ ] **Step 4: Verify TypeScript build**

Run: `npm run build`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib/plot src/commands/root.ts package.json
git commit -m "feat: define plot snapshot contract"
```

### Task 2: Add Node-side plot mode entrypoint

**Files:**
- Modify: `src/commands/root.ts`
- Modify: `package.json`

- [ ] **Step 1: Add a new mode state**

Extend the current mode model so the UI can switch between:
- `quota`
- `delta`
- `plot`

- [ ] **Step 2: Build snapshot from current menu state**

When `plot` is selected:
- gather all rendered profiles
- build `PlotSnapshot`
- write it to a temp file under the repo temp/writable area

- [ ] **Step 3: Spawn the Rust viewer process**

Use a non-interactive child process launch from Node:
- pass snapshot path as CLI arg
- forward stdio to terminal
- return to Node flow after the viewer exits

- [ ] **Step 4: Add package scripts**

Add scripts for:
- Rust viewer build
- Rust viewer run (for local testing)

- [ ] **Step 5: Verify Node build**

Run: `npm run build`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/commands/root.ts package.json
git commit -m "feat: add plot mode handoff from node cli"
```

### Task 3: Scaffold Rust viewer app

**Files:**
- Create: `rust/plot-viewer/Cargo.toml`
- Create: `rust/plot-viewer/src/main.rs`
- Create: `rust/plot-viewer/src/app.rs`
- Create: `rust/plot-viewer/src/model.rs`
- Create: `rust/plot-viewer/src/input.rs`

- [ ] **Step 1: Create Cargo package**

Add dependencies:
- `ratatui`
- `crossterm`
- `serde`
- `serde_json`
- `anyhow`

- [ ] **Step 2: Mirror the snapshot schema in Rust**

Implement `serde` structs matching the TypeScript snapshot exactly.

- [ ] **Step 3: Add snapshot loader**

Read snapshot JSON from the CLI arg path and deserialize into app state.

- [ ] **Step 4: Add terminal lifecycle**

Implement:
- enter alternate screen
- raw mode on/off
- graceful restore on panic/error

- [ ] **Step 5: Add basic event loop**

Support:
- `q` / `Esc` to exit
- left/right to cycle profiles
- tab / shift-tab to cycle focus if multiple panels are added later

- [ ] **Step 6: Verify Rust compile**

Run: `cargo check --manifest-path rust/plot-viewer/Cargo.toml`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add rust/plot-viewer
git commit -m "feat: scaffold rust plot viewer"
```

### Task 4: Render the 7d curve + 5h band chart

**Files:**
- Create: `rust/plot-viewer/src/render/mod.rs`
- Create: `rust/plot-viewer/src/render/chart.rs`

- [ ] **Step 1: Render the main chart with `Canvas`**

Use `ratatui::widgets::canvas::Canvas` rather than `Chart`.

Render:
- one 7d usage curve per profile
- current profile emphasized
- per-profile local X bounds derived from that profile’s 7d window

- [ ] **Step 2: Draw the current 5h parallel band**

For the focused profile:
- compute the x-range of the active 5h window relative to the 7d local axis
- draw two orange parallel guide lines
- keep line spacing equal to the projected band height in 7d Y units

- [ ] **Step 3: Add axis labels and legend text**

Show:
- 7d start/end
- 0–100% Y axis
- focused profile label
- small legend for line colors and 5h band meaning

- [ ] **Step 4: Add empty/fallback rendering**

If a profile lacks enough data for a band:
- still draw the 7d curve
- show a text note: `5h band unavailable`

- [ ] **Step 5: Verify visual rendering**

Run: `cargo run --manifest-path rust/plot-viewer/Cargo.toml -- <snapshot-path>`
Expected: viewer opens, chart draws, exit cleanly with `q`

- [ ] **Step 6: Commit**

```bash
git add rust/plot-viewer/src/render
git commit -m "feat: render 7d plot and 5h projection band"
```

### Task 5: Add side panels for routing decisions

**Files:**
- Create: `rust/plot-viewer/src/render/panels.rs`
- Modify: `rust/plot-viewer/src/app.rs`

- [ ] **Step 1: Add profile summary panel**

Render for focused profile:
- `Pacing Status`
- `Recommendation`
- `Weekly Drift`
- `5H Drift`
- `Time to reset`
- `Usage Left`

- [ ] **Step 2: Add compare panel**

Show ranked profiles with:
- profile name
- recommendation label
- bottleneck source `[W]/[5H]`

- [ ] **Step 3: Highlight routing recommendation**

Apply stronger style to the recommended target profile.

- [ ] **Step 4: Verify focus changes**

Arrow keys or tab should update focused profile and panel details.

- [ ] **Step 5: Commit**

```bash
git add rust/plot-viewer/src/app.rs rust/plot-viewer/src/render/panels.rs
git commit -m "feat: add routing side panels to plot viewer"
```

### Task 6: Integrate and document the first migration step

**Files:**
- Modify: `README.md`
- Modify: `package.json`
- Modify: `src/commands/root.ts`

- [ ] **Step 1: Add explicit “Rust viewer is Phase 1 migration” notes**

Document that:
- Node remains source of truth for data access
- Rust owns only rendering in this phase
- later phases can move data logic over incrementally

- [ ] **Step 2: Add local run instructions**

Document exact commands to:
- build TS
- build Rust viewer
- launch plot mode

- [ ] **Step 3: End-to-end verification**

Run:
- `npm run build`
- `cargo check --manifest-path rust/plot-viewer/Cargo.toml`

Expected:
- both pass
- no broken Node CLI build

- [ ] **Step 4: Final commit**

```bash
git add README.md package.json src/commands/root.ts rust/plot-viewer
git commit -m "docs: describe rust plot viewer migration path"
```

## Testing Notes

- Use snapshot fixtures with:
  - one fully unused profile
  - one profile with steep recent 7d jump
  - one profile with a narrow 5h window and high projected band
- Verify that profile-local X axes differ across profiles.
- Verify that 5h band geometry matches the intended projection rule, not absolute 5h percentage alone.
- Verify ANSI restore on crash/exit so terminal state is not left broken.

## Design Constraints

- Do not reimplement auth/cache/API fetch logic in Rust during Phase 1.
- Do not invent a shared absolute-time X axis for all profiles; keep each profile aligned to its own 7d window origin.
- Prefer `Canvas` over `Chart` for the production implementation because the orange projection band is custom geometry. `Chart` may still be used later for fallback/minimap views.

## Relevant Docs

- Ratatui `Canvas`: https://docs.rs/ratatui/latest/ratatui/widgets/canvas/struct.Canvas.html
- Ratatui canvas module: https://docs.rs/ratatui/latest/ratatui/widgets/canvas/index.html
- Ratatui widget overview: https://docs.rs/ratatui/latest/ratatui/widgets/index.html
- Ratatui chart docs (reference only): https://docs.rs/ratatui/latest/ratatui/widgets/struct.Chart.html
