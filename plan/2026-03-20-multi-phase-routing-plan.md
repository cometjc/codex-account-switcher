# Multi-Phase Routing Plan (2026-03-20)

## Phase 1 - UI Relationship Redesign
- Switch to per-profile multi-line block layout.
- Show Weekly and 5hr side-by-side in a dual-compare pattern.
- Add explicit bottleneck marker (`<- Bottleneck`) on the worse window.
- Keep existing actions/hotkeys unchanged.

## Phase 2 - Recommendation Engine (Auto)
- Introduce routing score focused on maximizing 7d utilization.
- Use 5hr as smoothing constraint (not hard blocker).
- Include small switch-cost penalty to reduce churn.
- Sort by recommendation score with stable tie behavior.

## Phase 3 - Pacing Status Semantics
- Rename summary concept to `Pacing Status`.
- Display concise status text:
  - `+x.x% Overuse [W|5H]`
  - `-x.x% Under [W|5H]`
  - `Unused, good [W|5H]`
- Add recommendation label tiers: `Strong/Good/Neutral/Caution/Risky`.

## Phase 4 - Visual Guidance
- Apply recommendation color to profile summary row.
- Apply drift-based background color on Delta bars.
- Keep ANSI-off mode fully readable with text-only cues.

## Phase 5 - Workload-Aware Extension (Future)
- Add quick workload tiers `Low/Medium/High`.
- Re-rank recommendations based on projected workload impact.
- Keep this optional and backward compatible with auto mode.
