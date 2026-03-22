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

- [ ] Add a shared fallback path that prefers runtime scoreboard artifacts but degrades to tracked scoreboard rows when parsing fails
- [ ] Keep scoreboard-derived scheduling fields stable even when the runtime scoreboard is absent, empty, or malformed

## V - Scoreboard Surface

- [ ] Keep commit-intake and coordinator reads usable without forcing a runtime scoreboard regeneration step
- [ ] Preserve coordinator-authored tracked intent fields while degraded-mode reads fall back to tracked scoreboard rows

## C - Coordinator Flow

- [ ] Make the runtime-scoreboard boundary truthful for coordinator read loops instead of crashing on auxiliary-surface drift

## Current Lane Status

- [x] Projected phase: parked
- [x] Current item: Self-hosting scoreboard rows and schedule-facing refresh
- [x] Latest commit: `1613f3c`
- [x] Latest event: parked · nlsdd-go: park self-hosting after dev-flow improvement plan completed
- [x] Next expected phase: n/a
- [x] Next refill target: Schedule-facing scoreboard wording polish
- [x] Latest note: nlsdd-go: park self-hosting after dev-flow improvement plan completed

## Refill Order

- [ ] First refill target: fail-soft runtime scoreboard loading for coordinator / commit intake
- [ ] Then surface extra derived hints only if the read-only refresh flow still needs them
