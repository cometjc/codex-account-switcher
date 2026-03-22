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
- [x] Surface routing recommendation details inside the Compare panel without stealing layout ownership from the runtime lane
- [x] Highlight the adopted routing target more clearly once compare-panel data becomes meaningful
- [x] Refresh Summary / Compare content from real selected/current/focus state once the runtime/chart reactivation lands

## Current Lane Status

- [x] Projected phase: implementing
- [x] Current item: Refresh Summary / Compare content from real selected/current/focus state once the runtime/chart reactivation lands
- [x] Latest commit: `6bb1fba`
- [x] Latest event: state-update · Panels now read shared selection/chart state directly and carry stable Summary / Compare regression coverage.
- [x] Next expected phase: spec-review-pending
- [x] Next refill target: n/a
- [x] Latest note: Summary / Compare lines now distinguish adopted target vs current route, include live 7d sample and 5h band status, and stop relying on placeholder routing copy.

## Refill Order

- [x] First refill target after the current item: panel-specific regression coverage for the visible Summary / Compare structure
- [x] Then keep the panel structure extensible through a lane-local field-mapping refactor
- [x] Next active refill: add recommendation/bottleneck-rich Compare content with stable visible output after Lane 2 exposes the render-boundary payload
- [x] Current refill: highlight the adopted routing target more clearly once compare data is present
- [x] Then extend panel structure only if a later execution item truly needs richer side-panel content
