# Lane 2 Plan - Scoreboard Integration

> Ownership family:
> `NLSDD/scripts/nlsdd-refresh-scoreboard.cjs`, `NLSDD/scoreboard.md`
>
> NLSDD worktree: `.worktrees/nlsdd-lane-2-scoreboard`
>
> Lane-local verification:
> `npm run nlsdd:scoreboard:refresh`
> `node NLSDD/scripts/nlsdd-suggest-schedule.cjs --execution nlsdd-self-hosting`

## M - Derived Columns

- [ ] Keep scoreboard derived columns stable when an execution has more lanes than active threads
- [ ] Reflect queued lanes and available slots without overwriting coordinator-owned intent fields

## V - Scoreboard Surface

- [ ] Document which scoreboard fields are used by schedule suggestion
- [ ] Add self-hosting scoreboard rows for `nlsdd-self-hosting`

## C - Coordinator Flow

- [ ] Make refresh + schedule output usable as a single coordinator-side loop for dispatch decisions

## Current Lane Status

- [ ] Active lane item: connect scoreboard rows to the new multi-lane schedule helper in the initial active set

## Refill Order

- [ ] First refill target: schedule-facing scoreboard wording polish
- [ ] Then surface extra derived hints only if the first loop needs them
