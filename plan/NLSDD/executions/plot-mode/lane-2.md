# Lane 2 Plan - Rust Runtime and State

> Ownership family:
> `rust/plot-viewer/src/main.rs`, `rust/plot-viewer/src/app.rs`, `rust/plot-viewer/src/input.rs`, `rust/plot-viewer/src/model.rs`, `rust/plot-viewer/src/render/mod.rs`, `tests/plot-rust-model-contract.test.js`, `tests/plot-viewer-scaffold.test.js`
>
> NLSDD worktree: `.worktrees/lane-2-runtime`
>
> Lane-local verification:
> `cargo test --manifest-path rust/plot-viewer/Cargo.toml`
> `cargo check --manifest-path rust/plot-viewer/Cargo.toml`

## M - Model / State

- [x] Scaffold the Rust snapshot model and loader
- [x] Align Rust `serde` schema with the TypeScript snapshot contract
- [x] Add helpers for current/active profile lookup
- [x] Add a source-level contract test guarding Rust/TS schema alignment
- [ ] Tighten typed decoding for nested `usage` payload only when the viewer truly needs it

## V - View / Shared Runtime Boundary

- [x] Introduce a shared render boundary in `render/mod.rs`
- [x] Move layout ownership from `app.rs` into the render boundary
- [x] Expand the render boundary so panels can receive real render space without borrowing ad-hoc scope during Lane 4 work
- [x] Stabilize the render-boundary API so chart/panels lanes stop needing scope expansion

## C - Controller / Runtime Flow

- [x] Scaffold `main.rs`, terminal lifecycle, input mapping, and event loop
- [x] Keep profile/focus navigation flowing through app state
- [x] Add scaffold regression tests for crate/runtime surface
- [x] Add a runtime smoke path that proves snapshot load plus clean quit against a real fixture

## Current Lane Status

- [x] First-round boundary seam landed and passed review
- [x] Second-round boundary API stabilization implementer commit landed
- [ ] Review second-round commit `3b62c5b`
- [x] This lane occupies one active slot in the execution's lane pool; coordinator may keep extra lanes queued or parked until a slot opens.

## Refill Order

- [x] First refill target after the current item: remaining Controller/runtime reliability work if any appear
- [x] Then consume remaining View/runtime-boundary hardening
- [ ] Leave model deepening for later unless a runtime task is blocked on it
