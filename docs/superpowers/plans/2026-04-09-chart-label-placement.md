# Chart Label Placement Plan

See design spec: `docs/superpowers/specs/2026-04-09-chart-label-placement-design.md`

## Review / verification notes

- **Live pane checked after hot reload:** Yes, after `make test` completed and installed the binary.
- **Pane id used:** `AS:0.0`
- **Observed full-label result:** Full labels visible — `[copilot 30d] teamt5-it 95% ~hit 172.4h` and `[claude 7d] CC 78%/23% hit 2.1h` both present with forecast suffixes in the live pane. No qualifying live `comet.jc`/`Beta` example in the current session data.
- **Observed zero-state result:** Zero-state label `[codex] acct reset / no usage yet` present and unchanged at origin.
- **Reset-line status on branch:** Not present on this branch. The branch uses the older single-line `forecast_label: Option<&'a str>` path, not a multiline reset-line contract. Audited and regression-tested via `format_end_label_keeps_forecast_suffix_on_full_variant`, `compact_end_label_variants_omit_forecast_suffix`, and `layout_end_labels_force_fallback_keeps_full_label_with_forecast_suffix`.
- **Fixture concern (reported as DONE_WITH_CONCERNS):** The `layout_end_labels_force_fallback_keeps_full_label_with_forecast_suffix` test in the plan spec used `blocked = {(0,3),(21,3)}`. The exclusion expansion of `(21,3)` adds `{(20,3),(21,3),(22,3)}` to the exclusion zone, making every valid 51-char placement impossible in a 64-wide single-row graph. Corrected to `blocked = {(0u16,3u16)}` (single left-side blocker) so the full label fits at `x=13`. The test intent (force-pass prefers full-with-forecast over compact-without-forecast) is correctly exercised.
