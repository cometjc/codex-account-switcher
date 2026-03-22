# NLSDD Dev Flow Improvement Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the main NLSDD coordination frictions that still force manual cleanup or cause brittle failures: runtime scoreboard hard-fail, stale lane-state drift after work lands on `main`, and repeated manual syncing across tracked planning surfaces.

**Architecture:** Keep `events.ndjson` + lane journal + tracked scoreboard as the existing NLSDD execution model, but harden the coordinator path so it degrades gracefully when runtime artifacts are missing and add one explicit reconcile/sync path that can collapse stale implementing lanes into honest parked/no-op truth. Reduce manual tracked-surface drift by generating or refreshing the lane-status sections that currently need hand editing after every accepted lane result.

**Tech Stack:** Node.js CommonJS scripts, markdown tracked docs, `node --test tests/nlsdd-automation.test.js`, existing NLSDD envelope/reducer helpers.

---

## File Structure

- Modify: `NLSDD/scripts/nlsdd-intake-ready-to-commit.cjs`
  - Make commit-intake loading robust when runtime scoreboard output is missing or malformed.
- Modify: `NLSDD/scripts/nlsdd-run-coordinator-loop.cjs`
  - Prevent one auxiliary surface failure from crashing the whole coordinator snapshot.
- Modify: `NLSDD/scripts/nlsdd-lib.cjs`
  - Add shared helpers for resilient scoreboard loading and stale-lane reconciliation.
- Modify: `NLSDD/scripts/nlsdd-refresh-scoreboard.cjs`
  - Optionally expose a safe fallback path when runtime scoreboard needs regeneration.
- Create: `NLSDD/scripts/nlsdd-sync-execution-truth.cjs`
  - Reconcile stale implementing lanes and refresh tracked execution docs from runtime/mainline truth.
- Modify: `tests/nlsdd-automation.test.js`
  - Add regressions for missing runtime scoreboard handling, stale implementing reconciliation, and tracked lane-status sync.
- Modify: `NLSDD/AGENTS.md`
  - Document the new reconcile/sync path and the degraded-mode expectations.
- Modify: `spec/NLSDD/operating-rules.md`
  - Clarify what must be projection-only versus coordinator-authored, and what runtime helpers must do when auxiliary artifacts are absent.
- Modify: `tasks/todo.md`
  - Track progress and review notes.

## Chunk 1: Make Coordinator Loop Fail Softly

### Task 1: Stop runtime scoreboard absence from crashing dispatch/coordinator flow

**Files:**
- Modify: `NLSDD/scripts/nlsdd-lib.cjs`
- Modify: `NLSDD/scripts/nlsdd-intake-ready-to-commit.cjs`
- Modify: `NLSDD/scripts/nlsdd-run-coordinator-loop.cjs`
- Test: `tests/nlsdd-automation.test.js`

- [x] **Step 1: Write the failing test**

Add coverage for:
- runtime scoreboard path exists but is empty / malformed
- `intakeReadyToCommit()` still returns `[]` instead of throwing
- `runCoordinatorLoop()` still returns launch/review/insight results even when commit-intake has to fall back

- [x] **Step 2: Run test to verify it fails**

Run: `node --test tests/nlsdd-automation.test.js`
Expected: FAIL because `loadScoreboardTable()` currently throws through `intakeReadyToCommit()`.

- [x] **Step 3: Write minimal implementation**

Implement:
- one helper that tries the preferred runtime scoreboard first, then falls back to tracked scoreboard if parsing fails
- commit-intake degraded mode that treats unreadable scoreboard surfaces as `[]`
- coordinator loop behavior that records the degraded condition instead of aborting the whole snapshot

- [x] **Step 4: Run verification**

Run: `node --test tests/nlsdd-automation.test.js`
Expected: PASS

- [x] **Step 5: Commit**

```bash
git add tests/nlsdd-automation.test.js NLSDD/scripts/nlsdd-lib.cjs NLSDD/scripts/nlsdd-intake-ready-to-commit.cjs NLSDD/scripts/nlsdd-run-coordinator-loop.cjs
git commit -m "fix(nlsdd): 避免 runtime scoreboard 缺失時協調流程崩潰"
```

## Chunk 2: Reconcile Stale Implementing Lanes Automatically

### Task 2: Add one execution-truth sync path for stale implementing cleanup

**Files:**
- Create: `NLSDD/scripts/nlsdd-sync-execution-truth.cjs`
- Modify: `NLSDD/scripts/nlsdd-lib.cjs`
- Modify: `NLSDD/scripts/nlsdd-refresh-scoreboard.cjs`
- Test: `tests/nlsdd-automation.test.js`

- [x] **Step 1: Write the failing test**

Add coverage for an execution where:
- tracked scoreboard says `implementing`
- lane journal says `implementing`
- worktree is clean at the same `HEAD`
- tracked lane docs still show old implementing/current-item text

Assert the sync helper:
- marks the lane `parked` or another honest phase
- updates lane journal
- refreshes tracked scoreboard row
- returns a machine-readable summary of reconciled lanes

- [x] **Step 2: Run test to verify it fails**

Run: `node --test tests/nlsdd-automation.test.js`
Expected: FAIL because no execution-truth sync helper exists.

- [x] **Step 3: Write minimal implementation**

Implement a helper that:
- scans one execution via existing schedule/probe logic
- finds `stale-implementing` lanes
- rewrites lane journal + tracked scoreboard phase atomically
- refreshes runtime scoreboard after reconciliation

Do not auto-edit plan bodies yet in this task; keep the first version focused on phase/state truth.

- [x] **Step 4: Run verification**

Run:
- `node --test tests/nlsdd-automation.test.js`
- `npm run nlsdd:scoreboard:refresh`

Expected: PASS

- [x] **Step 5: Commit**

```bash
git add tests/nlsdd-automation.test.js NLSDD/scripts/nlsdd-lib.cjs NLSDD/scripts/nlsdd-refresh-scoreboard.cjs NLSDD/scripts/nlsdd-sync-execution-truth.cjs
git commit -m "feat(nlsdd): 新增 execution truth 同步 helper"
```

## Chunk 3: Reduce Manual Tracked-Surface Drift

### Task 3: Sync lane-status sections from execution truth

**Files:**
- Modify: `NLSDD/scripts/nlsdd-sync-execution-truth.cjs`
- Modify: `NLSDD/AGENTS.md`
- Modify: `spec/NLSDD/operating-rules.md`
- Modify: `tasks/todo.md`
- Test: `tests/nlsdd-automation.test.js`

- [ ] **Step 1: Write the failing test**

Add coverage for tracked lane docs where `## Current Lane Status` still says:
- old phase
- old latest commit
- stale current item / note

Assert the sync helper can refresh only that section from execution truth without rewriting the rest of the lane plan.

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test tests/nlsdd-automation.test.js`
Expected: FAIL because no tracked lane-status sync exists.

- [ ] **Step 3: Write minimal implementation**

Extend the sync helper so it:
- updates the `Current Lane Status` section in `NLSDD/executions/<execution>/lane-<n>.md`
- leaves MVC/refill checklist text untouched
- optionally refreshes overview/scoreboard summary notes where phase drift would otherwise linger

Update docs/rules to say:
- tracked lane-status sections are sync targets, not freehand coordinator memory
- `nlsdd-sync-execution-truth` is the preferred cleanup path after accepted work lands on `main`

- [ ] **Step 4: Run verification**

Run:
- `node --test tests/nlsdd-automation.test.js`
- `node NLSDD/scripts/nlsdd-sync-execution-truth.cjs --execution plot-mode --dry-run`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add tests/nlsdd-automation.test.js NLSDD/scripts/nlsdd-sync-execution-truth.cjs NLSDD/AGENTS.md spec/NLSDD/operating-rules.md tasks/todo.md
git commit -m "feat(nlsdd): 收斂 lane status 與 execution truth 同步流程"
```
