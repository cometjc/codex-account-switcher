# Lane 3 Plan - Rust Chart Surface

> Ownership family:
> `rust/plot-viewer/src/render/chart.rs` and chart-specific regression tests added later under `tests/`
>
> NLSDD worktree: `.worktrees/lane-3-chart`
>
> Lane-local verification:
> `cargo test --manifest-path rust/plot-viewer/Cargo.toml render::chart -- --nocapture`
> `cargo check --manifest-path rust/plot-viewer/Cargo.toml`

## M - Model / Chart Mapping

- [x] Derive a chart-friendly view model from the shared render context
- [x] Normalize focused profile curve inputs, 7d bounds, and 5h band availability into chart-local helpers
- [ ] Add chart-specific regression coverage once visible rendering becomes stable

## V - View / Chart Rendering

- [x] Replace the pure no-op chart renderer with a visible chart region
- [x] Render a first meaningful placeholder that uses focused profile and window data, not fixed text
- [x] Upgrade the placeholder into a more real 7d curve surface
- [ ] Add 5h band, axis labels, and fallback note for unavailable band data

## C - Controller / Focus Consumption

- [ ] Consume current selection and focus state from the shared render boundary without reintroducing app-owned layout
- [ ] Keep chart behavior compatible with later left/right profile cycling and focus changes

## Current Lane Status

- [x] First-round `ChartViewModel` extraction landed and passed review
- [x] Second-round ASCII 7d curve surface implementer commit landed
- [ ] Review second-round commit `907cbc7`
- [x] This lane occupies one active slot in the execution's lane pool; extra lanes can stay queued or parked until promotion.

## Refill Order

- [x] First refill target after the current item: chart fallbacks and legend/axis work
- [ ] Then land chart-specific regression tests
- [ ] Only then widen into richer focus behavior if still needed
