# Lane 4 Plan - Rust Panels and Recommendation Surface

> Ownership family:
> `rust/plot-viewer/src/render/panels.rs` and panel-specific regression tests added later under `tests/`
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

- [x] Keep visible panel copy aligned with the current plot-mode semantics
- [ ] Surface routing recommendation details inside the Compare panel without stealing layout ownership from the runtime lane
- [ ] Highlight the adopted routing target more clearly once compare-panel data becomes meaningful

## Current Lane Status

- [x] First-round visible Summary / Compare panel surface landed and passed review
- [x] Second-round panel docs / README regression implementer commit landed
- [x] Close the current correction / re-review loop for second-round commit `12785d1`
- [x] Third-round panel regression commit `abd8b10` landed and passed spec + quality review
- [x] Fourth-round panel field-mapping refactor commit `b24f12a` landed with stable visible output
- [ ] Next active refill should enrich the visible Compare panel with recommendation/bottleneck details on top of the recovery baseline
- [x] This lane occupies one active slot in the execution's lane pool; queued lanes may wait until a slot opens.

## Refill Order

- [x] First refill target after the current item: panel-specific regression coverage for the visible Summary / Compare structure
- [x] Then keep the panel structure extensible through a lane-local field-mapping refactor
- [ ] Next active refill: add recommendation/bottleneck-rich Compare content with stable visible output
- [ ] Then extend panel structure only if a later execution item truly needs richer side-panel content
