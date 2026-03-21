# Plot Mode NLSDD Overview

> Goal: keep plot-mode work inside stable, non-overlapping NLSDD lanes. This execution uses a 4-thread active cap, but the lane pool may grow beyond 4 total lanes as needed.

## Scheduling Model

- [x] Active subagent cap configured to 4 for this execution
- [x] Lane pool may grow beyond 4 when new non-overlapping work families appear
- [x] Only four lanes may occupy active slots at once; extra lanes stay queued or parked until a slot opens
- [x] When a slot opens, the coordinator can promote the next eligible queued lane without redefining the execution

## Lanes

- [x] Lane 1: Node Contract and Handoff
  - Plan: `NLSDD/executions/plot-mode/lane-1.md`
  - Worktree: `.worktrees/lane-1-node`

- [x] Lane 2: Rust Runtime and State
  - Plan: `NLSDD/executions/plot-mode/lane-2.md`
  - Worktree: `.worktrees/lane-2-runtime`

- [x] Lane 3: Rust Chart Surface
  - Plan: `NLSDD/executions/plot-mode/lane-3.md`
  - Worktree: `.worktrees/lane-3-chart`

- [x] Lane 4: Rust Panels, Docs, and Regression Surfaces
  - Plan: `NLSDD/executions/plot-mode/lane-4.md`
  - Worktree: `.worktrees/lane-4-panels`

## Current Progress

- [x] Lane 1 first-round real Rust viewer handoff verification landed and passed spec + quality review.
- [x] Lane 2 first-round drawable panel boundary seam landed and passed spec + quality review.
- [x] Lane 3 first-round `ChartViewModel` extraction landed and passed spec + quality review.
- [x] Lane 4 first-round visible Summary / Compare panel surface landed and passed spec + quality review.
- [x] Second-round implementer commits now exist for lanes 1-4 and are tracked in `NLSDD/scoreboard.md`.

## Refill Rules

- [x] Refill from the same lane's next unchecked item before inventing cross-lane work.
- [x] If a lane item fails review, keep correction inside the same lane.
- [x] If a lane requires another lane's seam, cut a dependency item first rather than widening scope mid-flight.
