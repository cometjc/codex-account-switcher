# Lane 3 Plan - Rules and Communication

> Ownership family:
> `spec/NLSDD/operating-rules.md`, `spec/NLSDD/guardrails.md`, `spec/NLSDD/communication.md`
>
> NLSDD worktree: `.worktrees/nlsdd-lane-3-rules`
>
> Lane-local verification:
> `rg -n "active lane count|4LSDD|4 active lanes" spec/NLSDD`

## M - Operating Model

- [ ] Rewrite remaining fixed-4-lane wording into lane-pool + active-cap language
- [ ] Make it explicit that queued lanes may exist without consuming active thread slots

## V - Communication Surface

- [ ] Align reviewer / implementer templates with the new scheduling model
- [ ] Clarify how queued lanes enter the active set when a slot opens

## C - Guardrails

- [ ] Keep autopilot refill, probe, and blocker rules consistent with a lane pool larger than the active cap

## Current Lane Status

- [x] Projected phase: parked
- [x] Current item: Lane-pool + active-cap rules alignment
- [x] Latest commit: `n/a`
- [x] Latest event: bootstrap-insight · Lane 3 rules stream converged to honest no-op
- [x] Next expected phase: n/a
- [x] Next refill target: Execution-level wording cleanup
- [x] Latest note: After queued-lane promotion and active-cap clarifications landed, there was no further reviewable spec-only step. Parked Lane 3 instead of inventing wording churn.

## Refill Order

- [ ] First refill target: execution-level wording cleanup
- [ ] Then lane-creation guidance if the scheduler rollout proves stable
