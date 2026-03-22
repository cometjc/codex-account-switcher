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
- [x] Add chart-specific regression coverage once visible rendering becomes stable

## V - View / Chart Rendering

- [x] Replace the pure no-op chart renderer with a visible chart region
- [x] Render a first meaningful placeholder that uses focused profile and window data, not fixed text
- [x] Upgrade the placeholder into a more real 7d curve surface
- [x] Add 5h band, axis labels, and fallback note for unavailable band data

## C - Controller / Focus Consumption

- [x] Consume current selection and focus state from the shared render boundary without reintroducing app-owned layout
- [x] Keep chart behavior compatible with later left/right profile cycling and focus changes

## Current Lane Status

- [x] Projected phase: implementing
- [x] Current item: Render the first real 7d curve and 5h band directly from runtime-owned chart state
- [x] Latest commit: `c5f6c26`
- [x] Latest event: state-update · Replaced chart placeholder copy with a real ratatui chart, axis labels, band overlays, and buffer-based regressions.
- [x] Next expected phase: spec-review-pending
- [x] Next refill target: n/a
- [x] Latest note: Chart now consumes shared runtime/chart state, renders a visible 7d line plus 5h band summary, and covers both available-band and unavailable-band cases with Rust tests.

## Refill Order

- [x] First refill target after the current item: add 5h band, axis labels, and unavailable-band fallback
- [x] Then land chart-specific regression tests
- [x] Then narrow focus consumption into a chart-local wording refinement that keeps layout ownership in the shared boundary
- [ ] Only then widen into richer focus behavior if still needed
