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

- [ ] Add regression coverage for linked-worktree / non-canonical-root reducer replay
- [ ] Add regression coverage for clearing stale `Current item` / `Next refill target` after parked or noop transitions

## V - Read-Loop Safety

- [ ] Add regression coverage proving `review` / `schedule` / `dispatch-plan` keep tracked files unchanged on read
- [ ] Add insight-summary coverage separating actionable execution issues from durable adopted learnings

## C - Verification Harness

- [ ] Keep the verification path fast enough to run as a lane-local smoke check
- [ ] Preserve existing scoreboard/schedule cross-check coverage while extending reducer/read-loop regressions

## Current Lane Status

- [x] Projected phase: parked
- [x] Current item: Wait for a fresh regression/CLI surface item after the cross-check coverage landed
- [x] Latest commit: `655aa39`
- [x] Latest event: parked · Lane 4 is parked after the stale-field clearing regression landed cleanly.
- [x] Next expected phase: n/a
- [x] Next refill target: Re-open only if a fresh regression/CLI surface gap appears beyond the accepted cross-check coverage
- [x] Latest note: Lane 4 is parked after the stale-field clearing regression landed cleanly.

## Refill Order

- [ ] First refill target: insight summary and supersession regression coverage
- [ ] Then deeper message-helper coverage only if the new reducer/read model exposes another coordinator bottleneck
