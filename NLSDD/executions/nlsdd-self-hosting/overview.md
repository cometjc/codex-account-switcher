# NLSDD Self-Hosting Overview

> Goal: use NLSDD to continue developing NLSDD itself. This execution maintains a 4-thread active cap while allowing the lane pool to grow beyond 4 whenever new non-overlapping work families appear.

## Scheduling Model

- [x] Active subagent cap configured to 4 for this execution
- [x] Lane pool may exceed 4 as long as queued lanes do not overlap active write scopes
- [x] Only 4 lanes may consume active subagent slots at one time
- [x] Extra lanes remain `queued`, `refill-ready`, `blocked`, or `parked` in the scoreboard until a slot opens

## Lane Pool

- [x] Lane 1: Scheduler Core
  - Plan: `NLSDD/executions/nlsdd-self-hosting/lane-1.md`
  - Live status: see lane plan `Current Lane Status` / `NLSDD/scoreboard.md`

- [x] Lane 2: Scoreboard Integration
  - Plan: `NLSDD/executions/nlsdd-self-hosting/lane-2.md`
  - Focus: runtime-scoreboard fail-soft work from the 2026-03-22 dev-flow improvement plan

- [x] Lane 3: Rules and Communication
  - Plan: `NLSDD/executions/nlsdd-self-hosting/lane-3.md`
  - Focus: projection-only / sync-path wording updates from the 2026-03-22 dev-flow improvement plan

- [x] Lane 4: Regression and CLI Surface
  - Plan: `NLSDD/executions/nlsdd-self-hosting/lane-4.md`
  - Focus: fail-soft and execution-truth regression coverage from the 2026-03-22 dev-flow improvement plan

- [x] Lane 5: Plot-Mode Execution Migration
  - Plan: `NLSDD/executions/nlsdd-self-hosting/lane-5.md`
  - Live status: see lane plan `Current Lane Status` / `NLSDD/scoreboard.md`

- [x] Lane 6: Self-Hosting Follow-up and Coordinator Ergonomics
  - Plan: `NLSDD/executions/nlsdd-self-hosting/lane-6.md`
  - Live status: see lane plan `Current Lane Status` / `NLSDD/scoreboard.md`

- [x] Lane 7: NLSDD Meta-Optimization
  - Plan: `NLSDD/executions/nlsdd-self-hosting/lane-7.md`
  - Focus: execution-truth sync helper work from the 2026-03-22 dev-flow improvement plan

## Refill Rules

- [x] Keep 4 active subagent slots saturated whenever 4 safe lane items exist
- [x] Refill from the same lane first when the next unchecked item stays inside that lane's ownership
- [x] If an active lane is exhausted or blocked, consume the next `queued` lane from this execution before inventing ad-hoc work
- [x] Coordinator-owned tracking files remain outside implementer scope
