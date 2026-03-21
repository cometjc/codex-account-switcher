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
- [x] Second-round implementer commits for lanes 1-4 were reviewed and either superseded by a third-round refill or parked as no-op evidence.
- [x] Lane 1 third-round snapshot semantics commit `baa7b8e` landed and passed spec + quality review.
- [x] Lane 3 third-round chart surface commit `585317d` landed and passed spec + quality review.
- [x] Lane 4 third-round panel regression commit `abd8b10` landed and passed spec + quality review.
- [x] Lane 3 fourth-round chart focus wording commit `35c8351` landed and passed review.
- [x] Lane 4 fourth-round panel field-mapping refactor commit `b24f12a` landed with stable visible output.
- [x] Lane 2 correction closed as a no-op: stronger nested `usage` decoding is still not required, so the lane is now parked by default.
- [x] Accepted Lane 1 / Lane 3 / Lane 4 stacks now merge cleanly on top of the shared plot baseline `d19d319` in `.worktrees/plot-integration-base`.

## Current 4-Active-Lane Plan

- [x] The third-round 4-active-lane refill has now closed for Lane 1, Lane 3, and Lane 4 with accepted lane-local commits.
- [x] Lane 2 no longer needs an active slot by default:
  - keep it parked unless chart/panels work later proves that nested `usage` decoding became a real runtime blocker
- [x] Do not invent Lane 5 just to keep four slots busy; refill only when a lane has a real reviewable next item.
- [x] Prefer the next queued work in this order:
  - Lane 3 -> later left/right profile cycling or richer focus behavior only if plot UX still needs it
  - Lane 4 -> README/doc polish only if panel wording changes again
  - Lane 1 -> future shell polish only if Rust viewer launch UX changes again

## Refill Rules

- [x] Refill from the same lane's next unchecked item before inventing cross-lane work.
- [x] If a lane item fails review, keep correction inside the same lane.
- [x] If a lane requires another lane's seam, cut a dependency item first rather than widening scope mid-flight.
