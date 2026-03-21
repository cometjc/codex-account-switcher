# Lane 1 Plan - Node Contract and Handoff

> Ownership family:
> `src/commands/root.ts`, `src/lib/plot/**`, `package.json`, `tests/plot-mode-shell.test.js`, `tests/plot-snapshot.test.js`, `tests/plot-handoff.test.js`
>
> NLSDD worktree: `.worktrees/lane-1-node`
>
> Lane-local verification:
> `npm run build`
> `node --test tests/plot-handoff.test.js`
> `node --test tests/plot-mode-shell.test.js`

## M - Model / Contract

- [x] Define the TypeScript plot snapshot contract and barrel export
- [x] Add stable JSON serialization for Rust handoff
- [x] Add regression tests for snapshot builder and serializer
- [x] Tighten snapshot builder semantics when real 7d history or 5h band math evolves

## V - View / Shell Surface

- [x] Add `plot` as a visible mode in the Node shell
- [x] Keep help text and mode cycle regression-tested
- [x] Refine plot-mode shell messaging once Rust viewer becomes reliably launchable

## C - Controller / Handoff

- [x] Build a temp snapshot from current menu state
- [x] Write the snapshot to `/tmp` and preserve the path when viewer launch is unavailable
- [x] Add cargo-backed `plot:viewer:build` and `plot:viewer:run` scripts
- [x] Add a Node handoff regression test for temp snapshot preparation
- [x] Switch from fallback logging to full viewer launch confidence once Rust runtime is stable
- [x] Add end-to-end handoff verification against the real Rust binary

## Current Lane Status

- [x] Projected phase: parked
- [x] Current item: Future shell polish only if Rust viewer launch UX changes again
- [x] Latest commit: `baa7b8e`
- [x] Latest event: bootstrap-insight · Lane 1 shell/handoff audit is already satisfied
- [x] Next expected phase: n/a
- [x] Next refill target: Re-activate only if plot launch/retry UX changes enough to justify fresh shell work
- [x] Latest note: Current plot-mode shell and handoff tests already align with the recovery baseline, so the truthful next phase is parked rather than pseudo-active.

## Refill Order

- [x] First refill target after the current item: remaining Controller items
- [x] Then tighten plot snapshot builder semantics for real 7d history and 5h band math inside `src/lib/plot/**`
- [ ] Only then touch future View polish if Rust viewer launch UX changes again
