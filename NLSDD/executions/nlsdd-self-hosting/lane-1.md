# Lane 1 Plan - Scheduler Core

> Ownership family:
> `NLSDD/scripts/nlsdd-lib.cjs`, `NLSDD/scripts/nlsdd-suggest-schedule.cjs`
>
> NLSDD worktree: `.worktrees/nlsdd-lane-1-scheduler`
>
> Lane-local verification:
> `node --test tests/nlsdd-automation.test.js`
> `node NLSDD/scripts/nlsdd-suggest-schedule.cjs --execution nlsdd-self-hosting --json`

## M - Scheduling Semantics

- [ ] Normalize scoreboard rows into explicit scheduling phases without conflating manual and derived fields
- [ ] Ensure active-thread counting is stable for `implementing`, review-pending, and correction phases

## V - Schedule Output

- [ ] Render schedule suggestions that prefer `refill-ready` lanes before `queued` lanes
- [ ] Surface enough schedule metadata for coordinator-side dispatch decisions

## C - CLI Glue

- [ ] Keep `nlsdd:schedule:suggest` usable for executions with lane pools larger than the active subagent cap

## Current Lane Status

- [x] Projected phase: parked
- [x] Current item: Multi-lane / 4-thread schedule helper
- [x] Latest commit: `n/a`
- [x] Latest event: parked · nlsdd-go: park self-hosting after dev-flow improvement plan completed
- [x] Next expected phase: n/a
- [x] Next refill target: Scheduler edge cases
- [x] Latest note: nlsdd-go: park self-hosting after dev-flow improvement plan completed

## Refill Order

- [ ] First refill target: scheduler edge cases
- [ ] Then widen into phase heuristics only if the tests expose a real gap
