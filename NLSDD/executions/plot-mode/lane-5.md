# Lane 5 Plan - Plot Viewer Docs and Operator Flow

> Ownership family:
> `README.md`, `tests/plot-readme.test.js`, `tests/plot-mode-shell.test.js`
>
> NLSDD worktree: `.worktrees/lane-5-docs`
>
> Lane-local verification:
> `npm run build`

## M - Model / Workflow Framing

- [x] Keep plot-mode framed as a Phase 1 migration where Node still owns data access
- [ ] Tighten README wording so the recovery baseline and local operator flow are understandable without lane-history context

## V - View / Operator Surface

- [x] Keep plot-mode visible in the Node shell and README as an in-progress developer-facing mode
- [ ] Add local run/build instructions that match the current recovery-baseline workflow

## C - Controller / Verification Surfaces

- [x] Keep README regression tests aligned with current plot-mode wording
- [ ] Add explicit operator guidance for building the Rust viewer before retrying plot-mode launch
- [ ] Bring the shell/readme regression files into the tracked recovery-baseline workflow before making them lane-local required verification

## Current Lane Status

- [ ] First-round docs/operator-flow lane not started on `main` yet
- [x] This lane exists to fill the fourth active slot without stealing ownership from the Node handoff or Rust render lanes

## Refill Order

- [ ] First refill target after the current item: document the trusted local plot-mode recovery workflow in `README.md`
- [ ] Then align shell/readme regression tests with that workflow
- [ ] Only then widen into extra doc polish if visible plot-mode behavior changes again
