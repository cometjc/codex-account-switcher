# NLSDD Operating Rules

> NLSDD = N-Lane Subagent-Driven Development. This repo uses NLSDD as its native multi-agent workflow. `N` is configurable per execution. Lane pool size may exceed the number of simultaneously active subagents, and queued lanes may wait for an open active slot.

## Core Workflow

- Every active task belongs to exactly one execution, one lane, and one lane item.
- Every lane uses its own dedicated worktree; do not run multiple lanes inside one shared dirty worktree.
- Every lane item must produce its own lane-item commit before review begins.
- Every lane item must pass two review gates in order:
  - spec compliance review
  - code quality review
- A lane item is complete only after both review gates pass.

## Execution Requirements

- Every execution must define:
  - execution id
  - lane pool size
  - active subagent cap
  - lane ownership families
  - lane worktree naming convention
  - lane-local verification commands
- Every execution must keep its runtime lane plans under `NLSDD/executions/<execution-id>/`.
- Every execution may keep execution-aware lane runtime state under `NLSDD/state/<execution-id>/`.
- Every execution may keep an append-only execution insights journal under `NLSDD/state/<execution-id>/execution-insights.ndjson`.
- Every execution must have one canonical tracked row set in `NLSDD/scoreboard.md`.
- Auto-refreshed runtime scoreboard output should be written under `NLSDD/state/`, not back into the tracked scoreboard.
- The tracked scoreboard should keep only coordinator-owned manual fields; full auto-derived lane state belongs in the runtime scoreboard.
- Runtime scoreboard rows may contain both manual coordinator fields and auto-derived fields; automation may suggest state, but the coordinator remains the decision-maker for dispatch.
- Not every lane row has to consume an active thread slot at all times; queued or parked lanes may remain visible in the scoreboard until a slot opens, then the coordinator can promote the next eligible queued lane into that slot.
- Runtime tooling must resolve the canonical repo root even when invoked from a linked worktree, so lane plans, worktrees, and state files always point back to the same execution root.
- When the coordinator redefines an execution's active set, it should update the tracked scoreboard and the lane journals as one atomic replan step, preferably through `nlsdd-replan-active-set`, rather than editing only the manual scoreboard first.

## Lane Worktree Rules

- Reuse the same worktree for later items in that lane unless the lane is retired.
- Reviewers must inspect the lane item's commit diff, not the lane worktree's total dirty state.
- If a lane worktree accumulates unrelated drift, stop and clean that lane before assigning more work there.
- Worktree-local build outputs and caches must be treated as noise, not as lane-item scope.

## Lane Item Rules

- A lane item must be reviewable in one diff:
  - clear goal
  - explicit write set
  - explicit verification
  - no hidden dependency on another lane's unimplemented boundary
- Prefer 1-2 responsibilities per lane item.
- When one lane-local MVC step is implemented and its planned verification passes, treat that MVC step as commit-worthy by default; do not keep stacking multiple completed MVC steps in one uncommitted worktree state.
- In this repo's default NLSDD flow, sub-agents should not finalize `git commit` themselves. They should hand off the completed MVC step, verification, and intended commit summary through `READY_TO_COMMIT`, and the coordinator should create the lane-item commit on their behalf.
- Only use sub-agent self-commit when the lane item explicitly says self-commit is allowed and the environment is known not to gate `git commit`.
- If a task depends on another lane expanding a seam or boundary, split that dependency into its own lane item first.
- Implementers do not update coordinator-owned tracking files unless the task explicitly says so.

## Coordinator-Owned Tracking

- The coordinator owns:
  - `tasks/todo.md`
  - roadmap status updates
  - execution and lane checklist updates
  - `NLSDD/scoreboard.md`
  - `NLSDD/state/<execution-id>/lane-<n>.json`
  - `NLSDD/state/scoreboard.runtime.md`
  - cross-lane lessons in `tasks/lessons.md`
- Implementers and reviewers should not "helpfully" update those files as part of feature work.
- Auto-refresh tooling may rewrite the scoreboard's derived columns, but must not overwrite manual intent fields such as `Current item`, `Phase`, or `Blocked by`.
- Runtime scoreboard generation may expand the tracked scoreboard into a richer derived table, but the tracked scoreboard itself should stay manual-only.
- Lane journal files are the execution-aware runtime source of truth for phase transitions, latest commit metadata, and next expected gate when those details need to survive across probes, reviews, and worktree-local invocations.
- Execution insights journals are the append-only runtime source for sub-agent suggestions, coordinator observations, and improvement opportunities discovered during execution. They complement lane journals and must not replace current lane phase/state.

## Review Rules

- Spec reviewers review only the requested lane item and its commit diff.
- Spec review checks:
  - requested behavior exists
  - no missing requirements
  - no unrequested scope
  - write-set compliance
- Code quality reviewers run only after spec review passes.
- Code quality review checks:
  - file responsibility and interface clarity
  - maintainability
  - test quality
  - accidental cross-lane coupling

## Autopilot Refill Rule

- When a lane item reaches `quality PASS`, try to refill from the next unchecked item in that same lane first.
- Keep the configured active subagent cap saturated, not the full lane pool.
- Do not wait for full tracking-file updates before dispatching the next non-overlapping lane item into an open thread slot.
- NLSDD automation may compute `refill-ready` and suggest the next lane-local item, but dispatch still happens explicitly through the coordinator.
- Only stop refilling a lane when:
  - the lane is genuinely exhausted
  - the next item is blocked by another lane
  - the next item would overlap an active lane's ownership
  - all active thread slots are already full

## Blockers and Borrowed Seams

- If an implementer cannot complete a lane item inside its write set, it must report `BLOCKED` or `NEEDS_CONTEXT`.
- Do not silently expand scope.
- If the blocker is real, the coordinator must choose one:
  - create a new dependency item in the owning lane
  - explicitly loan a borrowable seam for one lane item
  - re-cut the lane item to match the actual dependency graph
- Borrowed seams must be written down in the execution's lane plan before implementation resumes.

## Default Operating Sequence

1. Maintain up to the configured active subagent cap, even when the execution has more lanes than active threads.
2. Pick the next unchecked item from one execution lane plan that either owns the just-freed slot or is next in the queued lane pool.
3. Dispatch one implementer with the full lane-item spec.
4. Wait for implementer status.
5. If the planned MVC step is finished and verification passes, create the lane-item commit immediately. In this repo's default NLSDD path, the implementer should report `READY_TO_COMMIT` and the coordinator should create that commit. Only lanes that are explicitly marked as self-commit-safe should end with the sub-agent running `git commit` directly.
6. If `DONE` or `DONE_WITH_CONCERNS`, run spec review against the lane-item commit diff.
7. If spec review fails, return to the same implementer for correction and re-review.
8. If spec review passes, run code quality review against the same diff.
9. If quality review fails, return to the same implementer for correction and re-review.
10. After both pass, coordinator marks the lane as `refill-ready`, updates tracking docs in batch, and either refills the same lane or allocates the freed slot to another queued lane.

## Lane Journal Contract

- Each lane journal file should record, at minimum:
  - `execution`
  - `lane`
  - `phase`
  - `expectedNextPhase`
  - `latestCommit`
  - `lastReviewerResult`
  - `lastVerification`
  - `blockedBy`
  - `updatedAt`
- Scoreboard refresh, schedule suggestion, and lane probes should prefer lane journal state over cross-thread heuristics when the journal exists.
- If lane plans or worktrees go missing, automation should degrade explicitly rather than silently reusing stale derived values.

## Execution Insights Contract

- Each execution insights entry should record, at minimum:
  - `timestamp`
  - `execution`
  - `lane` or `global`
  - `source` (`subagent` or `coordinator`)
  - `kind` (`suggestion`, `observed-issue`, or `improvement-opportunity`)
  - `status` (`open`, `adopted`, `rejected`, or `resolved`)
  - `summary`
- Optional fields may include:
  - `detail`
  - `relatedLane`
  - `relatedCommit`
  - `relatedAgent`
  - `recordedBy`
- Use execution insights to preserve dynamic execution learnings that do not fit cleanly into lane state, such as blocker remediation suggestions, coordination drift, or workflow optimization ideas.

## Current Repo Defaults

- `plot-mode` is the first full NLSDD execution.
- The existing lane worktrees remain valid, but their source of truth moves under `NLSDD/executions/plot-mode/`.
- Future multi-agent streams should start from NLSDD directly rather than cloning earlier fixed-4-lane naming.
