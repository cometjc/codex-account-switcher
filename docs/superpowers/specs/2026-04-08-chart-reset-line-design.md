# Chart reset line design

## Problem

When a chart item has reached quota exhaustion, the chart should surface how long remains until reset directly in the chart label area instead of forcing the user to infer it elsewhere.

The new behavior applies to **normal chart end labels only**. It does **not** apply to zero-state anchors such as `reset / no usage yet`.

## Approved behavior

### Trigger

Show an additional `reset in <TIME>` line when either of these is true for a chart item:

- its raw 7d usage value is **>= 100**
- its raw 5h usage value is **>= 100**

If both 7d and 5h are 100%, prefer the **7d** reset countdown.

If both are 100% but the 7d reset cannot be resolved or formatted while the 5h reset can, fall back to the **5h** reset countdown. If neither reset can be resolved, do not render the second line.

`<TIME>` uses the repo's existing short duration format helper rather than introducing a new formatter.

### Normal chart end labels

Keep the existing first line unchanged.

When the trigger condition is met, render a second line directly below the existing label:

```text
[codex 7d] comet 100%/44%
reset in 3h 12m
```

This second line belongs to the same label block and should participate in the same collision-avoidance behavior as the first line.

### Zero-state anchors

No behavior change. Zero-state anchors keep their current branch geometry and do not gain a `reset in <TIME>` line from this feature.

## Data flow

1. Reuse reset metadata from `usage::UsageWindow.reset_after_seconds`, which is already the source used by the detail pane via `format_duration_short(...)`.
2. Extend `app_data::ProfileChartData` with optional reset-display inputs for the weekly and 5h windows so chart preparation does not need to re-open raw usage payloads during rendering.
3. Map those fields into `render::ChartSeries` inside `app::build_chart_state`, alongside the existing `last_seven_day_percent` and `five_hour_used_percent` values.
4. Add a derived render-layer contract equivalent to `Option<ResetLineDisplay>` on `ChartSeries`, where `ResetLineDisplay` contains:
   - the chosen source (`weekly` or `five_hour`)
   - the already-formatted text `reset in <TIME>`
5. Resolve that derived value from the raw weekly/5h percentages already used for labels:
   - trigger when raw 7d or 5h usage is `>= 100.0`
   - prefer weekly when both qualify
   - fall back to 5h if weekly qualifies but its reset countdown cannot be rendered
6. Feed the derived value only into end-label rendering.

## Error handling and fallback

- If reset metadata is missing or cannot be formatted, do not render the second line.
- Do not invent placeholder text like `reset in ?`.
- Do not change non-100% label behavior.
- If there is not enough safe vertical room for a second line, preserve the existing first line and omit only the `reset in <TIME>` line.
- If compact fallback label variants are in use, the `reset in <TIME>` line belongs only to the full multi-line label. Any degraded compact/minimal variant drops the second line first.

## Testing

Add render regressions for:

1. a normal end label with 7d>=100 showing a second line
2. a normal end label with 5h>=100 showing a second line
3. a label with both 7d and 5h at >=100 preferring the 7d reset
4. a label with both windows at >=100 where 7d reset is unavailable and 5h reset wins
5. zero-state anchors remaining unchanged
6. overlapping or neighboring labels still rendering legibly after the new second line is introduced

## Scope guardrails

- No new popup, tooltip, or side panel
- No new duration format
- No behavior change for labels below 100%
