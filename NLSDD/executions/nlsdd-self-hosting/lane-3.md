# Lane 3 Plan - Rules and Communication

> Ownership family:
> `spec/NLSDD/operating-rules.md`, `spec/NLSDD/guardrails.md`, `spec/NLSDD/communication.md`
>
> NLSDD worktree: `.worktrees/nlsdd-lane-3-rules`
>
> Lane-local verification:
> `rg -n "active lane count|4LSDD|4 active lanes" spec/NLSDD`

## M - Operating Model

- [ ] Clarify that tracked scoreboard and lane-plan status surfaces are projection-only outputs, not independent writable state
- [ ] Define insight lifecycle and graduation rules for adopted global learnings versus execution-local blockers

## V - Communication Surface

- [ ] Align review-time guidance so `execution-insights` are inspected during or after NLSDD-stage review passes
- [ ] Clarify which reducer/read helpers are observational and must not rewrite tracked docs on read

## C - Guardrails

- [ ] Keep autopilot, probe, and execution-insights rules consistent with projection-only tracked surfaces

## Current Lane Status

- [x] Projected phase: parked
- [x] Current item: Lane-pool + active-cap rules alignment
- [x] Latest commit: `ef5f71e`
- [x] Latest event: parked · Lane 3 remains parked until a fresh documentation/spec gap appears.
- [x] Next expected phase: queued
- [x] Next refill target: Execution-level wording cleanup
- [x] Latest note: Lane 3 remains parked until a fresh documentation/spec gap appears.

## Refill Order

- [ ] First refill target: insight graduation and projection-only wording cleanup
- [ ] Then lane-creation guidance only if the new reducer/read model exposes another documentation gap
