# Lane 4 Plan - Regression and CLI Surface

> Ownership family:
> `tests/nlsdd-automation.test.js`, future `tests/nlsdd-schedule.test.js`
>
> NLSDD worktree: `.worktrees/nlsdd-lane-4-tests`
>
> Lane-local verification:
> `node --test tests/nlsdd-automation.test.js`
>
> `node NLSDD/scripts/nlsdd-suggest-schedule.cjs --execution nlsdd-self-hosting`

## M - Reducer Regression

- [ ] Add regression coverage proving coordinator / commit-intake survive a missing or malformed runtime scoreboard
- [ ] Add regression coverage for stale-implementing reconciliation back to honest parked or noop truth

## V - Read-Loop Safety

- [ ] Add regression coverage for syncing `## Current Lane Status` sections without rewriting the rest of the lane plan
- [ ] Keep read-loop coverage honest about which helpers only observe state and which helpers intentionally sync tracked status sections

## C - Verification Harness

- [ ] Keep the verification path fast enough for lane-local smoke checks while adding fail-soft and sync-helper scenarios
- [ ] Preserve existing scoreboard/schedule cross-check coverage while extending degraded-mode and execution-truth regressions

## Current Lane Status

- [x] Projected phase: implementing
- [x] Current item: Wait for a fresh regression/CLI surface item after the cross-check coverage landed
- [x] Latest commit: `655aa39`
- [x] Latest event: pass · Cycle promoted Lane 4 from queued to implementing for Add regression coverage proving coordinator / commit-intake survive a missing or malformed runtime scoreboard.
- [x] Next expected phase: spec-review-pending
- [x] Next refill target: Re-open only if a fresh regression/CLI surface gap appears beyond the accepted cross-check coverage
- [x] Latest note: Cycle promoted Lane 4 from queued to implementing for Add regression coverage proving coordinator / commit-intake survive a missing or malformed runtime scoreboard.

## Refill Order

- [ ] First refill target: fail-soft coordinator and execution-truth sync regression coverage
- [ ] Then deeper message-helper coverage only if the new reducer/read model exposes another coordinator bottleneck
