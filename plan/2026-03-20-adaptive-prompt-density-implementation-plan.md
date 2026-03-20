# Adaptive Prompt Density Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a condensed prompt density mode that keeps the current prompt panel readable when profile count or terminal height makes the full layout too tall.

**Architecture:** Keep the current `Delta` and `Quota` semantic split, but add one extra density decision layer before rendering. Normal density continues to use the current renderers; condensed density reuses the same row models while emitting fewer lower-priority fields so we do not fork recommendation logic.

**Tech Stack:** TypeScript, Node.js, oclif, inquirer prompt rendering, existing prompt panel layout helpers

---

## MVP Assumptions

- The first condensed-density MVP introduces exactly one fallback level: `condensed`.
- Density should be chosen from vertical pressure first, not terminal width alone.
- A practical first heuristic is:
  - estimate per-profile line cost from current mode
  - add prompt/help/options overhead
  - switch to `condensed` when the full panel would consume most of the visible terminal height before the option list is shown
- `Delta` condensed mode must still keep:
  - profile
  - last update
  - `W:` / `5H:` markers
  - usage left
  - reset time
  - reset percent
  - pacing value
- `Quota` condensed mode must still keep:
  - profile
  - last update
  - quota bar
  - time to reset
  - usage left
- The first MVP should not introduce horizontal truncation logic beyond what existing terminal wrapping already does; it should only reduce vertical density.

## File Structure

- Create: `plan/2026-03-20-adaptive-prompt-density-implementation-plan.md`
- Modify: `src/commands/root.ts`
- Modify: `src/lib/root-panel-layout.ts`
- Modify: `tests/root-panel-layout.test.js`
- Modify: `tests/root-option-layout.test.js`
- Modify: `tasks/todo.md`
- Modify: `spec/2026-03-20-prompt-panel-layout.md` (only after implementation + verification)

## Chunk 1: Define Density Decision Rules

### Task 1: Add explicit prompt density state

**Files:**
- Modify: `src/commands/root.ts`
- Test: `tests/root-option-layout.test.js`

- [ ] **Step 1: Add a prompt density enum**

Define a local prompt density state with at least:
- `full`
- `condensed`

- [ ] **Step 2: Add a pure density decision helper**

Create a helper in `src/commands/root.ts` that decides density from:
- terminal rows / page size
- rendered profile count
- current bar style (`delta` vs `quota`)

Rules for the first MVP:
- prefer `full` when the current panel can show all profiles comfortably
- fall back to `condensed` when profile count or terminal height would force the user to scroll excessively
- do not add more than one condensed level yet
- encode the threshold in one helper so future tuning does not require touching multiple render paths

- [ ] **Step 3: Write the failing density test**

Add a test that shows:
- small profile count + taller terminal => `full`
- larger profile count or shorter terminal => `condensed`
- `Delta` and `Quota` can use different per-profile line estimates if needed, but the decision must stay deterministic

- [ ] **Step 4: Run the targeted test to verify it fails**

Run:
```bash
node --test tests/root-option-layout.test.js
```

Expected:
- FAIL because the new density helper/behavior does not exist yet

- [ ] **Step 5: Implement the minimal density helper**

Only add the helper and the simplest thresholds needed for the test to pass.

- [ ] **Step 6: Re-run the targeted test**

Run:
```bash
node --test tests/root-option-layout.test.js
```

Expected:
- PASS

## Chunk 2: Add Condensed Delta Panel Rendering

### Task 2: Keep profile header, compress lower-priority fields

**Files:**
- Modify: `src/lib/root-panel-layout.ts`
- Modify: `src/commands/root.ts`
- Test: `tests/root-panel-layout.test.js`

- [ ] **Step 1: Write a failing condensed delta panel test**

Add a test that locks this first condensed `Delta` behavior:
- first line remains `profile + last update`
- lower lines keep `W:` / `5H:`
- condensed lines still keep `📊`, `🔄 in`, and `Pacing`
- condensed lines drop extra width where possible while preserving numeric field alignment

- [ ] **Step 2: Run the targeted test to verify it fails**

Run:
```bash
node --test tests/root-panel-layout.test.js
```

Expected:
- FAIL because condensed rendering does not exist yet

- [ ] **Step 3: Add condensed delta renderer inputs**

Extend the current panel layout model only as needed so the renderer can choose:
- full field set
- condensed field set

Do not duplicate recommendation math or prompt assembly logic.

- [ ] **Step 4: Implement condensed delta detail lines**

Condensed delta should still preserve:
- `profile + last update`
- right-aligned numeric fields
- bottleneck-only pacing highlight

But it may reduce:
- optional spacing
- lower-priority descriptive padding
- repeated vertical breathing room between profile blocks

Target condensed delta shape for MVP:
- line 1: `profile + last update`
- line 2: `W:` compact detail line
- line 3: optional `5H:` compact detail line

- [ ] **Step 5: Re-run the targeted test**

Run:
```bash
node --test tests/root-panel-layout.test.js
```

Expected:
- PASS

## Chunk 3: Add Condensed Quota Panel Rendering

### Task 3: Keep Quota semantics while shortening the panel

**Files:**
- Modify: `src/commands/root.ts`
- Test: `tests/root-option-layout.test.js`

- [ ] **Step 1: Write a failing condensed quota prompt test**

Lock these rules:
- first line stays `profile + last update`
- quota mode still omits delta comparison content
- condensed quota mode keeps only:
  - bar
  - time to reset
  - usage left
- no drift/bottleneck comparison text leaks back in
- condensed quota should remain two lines when only `W:` exists and three lines when both `W:` and `5H:` exist

- [ ] **Step 2: Run the targeted test to verify it fails**

Run:
```bash
node --test tests/root-option-layout.test.js
```

Expected:
- FAIL because quota condensed rendering does not exist yet

- [ ] **Step 3: Implement condensed quota block**

Reuse the current quota block path, but allow it to remove only non-essential spacing/height.

- [ ] **Step 4: Re-run the targeted test**

Run:
```bash
node --test tests/root-option-layout.test.js
```

Expected:
- PASS

## Chunk 4: Wire Density Into the Prompt Flow

### Task 4: Route full vs condensed rendering from the menu builder

**Files:**
- Modify: `src/commands/root.ts`
- Test: `tests/root-option-layout.test.js`

- [ ] **Step 1: Write a failing integration-style test**

Add a test showing that `renderPromptPanelText(...)` or the nearest pure helper:
- uses `full` density under comfortable conditions
- switches to `condensed` under constrained conditions

- [ ] **Step 2: Run the targeted test to verify it fails**

Run:
```bash
node --test tests/root-option-layout.test.js
```

Expected:
- FAIL because density is not wired into rendering yet

- [ ] **Step 3: Pass density through the prompt-building path**

Wire the density decision through:
- current terminal dimensions
- current item count
- current bar mode

Keep the option list behavior unchanged.

- [ ] **Step 3a: Preserve a single rendering choke point**

Do not branch density decisions in multiple places.
One helper should decide density, and one render path per mode should consume it.

- [ ] **Step 4: Re-run the targeted test**

Run:
```bash
node --test tests/root-option-layout.test.js
```

Expected:
- PASS

## Chunk 5: Verification and Documentation

### Task 5: Verify and update shipped docs only after passing behavior

**Files:**
- Modify: `tasks/todo.md`
- Modify: `spec/2026-03-20-prompt-panel-layout.md`

- [ ] **Step 1: Run full verification**

Run:
```bash
npm run build
node --test tests/root-option-layout.test.js
node --test tests/root-panel-layout.test.js
node --test tests/root-table-layout.test.js
node --test tests/workload-tier.test.js
node --test tests/entrypoints.test.js
```

Expected:
- All PASS

- [ ] **Step 2: Update task review notes**

Add a `tasks/todo.md` section summarizing:
- density trigger rule
- condensed Delta behavior
- condensed Quota behavior
- verification commands
- any intentionally deferred ideas, such as a second condensed tier or width-aware truncation

- [ ] **Step 3: Update shipped spec after verification**

Only after all checks pass, update `spec/2026-03-20-prompt-panel-layout.md` to describe:
- when condensed density activates
- what fields are preserved in Delta vs Quota
- what readability guarantees remain
- what is explicitly out of scope for this first condensed-density MVP

- [ ] **Step 4: Commit**

```bash
git add src/commands/root.ts src/lib/root-panel-layout.ts tests/root-option-layout.test.js tests/root-panel-layout.test.js tasks/todo.md spec/2026-03-20-prompt-panel-layout.md plan/2026-03-20-adaptive-prompt-density-implementation-plan.md
git commit -m "feat(ui): 新增 prompt panel 自適應密度"
```
