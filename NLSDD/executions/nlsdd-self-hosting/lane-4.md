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

- [ ] Add fixture coverage for lane pools larger than the active thread cap
- [ ] Keep test fixtures readable enough for future NLSDD executions to copy

## V - CLI Regression

- [ ] Add schedule CLI coverage for human-readable and JSON output
- [ ] Verify refill-ready lanes sort ahead of queued lanes

## C - Verification Harness

- [ ] Keep the verification path fast enough to run as a lane-local smoke check

## Current Lane Status

- [ ] Active lane item: add schedule regression coverage for the first multi-lane self-hosting execution in the initial active set

## Refill Order

- [ ] First refill target: scoreboard/schedule cross-check coverage
- [ ] Then deeper message-helper coverage if it becomes the next bottleneck
