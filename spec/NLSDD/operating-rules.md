# NLSDD Operating Rules

> NLSDD = N-Lane Subagent-Driven Development. This repo now routes NLSDD orchestration through a repo-local central executor (`.nlsdd/executor.sqlite` + `NLSDD/scripts/nlsdd-executor.cjs`). Legacy markdown/json/ndjson surfaces may still exist during migration, but they are not the canonical write path.

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
- Every execution must live in the executor database, not in free-floating tracked markdown.
- `plan/` should be empty before dispatch starts; any remaining plan file must be imported into the executor first.
- Executor SQLite is the only state-changing write interface for plan state, lane state, assignment state, review state, and result state.
- `NLSDD/scoreboard.md`, `NLSDD/state/*`, and lane `Current Lane Status` sections are legacy migration surfaces only. They may be rendered for humans during transition, but they are not allowed to become a second writable truth.
- NLSDD may expose a single dispatch-cycle helper that performs the deterministic coordinator work in one pass: reconcile stale implementing lanes, refresh runtime state, promote the next dispatchable lanes according to the tracked plan, and report the resulting scheduling status.
- When the deterministic output needs to become real subagent work, NLSDD may expose a launch helper that wraps the dispatch cycle and emits coordinator-ready implementer assignment bundles for each newly promoted lane.
- NLSDD may also expose a review-loop driver that inspects lane state and emits the next coordinator action bundle for `spec-review-pending`, `quality-review-pending`, `correction`, and `coordinator-commit-pending` lanes.
- NLSDD may expose a READY_TO_COMMIT intake helper that turns `coordinator-commit-pending` lane state into a normalized commit bundle containing proposed title/body, verification already completed, scope, and next expected phase.
- NLSDD may expose a single-pass coordinator loop that composes dispatch-cycle, launch, review-loop, and READY_TO_COMMIT intake results into one scheduling snapshot for the current execution.
- NLSDD may expose a dispatch-plan helper that turns the coordinator loop output into a prioritized action queue for the main agent, so the main agent can execute the next low-judgment steps without recomputing priority order by hand.
- Not every lane row has to consume an active thread slot at all times; queued or parked lanes may remain visible in the scoreboard until a slot opens, then the coordinator can promote the next eligible queued lane into that slot.
- A healthy execution should avoid collapsing into a single serial critical lane while the other active slots are effectively idle; when the remaining work starts converging that way, coordinator should re-plan instead of pretending the active cap is still meaningfully saturated.
- When one execution no longer contains enough honest non-overlapping work to keep multiple slots productive, coordinator should prefer cutting 1-2 new independent lanes or advancing 2-3 plans/executions in parallel over keeping several slots pseudo-active behind one blocker.
- Scheduler/probe tooling should detect stale `implementing` lanes from worktree truth and stop counting them as active thread consumers once the lane is clean at the same committed `HEAD`.
- Runtime tooling must resolve the canonical repo root even when invoked from a linked worktree, so lane plans, worktrees, and state files always point back to the same execution root.
- When the coordinator redefines an execution's active set, it should do so through executor state only; any human-readable scoreboard or lane rendering should be regenerated from that state, never hand-edited first.
- When accepted work lands and a lane remains stale, reconcile it through executor state and branch/worktree truth, not by patching markdown projections.

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
  - executor plan / execution / lane / review state
  - any optional rendered human-readable summaries
  - cross-lane lessons in `tasks/lessons.md`
- Implementers and reviewers should not "helpfully" update those files as part of feature work.
- Optional renderers may rewrite human-readable summaries, but they must never bypass executor state.
- Result branches, assignments, review outcomes, and integration decisions must be recorded in executor state instead of lane journals or tracked scoreboard rows.
- Treat execution insights as three lifecycle classes:
  - actionable execution-local insights: open blockers, suggestions, no-op findings, and workflow issues that still matter to the current execution
  - adopted durable global learnings: stable, reusable learnings that should graduate into the appropriate spec or rule file, and usually into tracked lessons as well, before the runtime copy is marked resolved
  - resolved history: closed, rejected, or superseded runtime entries that remain as audit trail but should no longer drive active dispatch decisions
- Adopted durable global learnings should graduate out of the runtime journal into the appropriate spec or rule file once they are stable and reusable; after that, the runtime copy should be resolved so the journal stays a history of what was learned, not a second writable truth.

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
- Treat single-lane convergence as a smell, not a success condition. If 3 slots are only waiting on 1 lane to finish, pause and re-cut the work so at least 2-3 independent fronts can move again, or explicitly shift spare slots to other plans/executions.
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
11. When enough of the sequence is deterministic, coordinator may run a dispatch-cycle helper to reconcile stale states and promote the next safe queued/refill-ready lanes in one command, then use the returned dispatch status to drive actual agent assignment.

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
- When the execution event log exists, lane journals are reducer outputs rather than primary coordinator-authored state.

## Lane Handoff Envelope Contract

- A canonical lane handoff envelope should include, at minimum:
  - `execution`
  - `lane`
  - `role`
  - `eventType`
  - `summary`
  - `timestamp`
- State-changing envelopes should also include, when applicable:
  - `phaseBefore`
  - `phaseAfter`
  - `currentItem`
  - `nextRefillTarget`
  - `relatedCommit`
  - `verification`
  - `nextExpectedPhase`
  - `blockedBy`
  - `proposedCommitTitle`
  - `proposedCommitBody`
  - `insights[]`
- NLSDD sub-agents should return this envelope shape directly instead of free-form status text once the lane template requires envelope-only handoff.
- Coordinator-side wrappers may still accept legacy state/insight arguments for compatibility, but they must translate them into the canonical envelope before mutating any NLSDD surface.

## Execution Insights Contract

- Each execution insights entry should record, at minimum:
  - `timestamp`
  - `execution`
  - `lane` or `global`
  - `source` (`subagent` or `coordinator`)
  - `kind` (`suggestion`, `observed-issue`, `improvement-opportunity`, `noop-finding`, `blocker`, or `resolved-blocker`)
  - `status` (`open`, `adopted`, `rejected`, or `resolved`)
  - `summary`
- Optional fields may include:
  - `detail`
  - `relatedLane`
  - `relatedCommit`
  - `relatedAgent`
  - `recordedBy`
- Use execution insights to preserve dynamic execution learnings that do not fit cleanly into lane state, such as blocker remediation suggestions, coordination drift, no-op findings, or workflow optimization ideas.
- Coordinator-facing automation may summarize execution insights, but it must treat them as assistive input rather than silently mutating lane state.
- Promote an execution insight into a tracked lane item when the observation implies concrete follow-up work with a bounded write set; keep it in the journal only when it is still exploratory, diagnostic, or primarily about orchestration quality.

## Current Repo Defaults

- `plot-mode` is the first full NLSDD execution.
- The existing lane worktrees remain valid, but their source of truth moves under `NLSDD/executions/plot-mode/`.
- Future multi-agent streams should start from NLSDD directly rather than cloning earlier fixed-4-lane naming.
