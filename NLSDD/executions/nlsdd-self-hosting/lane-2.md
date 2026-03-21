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

- [ ] Keep scoreboard derived columns stable when runtime reducers replay from worktree-local execution roots
- [ ] Reflect runtime scoreboard data without overwriting coordinator-owned tracked intent fields

## V - Scoreboard Surface

- [ ] Document which tracked/manual scoreboard fields remain authoritative versus runtime-derived fields
- [ ] Keep runtime refresh usable for read loops without mutating tracked scoreboard rows

## C - Coordinator Flow

- [ ] Keep refresh + schedule output usable as a coordinator-side read loop after tracked/runtime separation lands

## Current Lane Status

- [x] Projected phase: parked
- [x] Current item: Self-hosting scoreboard rows and schedule-facing refresh
- [x] Latest commit: `1613f3c`
- [x] Latest event: parked · Lane 2 scoreboard integration remains stable on the refreshed baseline; keep it parked until a concrete new gap appears.
- [x] Next expected phase: n/a
- [x] Next refill target: Schedule-facing scoreboard wording polish
- [x] Latest note: Lane 2 scoreboard integration remains stable on the refreshed baseline; keep it parked until a concrete new gap appears.

## Refill Order

- [ ] First refill target: projection-only tracked scoreboard wording and refresh behavior
- [ ] Then surface extra derived hints only if the read-only refresh flow still needs them
