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

- [x] Add quick workload tiers `Low/Medium/High`
- [x] Expose current workload tier in the action/help area or another low-noise control surface
- [x] Re-rank recommendations based on projected workload impact while keeping current auto mode as default
- [x] Verify ranking changes stay stable when workload tier is unset
- [x] Update `spec/` with workload-aware routing behavior after MVP lands

## Completed Prompt Panel MVP

- [x] Move profile detail into a prompt-level panel above the options list
- [x] Keep the option list minimal with indicator, profile name, and delta only
- [x] Split comparable numeric values into aligned fields across Delta and Quota views
- [x] Separate `Delta` and `Quota` responsibilities so Quota keeps quota-oriented fields and Delta keeps pacing comparison
- [x] Tune pacing emphasis colors for light/dark terminal themes and bottleneck-only highlighting
- [x] Update `spec/` with the verified prompt panel behavior after MVP lands

## Next MVP: Adaptive Prompt Density

- [ ] Detect when profile count or terminal height makes the full prompt panel too tall to scan comfortably
- [ ] Add a condensed panel density that preserves `profile + last update` but reduces lower-priority detail fields
- [ ] Keep Delta and Quota semantics consistent across normal and condensed densities
- [ ] Define how condensed density behaves when some profiles lack `5H` data
- [ ] Verify the condensed layout still works on narrow remote terminals without reintroducing unreadable spacing
- [ ] Update `spec/` only after the condensed-density MVP is implemented and verified
- [ ] Execute the detailed checklist in `plan/2026-03-20-adaptive-prompt-density-implementation-plan.md`

Current implementation handoff:
- vertical pressure is the first trigger to validate
- first MVP adds exactly one fallback density: `condensed`
- width-aware truncation and multi-stage density are explicitly deferred to later follow-up

## Follow-up Track

- [ ] Keep the Rust plot-mode work in `plan/2026-03-20-ratatui-plot-mode-implementation-plan.md` as the separate visualization stream
- [ ] Add a visual indicator in the table body showing which workload tier most influenced the current ranking
- [ ] Evaluate whether workload tier should become per-session persisted state instead of reset-on-launch
- [ ] Evaluate whether workload tier should surface a short explanatory hint near the current recommendation
- [ ] Revisit whether condensed prompt density needs more than one level once the first adaptive-density MVP ships
