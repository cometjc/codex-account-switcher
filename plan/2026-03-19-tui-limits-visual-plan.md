# TUI Limits Visualization Plan (2026-03-19)

## Goal
- Upgrade `codex-auth` profile list UI to show time-progress + usage in the same bar.
- Add mode cycling via `B`: `full` -> `5h-only` -> `weekly-only`.
- Keep multi-line profile rendering in full mode.

## Confirmed Requirements
- Bar semantics: elapsed-time as background color; usage overlays foreground blocks.
- Time labels:
  - 5h: `HH:MM` on both ends.
  - Weekly: `MM/DD HH:MM` on both ends.
- Axis alignment per limit type:
  - Shared axis anchored at `now`.
  - End at max `reset_at` among visible profiles of same type.
- Single-line modes:
  - One line per profile with `profile + bar + time labels + can-use`.
- Weekly can-use format:
  - `f02.1%/day for next f1.00 days`.
- 5h can-use format (aligned style):
  - `f02.1%/hour for next f1.00 hours`.
- Color policy:
  - ANSI on by default; fallback to no-color when non-TTY or `NO_COLOR`.

## Implementation Plan
1. Add display mode state + `B` action in custom prompt layer.
2. Introduce view-model builder for per-profile lines by mode.
3. Implement shared-axis math for 5h and weekly windows.
4. Implement layered bar renderer (base/elapsed/used), with ANSI fallback.
5. Apply fixed-width padding for numeric fields and aligned line formatting.
6. Keep existing sort/use/delete/rename/refresh behavior unchanged.
7. Manual verification in three modes with mixed reset times and missing 5h windows.

## Verification Checklist
- `npm run build` passes.
- In `full`, limits keep extra indent and numeric columns align.
- Press `B` cycles all three modes without losing selection context.
- No forced centering scroll regression from current custom prompt behavior.
- Weekly and 5h can-use strings match exact width/precision format.

## Deferred
- Write final implementation record to `spec/*.md` only after code is fully landed.