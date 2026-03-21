# Lane 4 Plan - Rust Panels, Docs, and Regression Surfaces

> Ownership family:
> `rust/plot-viewer/src/render/panels.rs`, `README.md`, `tests/plot-readme.test.js`, and panel-specific regression tests added later under `tests/`
>
> NLSDD worktree: `.worktrees/lane-4-panels`
>
> Lane-local verification:
> `node --test tests/plot-readme.test.js`
> `cargo test render_panels_builds_visible_summary_and_compare_blocks --manifest-path rust/plot-viewer/Cargo.toml`

## M - Model / Panel Mapping

- [x] Build focused-profile summary mapping inside the panels lane
- [x] Build compare-skeleton mapping for selected/current routing context
- [ ] Add panel-specific regression coverage once panel wording and structure settle

## V - View / Panel Rendering

- [x] Replace the pure no-op panels renderer with a visible summary panel after Lane 2 exposes the required render boundary
- [x] Add a compare skeleton that can later host routing recommendation details after the same boundary item lands
- [ ] Keep the panel structure extensible for future side-panel richness

## C - Controller / Docs and Verification Surfaces

- [x] Keep README plot-mode positioning and phase-1 framing explicit
- [x] Update README wording when `plot:viewer:*` stops being placeholder scaffolding
- [x] Keep README regression tests aligned with current plot-mode wording
- [x] Add panel-oriented documentation once side panels become visually meaningful

## Current Lane Status

- [x] First-round visible Summary / Compare panel surface landed and passed review
- [x] Second-round panel docs / README regression implementer commit landed
- [ ] Close the current correction / re-review loop for second-round commit `12785d1`
- [x] This lane occupies one active slot in the execution's lane pool; queued lanes may wait until a slot opens.

## Refill Order

- [x] First refill target after the current item: panel-specific regression coverage for the visible Summary / Compare structure
- [ ] Then consume README/doc polish that reflects newly visible panel behavior
- [ ] Then extend panel structure only if a later execution item truly needs richer side-panel content
