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

## Current 4-Active-Lane Plan

- [x] Keep plot-mode on a 4-lane active set while the current second-round correction / re-review loop is still open for lanes 1-4.
- [x] Treat Lane 1, Lane 3, and Lane 4 as the post-correction refill priorities:
  - Lane 1 -> tighten plot snapshot builder semantics for real 7d history and 5h band math
  - Lane 3 -> add 5h band, axis labels, and unavailable-band fallback
  - Lane 4 -> add panel-specific regression coverage for the visible Summary / Compare surface
- [x] Treat Lane 2 as conditional after the current correction closes:
  - keep the lane active only while the current correction/re-review loop remains open
  - park it after closure unless another lane proves the stronger nested `usage` decode path is necessary
- [x] Do not invent Lane 5 yet; exhaust the current 4-lane pool first, because Lane 1/3/4 still have clearly reviewable lane-local refill items.

## Refill Rules

- [x] Refill from the same lane's next unchecked item before inventing cross-lane work.
- [x] If a lane item fails review, keep correction inside the same lane.
- [x] If a lane requires another lane's seam, cut a dependency item first rather than widening scope mid-flight.
