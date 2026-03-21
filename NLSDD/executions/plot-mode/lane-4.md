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
- [x] Add panel-specific regression coverage once panel wording and structure settle

## V - View / Panel Rendering

- [x] Replace the pure no-op panels renderer with a visible summary panel after Lane 2 exposes the required render boundary
- [x] Add a compare skeleton that can later host routing recommendation details after the same boundary item lands
- [x] Keep the panel structure extensible for future side-panel richness

## C - Controller / Docs and Verification Surfaces

- [x] Keep README plot-mode positioning and phase-1 framing explicit
- [x] Update README wording when `plot:viewer:*` stops being placeholder scaffolding
- [x] Keep README regression tests aligned with current plot-mode wording
- [x] Add panel-oriented documentation once side panels become visually meaningful

## Current Lane Status

- [x] First-round visible Summary / Compare panel surface landed and passed review
- [x] Second-round panel docs / README regression implementer commit landed
- [x] Close the current correction / re-review loop for second-round commit `12785d1`
- [x] Third-round panel regression commit `abd8b10` landed and passed spec + quality review
- [x] Fourth-round panel field-mapping refactor commit `b24f12a` landed with stable visible output
- [x] This lane occupies one active slot in the execution's lane pool; queued lanes may wait until a slot opens.

## Refill Order

- [x] First refill target after the current item: panel-specific regression coverage for the visible Summary / Compare structure
- [x] Then keep the panel structure extensible through a lane-local field-mapping refactor
- [ ] Then consume README/doc polish only if newly visible panel behavior changes again
- [ ] Then extend panel structure only if a later execution item truly needs richer side-panel content
