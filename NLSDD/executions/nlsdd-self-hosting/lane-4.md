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

## M - Fixture Coverage

- [x] Add fixture coverage for lane pools larger than the active thread cap
- [x] Keep test fixtures readable enough for future NLSDD executions to copy

## V - CLI Regression

- [x] Add schedule CLI coverage for human-readable and JSON output
- [x] Verify refill-ready lanes sort ahead of queued lanes

## C - Verification Harness

- [x] Keep the verification path fast enough to run as a lane-local smoke check
- [x] Add scoreboard/schedule cross-check coverage so runtime and tracked scheduling surfaces stay aligned

## Current Lane Status

- [x] Projected phase: parked
- [x] Current item: Wait for a fresh regression/CLI surface item after the cross-check coverage landed
- [x] Latest commit: `6d6c4e8`
- [x] Latest event: state-update · Lane 4 cross-check coverage landed, so the lane is parked until a fresh regression/CLI item appears
- [x] Next expected phase: queued
- [x] Next refill target: Re-open only if a fresh regression/CLI surface gap appears beyond the accepted cross-check coverage
- [x] Latest note: Commit 6d6c4e8 completed the schedule CLI smoke and scoreboard/schedule cross-check coverage, so the honest next phase is parked rather than pseudo-implementing.

## Refill Order

- [ ] First refill target: a fresh regression/CLI surface gap beyond the accepted cross-check coverage
- [ ] Then deeper message-helper coverage only if that new gap exposes a real coordinator bottleneck
