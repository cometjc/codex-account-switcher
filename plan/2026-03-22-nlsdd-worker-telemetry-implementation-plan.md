# NLSDD Worker Telemetry Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add execution telemetry that can reconstruct per-minute active/productive worker counts, total wall-clock duration, and post-run drop-segment diagnostics for NLSDD executions.

**Architecture:** Keep `NLSDD/state/<execution>/events.ndjson` as the canonical source of truth. Extend the existing envelope/reducer flow with worker-local command lifecycle events and add a telemetry projection layer that emits runtime summary artifacts plus coordinator-facing review text. Worker-local `ps`/`pstree` probes are advisory evidence only; they never replace event truth.

**Tech Stack:** Node.js CommonJS scripts, markdown/JSON runtime artifacts, existing NLSDD envelope + reducer helpers, `node --test` automation tests.

---

## File Structure

- Modify: `NLSDD/scripts/nlsdd-envelope.cjs`
  - Add new envelope event types for command lifecycle telemetry.
  - Normalize and validate new telemetry-specific fields.
- Modify: `NLSDD/scripts/nlsdd-lib.cjs`
  - Add telemetry artifact path helpers and shared telemetry aggregation helpers.
- Modify: `NLSDD/scripts/nlsdd-record-lane-state.cjs`
  - Reuse shared envelope/event plumbing where helpful, but keep lane-state responsibilities narrow.
- Modify: `NLSDD/scripts/nlsdd-run-coordinator-loop.cjs`
  - Surface telemetry summary hints once projection exists.
- Modify: `package.json`
  - Add scripts for telemetry projection/review generation.
- Create: `NLSDD/scripts/nlsdd-record-command-event.cjs`
  - CLI/helper for worker-local `command-started` / `command-finished` / `command-failed` / `command-blocked` / `command-probe`.
- Create: `NLSDD/scripts/nlsdd-summarize-telemetry.cjs`
  - Project execution events into per-minute worker metrics, wall-clock duration, and drop-segment analysis.
- Create: `NLSDD/scripts/nlsdd-render-telemetry-review.cjs`
  - Render coordinator-readable markdown review from telemetry summary JSON.
- Modify: `tests/nlsdd-automation.test.js`
  - Add regression coverage for telemetry event recording, fast-fail classification, blocked/waiting classification, unknown silence fallback, and drop-segment aggregation.
- Modify: `NLSDD/AGENTS.md`
  - Document how NLSDD workers should record command lifecycle events and when to emit probes.
- Modify: `tasks/todo.md`
  - Track implementation progress and review notes after execution.

## Chunk 1: Event Schema And Recording

### Task 1: Extend envelope schema for command lifecycle events

**Files:**
- Modify: `NLSDD/scripts/nlsdd-envelope.cjs`
- Test: `tests/nlsdd-automation.test.js`

- [ ] **Step 1: Write the failing test**

Add tests that call the envelope recorder with these event types:
- `command-started`
- `command-finished`
- `command-failed`
- `command-blocked`
- `command-probe`

Verify the stored event keeps:
- `command`
- `cwd`
- `status`
- `exitCode`
- `durationMs`
- `blockKind`
- `probeSummary`
- `pid`

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test tests/nlsdd-automation.test.js`
Expected: FAIL because the new event types or fields are rejected/absent.

- [ ] **Step 3: Write minimal implementation**

Update `NLSDD/scripts/nlsdd-envelope.cjs` to:
- add the new event types to `ENVELOPE_EVENT_TYPES`
- normalize telemetry fields onto the stored envelope
- reject invalid `blockKind` values if provided
- keep existing lane state behavior unchanged

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test tests/nlsdd-automation.test.js`
Expected: PASS for the new schema coverage and no regressions in prior event handling.

- [ ] **Step 5: Commit**

```bash
git add tests/nlsdd-automation.test.js NLSDD/scripts/nlsdd-envelope.cjs
git commit -m "feat(nlsdd): 納入 command lifecycle telemetry event"
```

### Task 2: Add worker-local command event recorder helper

**Files:**
- Create: `NLSDD/scripts/nlsdd-record-command-event.cjs`
- Modify: `package.json`
- Test: `tests/nlsdd-automation.test.js`

- [ ] **Step 1: Write the failing test**

Add tests that execute the helper for:
- started event with command/cwd/pid
- failed event with exit code and duration
- blocked event with `blockKind`
- probe event with `probeSummary`

Assert the helper appends canonical envelope rows into `events.ndjson`.

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test tests/nlsdd-automation.test.js`
Expected: FAIL because the helper and npm script do not exist.

- [ ] **Step 3: Write minimal implementation**

Create `NLSDD/scripts/nlsdd-record-command-event.cjs` that:
- parses required execution/lane/event/command args
- normalizes telemetry payload
- delegates persistence to `recordEnvelope`

Add package script:
- `nlsdd:command:record`

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test tests/nlsdd-automation.test.js`
Expected: PASS for helper coverage.

- [ ] **Step 5: Commit**

```bash
git add tests/nlsdd-automation.test.js NLSDD/scripts/nlsdd-record-command-event.cjs package.json
git commit -m "feat(nlsdd): 新增 command telemetry 記錄 helper"
```

## Chunk 2: Telemetry Projection And Diagnostics

### Task 3: Project per-minute worker metrics from execution events

**Files:**
- Create: `NLSDD/scripts/nlsdd-summarize-telemetry.cjs`
- Modify: `NLSDD/scripts/nlsdd-lib.cjs`
- Test: `tests/nlsdd-automation.test.js`

- [ ] **Step 1: Write the failing test**

Add fixture-style tests that build a synthetic execution with:
- multiple lanes entering `implementing`
- one lane moving to `ready-to-commit`
- one lane failing fast on `curl google.com`
- one lane recording `command-blocked`

Assert the summary JSON contains:
- `wallClockDurationMs`
- `firstActivityAt`
- `lastActivityAt`
- minute buckets with `activeWorkers`
- minute buckets with `productiveWorkers`

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test tests/nlsdd-automation.test.js`
Expected: FAIL because no telemetry summarizer exists.

- [ ] **Step 3: Write minimal implementation**

In `NLSDD/scripts/nlsdd-lib.cjs`, add shared helpers for:
- telemetry summary output paths
- minute bucket normalization
- event grouping by minute/lane

Create `NLSDD/scripts/nlsdd-summarize-telemetry.cjs` that:
- loads execution events
- infers worker minute states from phase and command lifecycle events
- writes `NLSDD/state/<execution>/telemetry-summary.json`
- optionally prints JSON

Initial classification rules:
- active: non-queued/non-parked lane activity or in-flight command
- productive: `implementing` or explicit productive in-flight command
- fast-fail: immediate `command-failed`
- blocked/waiting: `command-blocked`

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test tests/nlsdd-automation.test.js`
Expected: PASS with stable minute-bucket projection.

- [ ] **Step 5: Commit**

```bash
git add tests/nlsdd-automation.test.js NLSDD/scripts/nlsdd-lib.cjs NLSDD/scripts/nlsdd-summarize-telemetry.cjs
git commit -m "feat(nlsdd): 投影 execution worker telemetry 摘要"
```

### Task 4: Add drop-segment diagnostics and unknown-silence fallback

**Files:**
- Modify: `NLSDD/scripts/nlsdd-summarize-telemetry.cjs`
- Test: `tests/nlsdd-automation.test.js`

- [ ] **Step 1: Write the failing test**

Add tests for drop segments covering:
- handoff wait (`ready-to-commit` / review pending)
- dependency blocked
- fast fail (`curl` DNS failure shape)
- command blocked with probe evidence
- missing evidence fallback to `unknown-silence`

Assert each drop segment includes:
- `fromMinute`
- `toMinute`
- `metric`
- `reason`
- `confidence`
- `supportingEvents`
- `missingSignals`

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test tests/nlsdd-automation.test.js`
Expected: FAIL because diagnostics are absent or incomplete.

- [ ] **Step 3: Write minimal implementation**

Extend the telemetry summarizer to:
- detect when `activeWorkers` or `productiveWorkers` decline
- create drop segments
- attach the best available reason based on nearby events
- emit `unknown-silence` with explicit missing signals when evidence is insufficient

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test tests/nlsdd-automation.test.js`
Expected: PASS with deterministic diagnostics.

- [ ] **Step 5: Commit**

```bash
git add tests/nlsdd-automation.test.js NLSDD/scripts/nlsdd-summarize-telemetry.cjs
git commit -m "feat(nlsdd): 補上並行下降診斷與 unknown silence fallback"
```

## Chunk 3: Coordinator Surface And Workflow Integration

### Task 5: Render telemetry review artifacts for coordinator use

**Files:**
- Create: `NLSDD/scripts/nlsdd-render-telemetry-review.cjs`
- Modify: `package.json`
- Test: `tests/nlsdd-automation.test.js`

- [ ] **Step 1: Write the failing test**

Add tests that feed a summarized telemetry fixture into the renderer and assert output includes:
- wall-clock duration
- per-minute worker table
- drop segment list
- reasons + confidence
- missing information guidance

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test tests/nlsdd-automation.test.js`
Expected: FAIL because review renderer does not exist.

- [ ] **Step 3: Write minimal implementation**

Create renderer that writes:
- `NLSDD/state/<execution>/telemetry-review.md`

Add npm script:
- `nlsdd:telemetry:review`

Keep output concise and coordinator-focused.

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test tests/nlsdd-automation.test.js`
Expected: PASS with stable markdown rendering.

- [ ] **Step 5: Commit**

```bash
git add tests/nlsdd-automation.test.js NLSDD/scripts/nlsdd-render-telemetry-review.cjs package.json
git commit -m "feat(nlsdd): 新增 telemetry review 輸出"
```

### Task 6: Wire telemetry into coordinator workflow and rules

**Files:**
- Modify: `NLSDD/scripts/nlsdd-run-coordinator-loop.cjs`
- Modify: `NLSDD/AGENTS.md`
- Modify: `tasks/todo.md`
- Test: `tests/nlsdd-automation.test.js`

- [ ] **Step 1: Write the failing test**

Add integration coverage showing coordinator output now references telemetry summary/review availability for an execution with events.

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test tests/nlsdd-automation.test.js`
Expected: FAIL because coordinator output has no telemetry awareness.

- [ ] **Step 3: Write minimal implementation**

Update coordinator loop to:
- load telemetry summary when present
- surface high-level counts or review hints

Update `NLSDD/AGENTS.md` to require workers to:
- emit `command-started` before long/meaningful commands
- emit `command-finished` / `command-failed` after completion
- emit `command-blocked` or `command-probe` when a command appears stuck or silent

Update `tasks/todo.md` review notes after implementation.

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test tests/nlsdd-automation.test.js`
Expected: PASS with workflow-level integration.

- [ ] **Step 5: Run full verification**

Run:
- `node --test tests/nlsdd-automation.test.js`
- `npm run nlsdd:scoreboard:refresh`
- `npm run build`

Expected:
- all tests pass
- runtime scoreboard refresh succeeds
- TypeScript build remains green

- [ ] **Step 6: Commit**

```bash
git add tests/nlsdd-automation.test.js NLSDD/scripts/nlsdd-run-coordinator-loop.cjs NLSDD/AGENTS.md tasks/todo.md
git commit -m "feat(nlsdd): 將 worker telemetry 接進 coordinator workflow"
```
