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

## Current 4-Active-Lane Plan

- [x] The earlier handoff/chart/panel foundation rounds are integrated enough that the next 4a set can move beyond Lane 1-first refill.
- [x] Lane 1 is effectively exhausted for now:
  - keep it parked unless Rust viewer launch UX changes enough to justify fresh shell work
- [x] Lane 2 is promoted back into the next active set for runtime navigation and focus-flow work:
  - use it for left/right profile cycling plus panel-detail refresh behavior
- [x] Lane 3 stays active for chart-side focus compatibility:
  - keep chart rendering aligned with the richer focus/navigation behavior landing through Lane 2
- [x] Lane 4 stays active for richer compare/recommendation panel content:
  - use it to surface real routing recommendation details rather than only the first visible skeleton
- [x] Lane 5 enters the lane pool as the fourth active slot:
  - use it for README/operator-flow/run-instruction cleanup once the recovery baseline becomes the trusted local workflow
- [x] Runtime execution evidence tightened the next refill order:
  - Lane 2 owns the next seam because Lane 4 cannot finish compare-panel richness until `render/mod.rs` exposes compare payload
  - Lane 4 should remain blocked rather than pretending to implement against missing boundary data
  - Lane 5 can continue taking docs/test-only cleanup as long as it stays inside README and README regression ownership
- [x] Prefer the next queued work in this order:
  - Lane 2 -> render-boundary compare payload seam for Lane 4, then return to focus/navigation flow if another runtime item still exists
  - Lane 3 -> chart compatibility with richer focus and profile cycling
  - Lane 4 -> recommendation / compare-panel richness with stable visible copy after the Lane 2 seam lands
  - Lane 5 -> README and local run instructions for the recovery-baseline workflow, then shell/readme alignment if still docs-owned

## Runtime-State Note

- [x] Manual lane planning now diverges from the currently recorded lane journals for lanes 2-4.
- [x] Before dispatching the next 4a execution round, refresh or rewrite the lane journals so runtime tooling reflects the new manual active set instead of the stale recovery-branch dispatch state.
- [x] During the current 4a round, repeated probe evidence showed that clean worktrees plus stale `implementing` journals should trigger re-assignment rather than be treated as real active progress.

## Refill Rules

- [x] Refill from the same lane's next unchecked item before inventing cross-lane work.
- [x] If a lane item fails review, keep correction inside the same lane.
- [x] If a lane requires another lane's seam, cut a dependency item first rather than widening scope mid-flight.
