# Multi-Phase Routing Plan (2026-03-20)

## Completed Foundations

- [x] Phase 1 - UI relationship redesign
  - per-profile multi-line block layout
  - weekly and 5h dual-compare presentation
  - explicit bottleneck marker
  - existing actions/hotkeys preserved
- [x] Phase 2 - recommendation engine
  - score centered on 7d utilization
  - 5h used as smoothing constraint
  - switch-cost penalty added
  - stable ranking behavior kept
- [x] Phase 3 - pacing status semantics
  - `Pacing Status` naming adopted
  - concise status text added
  - recommendation label tiers added
- [x] Phase 4 - visual guidance
  - recommendation color on summary row
  - drift-based delta-bar background
  - ANSI-off text readability kept

## Next MVP

- [ ] Add quick workload tiers `Low/Medium/High`
- [ ] Expose current workload tier in the action/help area or another low-noise control surface
- [ ] Re-rank recommendations based on projected workload impact while keeping current auto mode as default
- [ ] Verify ranking changes stay stable when workload tier is unset
- [ ] Update `spec/` with workload-aware routing behavior after MVP lands

## Follow-up Track

- [ ] Keep the Rust plot-mode work in `plan/2026-03-20-ratatui-plot-mode-implementation-plan.md` as the separate visualization stream
