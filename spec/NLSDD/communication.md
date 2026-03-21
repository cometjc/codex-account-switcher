# NLSDD Communication Flow

## Channel Model

- Implementer does not talk directly to reviewer.
- Reviewer does not talk directly to implementer.
- Coordinator is the only official bridge between them.
- Coordinator may operate in sidecar mode:
  - default: template-based forwarding
  - escalation: interrupt and arbitrate when communication quality degrades

## Queue Promotion

- When an active slot opens, the coordinator may promote the next eligible queued lane into that slot.
- Promotion should respect write-set overlap, lane priority, and the current execution's refill order.
- Queued lanes stay in coordinator-visible state until they are promoted, blocked, or parked.

## Allowed Status Values

- `IN_PROGRESS`
- `DONE`
- `DONE_WITH_CONCERNS`
- `BLOCKED`
- `NEEDS_CONTEXT`
- `PASS`
- `FAIL`

## Required Phase Hint

- Every reviewer or implementer handoff that changes lane state should include the next expected phase whenever it is knowable.
- Examples:
  - implementer `DONE` => `next expected phase: spec-review-pending`
  - spec `PASS` => `next expected phase: quality-review-pending`
  - quality `PASS` => `next expected phase: refill-ready`
  - `BLOCKED` => `next expected phase: blocked`

## Blocker Reporting

- When an implementer or reviewer reports `BLOCKED` or `NEEDS_CONTEXT`, it should include:
  - the exact blocker
  - why the blocker prevents the current lane item from completing cleanly
  - one preferred remediation suggestion
  - optional fallback suggestions if there is a clearly safer second choice
- Reviewers may also attach a workflow suggestion when the blocker is not source-code scope but orchestration noise, for example tracked `target/` artifacts making review harder.
- Coordinator remains the decision-maker; suggestions are advisory, not implicit approval.

## Coordinator Templates

These templates may be generated through `node NLSDD/scripts/nlsdd-compose-message.cjs ...`, but the coordinator still decides when and how to send them.

### Implementer Assignment

- lane name
- lane item intent
- write scope
- acceptance intent
- required verification
- required handoff format

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
- require a new commit sha plus verification results
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
