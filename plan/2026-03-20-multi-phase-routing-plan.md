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

## Completed Adaptive Prompt Density MVP

- [x] Detect when profile count or terminal height makes the full prompt panel too tall to scan comfortably
- [x] Add a condensed panel density that preserves `profile + last update` but reduces lower-priority detail fields
- [x] Keep Delta and Quota semantics consistent across normal and condensed densities
- [x] Define how condensed density behaves when some profiles lack `5H` data
- [x] Verify the condensed layout still works on narrow remote terminals without reintroducing unreadable spacing
- [x] Update `spec/` after the condensed-density MVP is implemented and verified
- [x] Execute the detailed checklist in the adaptive-density implementation plan

Completed MVP notes:
- vertical pressure is now the first density trigger
- the first shipped fallback density is `condensed`
- `Delta` and `Quota` both keep their mode semantics under `condensed`
- missing `5H` data now keeps a stable two-line condensed block

## Width-Aware Compression Follow-up

- [x] Evaluate whether width-aware prompt compression should be the next MVP
- [x] Confirm with the user whether narrow-width handling still needs more work
- [x] Defer this follow-up because the current narrow-width behavior is already satisfactory

Decision note:
- width-aware prompt compression is intentionally not scheduled right now
- current prompt behavior is accepted as good enough for the user's remote-terminal usage
- revisit only if future real usage reintroduces wrapping or alignment pain

## Follow-up Track

- [ ] Keep the Rust plot-mode work in `plan/2026-03-20-ratatui-plot-mode-implementation-plan.md` as the separate visualization stream
- [ ] Add a visual indicator in the table body showing which workload tier most influenced the current ranking
- [ ] Evaluate whether workload tier should become per-session persisted state instead of reset-on-launch
- [ ] Evaluate whether workload tier should surface a short explanatory hint near the current recommendation
- [ ] Revisit whether prompt density eventually needs more than one condensed level if future real usage shows the current layout is no longer enough
