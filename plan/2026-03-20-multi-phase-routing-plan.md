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

## Completed Workload Tier Influence Hint MVP

- [x] Add one concise hint that explains the currently active workload tier bias
- [x] Place the hint in a low-noise shared prompt surface rather than repeating it on every profile row
- [x] Keep the options list unchanged so `Delta` and `Quota` still scan quickly
- [x] Verify the hint works in both `Delta` and `Quota` modes without reintroducing prompt clutter
- [x] Update `spec/` after the hint MVP is implemented and verified
- [x] Execute the detailed checklist in the workload-tier influence hint plan

Completed MVP notes:
- workload tier bias is now visible in the shared status line
- hint wording stays mode-agnostic so it works in both `Delta` and `Quota`
- option rows remain minimal and do not inherit explanatory text

## Completed Workload Influence Indicator MVP

- [x] Add a compact indicator showing which window most influenced the current recommendation
- [x] Keep the indicator readable without turning the table or panel back into a verbose explanation surface
- [x] Ensure the indicator behaves consistently in both `Delta` and `Quota` modes
- [x] Update `spec/` after the indicator MVP is implemented and verified

Completed MVP notes:
- option labels now carry a compact `[W]` / `[5H]` influence marker
- the marker stays short enough to preserve the quick-scan option list
- `Delta` keeps its delta value while `Quota` stays free of pacing residue

## Completed Workload Tier Persistence MVP

- [x] Evaluate whether workload tier should become per-session persisted state instead of reset-on-launch
- [x] Decide whether persistence should be opt-in, always-on, or remain intentionally ephemeral
- [x] Keep the current UX understandable if persistence is added later
- [x] Update `spec/` after the persistence MVP is implemented and verified

Completed MVP notes:
- workload tier now persists in local UI state under `~/.codex`
- invalid or missing persisted state safely falls back to `Auto`
- the existing help and status surfaces continue to explain the active tier after restore

## Completed Table-Body Influence Indicator MVP

- [x] Add a compact influence indicator in the detailed table/body view, not only in the option list
- [x] Keep the table/body indicator aligned with existing `Delta` and `Quota` semantics
- [x] Ensure the extra marker does not reintroduce noisy or overly wide row layouts
- [x] Update `spec/` after the table-body indicator MVP is implemented and verified

Completed MVP notes:
- detail rows now use `W:*` / `5H:*` on the adopted bottleneck source
- option list and prompt body now share the same influence story across two levels of detail
- the indicator remains short enough to preserve the current row rhythm

## Tier vs Window Influence Decision

- [x] Decide whether the UI should distinguish workload-tier influence from raw window bottleneck influence
- [x] Keep workload-tier bias in the shared status line and keep row/body markers focused on window truth
- [x] Avoid duplicating explanation text across the status line, option list, and detail rows

Decision note:
- the UI intentionally keeps `[W]` / `[5H]` and `W:*` / `5H:*` as window-truth markers only
- workload tier remains a shared status-line explanation rather than becoming a second row-level marker
- this follow-up is intentionally closed unless future usage shows that tier influence is still too opaque

## Follow-up Track

- [x] Keep the Rust plot-mode work in `plan/2026-03-20-ratatui-plot-mode-implementation-plan.md` as the separate visualization stream
- [x] Add a visual indicator in the table body showing which workload tier most influenced the current ranking
- [x] Revisit whether prompt density eventually needs more than one condensed level if future real usage shows the current layout is no longer enough

Follow-up closure note:
- Rust plot-mode work was kept as the separate visualization stream and later reactivated through `plan/2026-03-22-plot-mode-reactivation-plan.md`.
- The table/body influence follow-up is closed by the shipped `W:*` / `5H:*` body markers plus the later tier-vs-window decision: row/body markers stay focused on window truth, while workload tier remains a shared status-line explanation.
- Extra condensed-density levels remain intentionally deferred; current remote-terminal usage has not produced evidence that the shipped `condensed` fallback is insufficient.

## Plot Mode Lane Planning

- [x] Decide that plot-mode sub-agent work should run through stable non-overlapping lanes rather than ad-hoc refill tasks
- [x] Create a fixed lane overview plus lane plans covering Node handoff, Rust runtime/state, Rust chart surface, and Rust panels/docs
- [x] Update sub-agent guardrails so refills stay inside lane-local MVC steps and only create a new lane plan when a lane is exhausted
- [x] Promote the workflow into repo-native `NLSDD` operating rules instead of continuing to reference the original SDD skill name
- [x] Add explicit rules for per-lane worktrees, per-item implementer commits, commit-diff-based review, and coordinator-owned tracking updates
- [x] Re-cut the lane relationship so Lane 4 panel visibility depends on a Lane 2 boundary item instead of ad-hoc scope expansion
- [x] Execute the first lane round and prove that all 4 lanes can complete one reviewable item with lane-local correction loops
- [x] Validate that Lane 2 can unblock Lane 4 through a boundary seam rather than by widening Lane 4 scope
- [x] Land the first visible plot viewer panel surface without breaking the lane ownership split
- [x] Migrate the repo-native workflow from `4LSDD` naming into centralized `NLSDD` docs, scoreboard, and communication rules
- [x] Add autopilot refill, lane probe, and scoreboard rules to reduce coordinator bottlenecks
- [x] Start treating Rust `target/` churn as workflow noise, with `.gitignore` plus lane hygiene cleanup instead of reviewer confusion
- [x] Re-plan the current 4-active-lane set from live NLSDD runtime/manual state instead of preserving stale second-round review labels
- [x] Re-cut the next plot-mode 4a active set around the recovery baseline: Lane 2 + Lane 3 + Lane 4 + Lane 5, with Lane 1 parked
- [x] Promote a dedicated docs/operator-flow lane instead of inventing overlapping work just to keep four slots busy
- [x] Capture that stale lane journals must be refreshed before the next dispatch round so runtime tooling does not override the new manual plan

Decision note:
- plot-mode parallel work now follows an NLSDD execution with 4 active lanes
- new sub-agent assignments should start from the current lane plans instead of free-form task slicing
- lane progress should be reported in lane-local MVC terms so the coordinator can refill predictably
- future execution should use `NLSDD` as the workflow name, not `$subagent-driven-development`
- the next 4-active-lane plan parks Lane 1, reactivates Lane 2 for runtime navigation/focus flow, keeps Lane 3/4 active, and adds Lane 5 for docs/operator flow
