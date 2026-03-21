# NLSDD Guardrails

## Purpose

Use this file to keep NLSDD execution predictable, low-conflict, and easy to review.

## Core Rules

- Every sub-agent owns an explicit write set before starting work.
- Every sub-agent should be assigned through one of the current execution lane plans first, not through an ad-hoc one-off task description.
- Every lane should run inside its own dedicated worktree.
- Every lane item should create its own implementer commit before review starts.
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

## Lane Status Probe

- Trigger a probe when:
  - a sub-agent reports `IN_PROGRESS` more than once without a commit SHA
  - a thread goes quiet longer than expected for the lane item
  - thread status and worktree status appear inconsistent
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
- If correction loops exceed 2 rounds, escalate to coordinator arbitration.
- When coordinator records a new lane state after review, correction, or blockage, prefer `node NLSDD/scripts/nlsdd-record-lane-state.cjs ...` over hand-editing journal JSON.
- When coordinator needs a refreshed scoreboard snapshot, prefer `npm run nlsdd:scoreboard:refresh` and inspect `NLSDD/state/scoreboard.runtime.md` rather than staging runtime churn from the tracked scoreboard.

## Required Handoff Format

- Lane name
- MVC step completed
- Commit SHA for the lane item
- Next expected phase
- Files changed
- What was implemented
- What was intentionally stubbed or deferred
- Verification run
- Open concerns or dependency assumptions
- Suggested remediation when blocked or when a recurring workflow problem is detected

## Batch Tracking Policy

- Prefer one coordinator tracking update for 2-4 lane state changes rather than rewriting tracking files after every single lane transition.
- Scoreboard should be updated first when quick state clarity matters.
- `tasks/todo.md`, roadmap files, and lessons can follow in the same batch unless an urgent correction pattern needs to be captured immediately.
