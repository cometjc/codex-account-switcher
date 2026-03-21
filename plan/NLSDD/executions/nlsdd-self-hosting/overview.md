# NLSDD Self-Hosting Overview

> Goal: use NLSDD to continue developing NLSDD itself. This execution maintains a 4-thread active cap while allowing the lane pool to grow beyond 4 whenever new non-overlapping work families appear.

## Scheduling Model

- [x] Active subagent cap configured to 4 for this execution
- [x] Lane pool may exceed 4 as long as queued lanes do not overlap active write scopes
- [x] Only 4 lanes may consume active subagent slots at one time
- [x] Extra lanes remain `queued`, `refill-ready`, `blocked`, or `parked` in the scoreboard until a slot opens

## Lane Pool

- [x] Lane 1: Scheduler Core
  - Plan: `plan/NLSDD/executions/nlsdd-self-hosting/lane-1.md`
  - Status: initial active set

- [x] Lane 2: Scoreboard Integration
  - Plan: `plan/NLSDD/executions/nlsdd-self-hosting/lane-2.md`
  - Status: initial active set

- [x] Lane 3: Rules and Communication
  - Plan: `plan/NLSDD/executions/nlsdd-self-hosting/lane-3.md`
  - Status: initial active set

- [x] Lane 4: Regression and CLI Surface
  - Plan: `plan/NLSDD/executions/nlsdd-self-hosting/lane-4.md`
  - Status: initial active set

- [x] Lane 5: Plot-Mode Execution Migration
  - Plan: `plan/NLSDD/executions/nlsdd-self-hosting/lane-5.md`
  - Status: queued follow-up lane

- [x] Lane 6: Self-Hosting Follow-up and Coordinator Ergonomics
  - Plan: `plan/NLSDD/executions/nlsdd-self-hosting/lane-6.md`
  - Status: queued follow-up lane

## Refill Rules

- [x] Keep 4 active subagent slots saturated whenever 4 safe lane items exist
- [x] Refill from the same lane first when the next unchecked item stays inside that lane's ownership
- [x] If an active lane is exhausted or blocked, consume the next `queued` lane from this execution before inventing ad-hoc work
- [x] Coordinator-owned tracking files remain outside implementer scope
