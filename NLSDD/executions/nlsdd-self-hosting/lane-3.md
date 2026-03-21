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

- [ ] Active lane item: finish documentation alignment for self-hosting NLSDD scheduling in the initial active set

## Refill Order

- [ ] First refill target: execution-level wording cleanup
- [ ] Then lane-creation guidance if the scheduler rollout proves stable
