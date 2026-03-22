# NLSDD Communication Flow

## Channel Model

- Implementer does not talk directly to reviewer.
- Reviewer does not talk directly to implementer.
- Coordinator is the only official bridge between them.
- The bridge is mediated through the central executor, not ad-hoc markdown or free-form state files.
- Coordinator may operate in sidecar mode:
  - default: template-based forwarding
  - escalation: interrupt and arbitrate when communication quality degrades
- The only state-changing payload that should survive that bridge is the executor-recorded status/result payload; free-form thread text is transport, not state.

## Queue Promotion

- When an active slot opens, the coordinator may promote the next eligible queued lane into that slot.
- Promotion should respect write-set overlap, lane priority, and the current execution's refill order.
- Queued lanes stay in coordinator-visible state until they are promoted, blocked, or parked.

## Allowed Status Values

- `RUNNING`
- `BLOCKED`
- `READY_FOR_REVIEW`
- `DONE`
- `FAILED`
- `CANCELLED`

## Required Phase Hint

- Every reviewer or implementer handoff that changes lane state should include the next expected phase whenever it is knowable.
- Examples:
  - implementer `DONE` => `next expected phase: spec-review-pending`
  - implementer `READY_TO_COMMIT` => `next expected phase: coordinator-commit-pending`
  - spec `PASS` => `next expected phase: quality-review-pending`
  - quality `PASS` => `next expected phase: refill-ready`
  - `BLOCKED` => `next expected phase: blocked`

## Required Executor Payload

- NLSDD implementer and reviewer handoffs should be recorded through the executor interface rather than direct markdown/json writes.
- A result payload should carry, at minimum:
  - lane identity
  - normalized status
  - result branch
  - verification summary or blocker reason when applicable
- Free-form notes may still be included as payload fields, but the lane state transition itself must be explicit in executor state.

## Result-Branch Reporting

- In this repo's default NLSDD flow, sub-agents finish by reporting a single normalized status plus a `result_branch`.
- A sub-agent should not run `git commit` itself unless the lane explicitly authorizes self-commit.
- When implementation and verification are finished, the implementer should report `READY_FOR_REVIEW` and include:
  - `result_branch`
  - verification already completed
  - any blocker/probe note that still matters to review or integration
- Coordinator should treat `READY_FOR_REVIEW` as the normal end-of-implementation handoff.
- Review, commit intake, and merge remain coordinator/executor responsibilities after that handoff.

## Blocker Reporting

- When an implementer or reviewer reports `BLOCKED` or `NEEDS_CONTEXT`, it should include:
  - the exact blocker
  - why the blocker prevents the current lane item from completing cleanly
  - one preferred remediation suggestion
  - optional fallback suggestions if there is a clearly safer second choice
- Reviewers may also attach a workflow suggestion when the blocker is not source-code scope but orchestration noise, for example tracked `target/` artifacts making review harder.
- Coordinator remains the decision-maker; suggestions are advisory, not implicit approval.
- If the blocker or suggestion reveals a reusable execution insight, coordinator should record it through executor state so it survives beyond the transient thread.

## Execution Insight Reporting

- Use the execution insights journal for three kinds of runtime learnings:
  - actionable execution-local insights, such as sub-agent suggestions, coordinator-observed issues, and improvement opportunities that still affect the current execution
  - adopted durable global learnings that are ready to graduate into tracked spec or lesson files, after which the runtime copy should be resolved
  - resolved history, including closed, rejected, or superseded items that remain only as audit trail
- Insights are append-only runtime artifacts. They preserve dynamic learnings without overloading lane-state JSON or tracked docs.
- Insights should be concise enough to scan later, but specific enough to support a follow-up decision.
- When work is during or after an NLSDD stage and the prompt includes `review`, coordinator should inspect and summarize the execution insights journal as part of that pass, then decide whether each open/adopted insight stays exploratory, becomes a lane item, graduates into tracked spec/lessons and is resolved in runtime, or can be resolved/rejected.
- Review/autopilot helpers may surface open/adopted insights inline, but they should not auto-close or graduate them without an explicit coordinator decision.

## Coordinator Templates

These templates may be generated through `node NLSDD/scripts/nlsdd-compose-message.cjs ...`, but the coordinator still decides when and how to send them.

- When coordinator wants to collapse "reconcile stale lanes + promote next lanes + generate implementer-assignment text" into one deterministic pass, prefer `npm run nlsdd:launch -- --execution <id>`. The launch helper should reuse the same implementer-assignment template rather than inventing a second dispatch wording.
- When coordinator wants to collapse "inspect current review phases + decide the next spec/quality/correction/coordinator-commit step" into one deterministic pass, prefer `npm run nlsdd:review -- --execution <id>`. The review helper should emit the same template families already defined below, not invent a separate review wording.
- When coordinator wants to inspect only commit-ready handoffs, prefer `npm run nlsdd:intake -- --execution <id> [--lane <n>]`. The intake helper should output a normalized commit bundle rather than a reviewer message.
- When coordinator wants the whole deterministic round in one place, prefer `npm run nlsdd:autopilot -- --execution <id>`. The autopilot helper should aggregate launch assignments, review actions, and commit intake bundles without inventing new message formats.
- When main agent wants to act on that snapshot directly, prefer `npm run nlsdd:dispatch-plan -- --execution <id>`. The dispatch-plan helper should turn autopilot output into a prioritized queue of actionable bundles rather than inventing new message bodies.

### Implementer Assignment

- lane name
- lane item intent
- write scope
- acceptance intent
- required verification
- required handoff format
- explicit instruction that commit-ready MVC work should be handed back to coordinator if sub-agent commit may be gated
- explicit instruction that sub-agents should not self-commit unless the lane explicitly allows it

### Spec Review

- inspect only the lane-item commit diff
- ignore total dirty worktree state
- evaluate requested behavior, scope, and write-set compliance
- return `PASS` or `FAIL` with file/line refs

### Quality Review

- inspect only the same lane-item commit diff
- review maintainability, interface clarity, coupling, and test quality
- return `PASS` or `FAIL` with file/line refs

### Correction Loop

- cite the failing commit sha
- forward the exact reviewer finding
- restate accepted write scope
- require either a new commit sha plus verification results, or a fresh `READY_TO_COMMIT` handoff package for coordinator commit
- restate the next expected phase after the correction lands

### Blocker Escalation

- cite the blocking lane item and current phase
- forward the exact blocker text
- forward the suggested remediation
- decide one of:
  - accept remediation as-is
  - shrink or redirect the remediation
  - defer and keep the lane blocked

## Arbitration Rules

- Coordinator enters arbitration mode when:
  - spec and implementation disagree about lane scope
  - a correction loop exceeds 2 rounds
  - reviewer feedback conflicts with execution rules
  - the lane is blocked by cross-lane ambiguity
- In arbitration mode, coordinator may:
  - shrink the lane item
  - split a dependency item
  - loan a seam explicitly
  - replace a stalled reviewer or implementer
