# NLSDD Guardrails

## Purpose

Use this file to keep NLSDD execution predictable, low-conflict, and easy to review.

## Core Rules

- Every sub-agent owns an explicit write set before starting work.
- Every sub-agent should be assigned through one of the current execution lane plans first, not through an ad-hoc one-off task description.
- Every lane should run inside its own dedicated worktree.
- Every lane item should create its own lane-item commit before review starts.
- When a lane-local MVC step is done and verified, the default next action is to finalize that step into its own lane-item commit rather than batching additional completed MVC work into the same uncommitted state.
- Distinguish commit ownership by context:
  - main agent direct local work: verify first, then commit directly
  - NLSDD sub-agent lane work: default to `READY_TO_COMMIT`, and coordinator/main agent performs the commit
  - self-commit by a sub-agent is an explicit exception, not the default
- Do not assign the same file to two active implementer agents at the same time.
- Every sub-agent must assume other agents are editing nearby code and must not revert unrelated changes.
- A completed lane item is not considered integrated until the coordinator sees both spec review and code quality review pass on that lane-item diff.

## Autopilot Refill

- Keep the configured active subagent cap saturated whenever there are enough non-overlapping lane items.
- Lane pool size may exceed the active cap; extra lanes should remain queued or parked rather than stealing a running slot.
- When an active slot opens, promote the next eligible queued lane whose write set does not overlap any active lane.
- When one lane reaches `quality PASS`, dispatch the next unchecked item from that same lane before doing batch tracking updates.
- Use the refill assistant to suggest the next unchecked lane-local item, but treat its output as a coordinator-side draft rather than automatic dispatch.
- If refill work would overlap an active lane's ownership, do not dispatch it yet.
- If the lane that freed a slot has no safe refill item, use the next queued lane rather than leaving the slot idle.
- Only create a new lane plan when an existing lane family is genuinely exhausted.
- Do not fake saturation with pseudo-active lanes. If one blocker or one nearly-finished lane is causing 2-3 slots to wait, treat that as a replan trigger.
- Preferred remediation order for convergence:
  - cut the next honest independent lane inside the same execution
  - if that is not available, advance 2-3 plans/executions in parallel instead of forcing all slots to stay on one execution
  - only accept temporary under-utilization when no truthful parallel work exists

## Anti-Convergence Heuristic

- A convergence warning exists when:
  - one lane is the only lane with real forward motion
  - other active lanes are repeatedly returning `NOOP`, `BLOCKED`, or clean probes
  - 3 slots are effectively waiting for 1 lane to finish before anything meaningful can continue
- When convergence warning triggers, coordinator should:
  - record the observation in execution insights
  - decide whether to cut another lane, park exhausted lanes, or bring a different plan/execution forward
  - avoid preserving a 4-active illusion in tracked docs if only 1 lane is truly advancing

## Lane Status Probe

- Trigger a probe when:
  - a sub-agent reports `IN_PROGRESS` more than once without a commit SHA
  - a thread goes quiet longer than expected for the lane item
  - thread status and worktree status appear inconsistent
  - a lane item is expected to end in a commit and the sub-agent may be waiting on a permission or confirmation prompt instead of actively coding
- Treat `implementing` as stale when the lane journal still says `implementing`, but the lane worktree is clean and `HEAD` still matches `latestCommit`; in that case, scheduler/probe output should surface `stale-implementing` so coordinator can reassign or park the lane instead of counting it as an active slot.
- Probe checklist:
  - `node NLSDD/scripts/nlsdd-probe-lane.cjs --execution <id> --lane <n>`
  - or, if the helper is unavailable, fall back to:
    - `git rev-parse --short HEAD`
    - `git status --short`
    - `git diff --stat`
    - `git log --oneline -n 1`
    - the lane-local verification command from the lane plan
- Probe results override thread assumptions and must be reflected in the scoreboard.
- Runtime probe/refresh output belongs under `NLSDD/state/`; do not use auto-refresh to rewrite the tracked `NLSDD/scoreboard.md`.
- When a lane journal exists, probes should treat it as the primary execution-aware state surface and only use thread/session heuristics as fallback.
- If a lane appears stalled near the end of implementation, coordinator should explicitly consider whether the sub-agent may be blocked on a commit/permission prompt before classifying it as unresponsive.

## Blocker Suggestions

- Sub-agents are allowed and encouraged to propose a concrete remediation when reporting `BLOCKED`, `NEEDS_CONTEXT`, or a workflow-level concern.
- The suggestion should stay narrowly tied to the blocker, for example:
  - lane hygiene cleanup for tracked `target/` noise
  - dependency seam expansion in the owning lane
  - review-scope correction when a reviewer is reading the wrong diff
- Suggestions do not authorize the sub-agent to expand scope on its own; coordinator still decides whether to accept, defer, or split the proposed fix.

## Noise Handling

- Classify noise as one of:
  - `none`
  - `untracked-artifact-noise`
  - `tracked-artifact-noise`
  - `mixed`
- Build outputs such as `rust/plot-viewer/target/` are artifacts, not lane-item scope.
- Reviewers ignore artifact-noise paths and evaluate only source changes in the lane-item diff.
- If tracked artifact noise appears, coordinator should schedule a lane hygiene cleanup rather than letting it accumulate.

## Communication Heuristics

- Reviewer and implementer communication flows through coordinator sidecar mode.
- Use fixed templates for:
  - implementer assignment
  - spec review
  - quality review
  - correction loop
- Prefer `node NLSDD/scripts/nlsdd-compose-message.cjs ...` to generate those templates consistently.
- Those templates should ask for one strict lane handoff envelope JSON object on return, not a free-form NLSDD status paragraph.
- If correction loops exceed 2 rounds, escalate to coordinator arbitration.
- When coordinator records a new lane state after review, correction, or blockage, prefer `node NLSDD/scripts/nlsdd-record-lane-state.cjs ...` over hand-editing journal JSON.
- When a sub-agent suggests a remediation, or coordinator notices a workflow issue or optimization opportunity during execution, prefer `node NLSDD/scripts/nlsdd-record-insight.cjs ...` over burying that observation only in thread history.
- `nlsdd-record-lane-state` and `nlsdd-record-insight` are compatibility entrypoints. They should translate their input into the canonical envelope flow instead of bypassing the reducer/projection path.
- Prefer `node NLSDD/scripts/nlsdd-envelope.cjs ...` or reducer-backed helpers when adding new NLSDD automation, so the event log remains the only write interface.
- When an NLSDD prompt includes `review`, coordinator should also inspect the execution insights journal and decide whether any open/adopted insight needs planning, adoption, or closure alongside the normal lane review work.
- When coordinator needs a refreshed scoreboard snapshot, prefer `npm run nlsdd:scoreboard:refresh` and inspect `NLSDD/state/scoreboard.runtime.md` rather than staging runtime churn from the tracked scoreboard.
- When coordinator changes the active/parked lane set for an execution, prefer `npm run nlsdd:active-set:replan -- --execution <id> --active ... --parked ...` so tracked `Phase` values and lane journals stay aligned.
- When coordinator wants to do the routine, low-judgment part of dispatching in one step, prefer `npm run nlsdd:cycle -- --execution <id>`. The cycle helper should reconcile stale implementing lanes, promote the next safe queued/refill-ready lanes, refresh runtime state, and return a dispatch summary with completed lanes, newly promoted lanes, and idle slots.
- When coordinator wants the same pass to also produce ready-to-send implementer handoff text, prefer `npm run nlsdd:launch -- --execution <id>`. The launch helper should run the cycle, then build assignment bundles from the promoted lanes' plan scope, verification commands, and next items.
- When coordinator wants to advance the post-implementation review loop without re-reading every lane manually, prefer `npm run nlsdd:review -- --execution <id>`. The review helper should inspect the current lane phase and emit the next coordinator-ready bundle for spec review, quality review, correction, or coordinator-commit intake.
- When implementers hand back `READY_TO_COMMIT`, prefer `npm run nlsdd:intake -- --execution <id> [--lane <n>]` before creating the commit. The intake helper should surface the proposed commit title/body, completed verification, scope, and note so the coordinator can commit without another clarification round.
- When coordinator wants one low-judgment pass that combines launch, review, and commit-intake visibility, prefer `npm run nlsdd:autopilot -- --execution <id>`. The autopilot helper should not dispatch tools by itself, but it should return the full coordinator-ready snapshot for this round.
- When main agent wants the exact execution order for this round, prefer `npm run nlsdd:dispatch-plan -- --execution <id>`. The dispatch-plan helper should prioritize commit intake first, then review/correction work, then new launch assignments, and it should not invent a separate source of truth beyond the coordinator loop output.
- When execution insights exist, prefer `node NLSDD/scripts/nlsdd-summarize-insights.cjs --execution <id>` before or during review/autopilot work so open and adopted runtime learnings do not stay invisible.
- Implementer assignments for NLSDD sub-agents should tell them not to run `git commit` themselves unless the lane explicitly says self-commit is allowed. The default end state is `READY_TO_COMMIT` with verification results and commit-ready handoff details.
- Do not keep a completed MVC step uncommitted just because the lane may continue later; later refill items should start from the committed lane state, not from piled-up local progress.

## Required Handoff Format

- Lane name
- MVC step completed
- Commit SHA for the lane item, or intended commit summary if coordinator commit is pending
- Next expected phase
- Files changed
- What was implemented
- What was intentionally stubbed or deferred
- Verification run
- Open concerns or dependency assumptions
- Suggested remediation when blocked or when a recurring workflow problem is detected
- If commit is pending due to an environment prompt or expected permission gate, say so explicitly instead of remaining silent
- If coordinator commit is needed, include a proposed commit title/body summary so the coordinator can commit without another clarification round
- If the handoff includes a broader process suggestion, execution insight, or improvement opportunity, coordinator should also append it into the execution insights journal.

## Batch Tracking Policy

- Prefer one coordinator tracking update for 2-4 lane state changes rather than rewriting tracking files after every single lane transition.
- Scoreboard should be updated first when quick state clarity matters.
- `tasks/todo.md`, roadmap files, and lessons can follow in the same batch unless an urgent correction pattern needs to be captured immediately.
