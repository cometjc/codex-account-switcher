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

- [x] Lane 5: Plot Viewer Docs and Operator Flow
  - Plan: `NLSDD/executions/plot-mode/lane-5.md`
  - Worktree: `.worktrees/lane-5-docs`

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
- [x] Lane 2 fifth-round runtime navigation regression commit `1fd4db4` landed and passed spec + quality review.
- [x] Lane 5 first two docs/operator-flow commits `888e2d9` and `25ea3c1` landed with README regression coverage.
- [x] Lane 4 correction surfaced a real dependency: the render boundary still needs to expose compare recommendation/bottleneck payload, so the lane is blocked until Lane 2 lands that seam.
- [x] Lane 4 adopted-target emphasis is now also proven complete in `6bb1fba`; the lane should stay parked until a fresh panel-local follow-up exists rather than lingering as stale implementing.
- [x] Plot-mode reactivation landed on `main` in `5c2d643`: runtime shared state, visible 7d/5h chart rendering, and Summary / Compare refresh are now integrated on the mainline.
- [x] After the reactivation work landed, the execution no longer has an honest queued lane; Lane 2 / Lane 3 / Lane 4 should be parked until a fresh plot-mode follow-up appears.

## Current 4-Active-Lane Plan

- [x] The earlier handoff/chart/panel foundation rounds are integrated enough that the next 4a set can move beyond Lane 1-first refill.
- [x] Lane 1 is effectively exhausted for now:
  - keep it parked unless Rust viewer launch UX changes enough to justify fresh shell work
- [x] Lane 2 is the first lane to re-open:
  - use it for left/right profile cycling plus focus-state propagation so every render surface reads the same selected/current/focus truth
- [x] Lane 3 re-opens immediately after Lane 2:
  - replace scaffold copy with a real 7d curve plus 5h band renderer
- [x] Lane 4 re-opens behind Lane 2/3:
  - refresh Summary / Compare content from the real viewer state once runtime/chart slices stop being placeholders
- [x] Lane 5 stays parked by default:
  - wake it only if the reactivated viewer changes the trusted local run/recovery workflow enough to require docs/test wording updates
- [x] This reactivation round is now complete:
  - all five lanes are parked until a genuinely new plot-mode gap appears
- [x] Runtime execution evidence tightened the next refill order:
  - Lane 2 owns the next seam because the product gap is now runtime-state coherence, not another docs-only pass
  - Lane 3 should not pretend the chart is done while `chart.rs` still renders scaffold text
  - Lane 4 should consume the real state/chart output rather than invent new panel richness against placeholder data
  - Lane 5 should stay parked until visible operator behavior truly changes
- [x] Prefer the next queued work in this order:
  - Lane 2 -> runtime interaction / render-state coherence for selected profile and focus propagation
  - Lane 3 -> real 7d curve plus 5h band rendering once Lane 2's state contract is stable
  - Lane 4 -> summary/compare refresh against the richer runtime/chart state
  - Lane 5 -> docs/run-instruction updates only if the visible viewer behavior changed enough to make current wording stale

## Runtime-State Note

- [x] The older tracked execution converged to all parked lanes even though the product gap remained open.
- [x] Before dispatching the reactivated round, rewrite the tracked lane phases so runtime tooling sees real queued work instead of replaying the no-dispatch plateau.
- [x] The current replan exists specifically because a scaffold-only Rust viewer is not a truthful completion state for plot-mode.

## Refill Rules

- [x] Refill from the same lane's next unchecked item before inventing cross-lane work.
- [x] If a lane item fails review, keep correction inside the same lane.
- [x] If a lane requires another lane's seam, cut a dependency item first rather than widening scope mid-flight.
