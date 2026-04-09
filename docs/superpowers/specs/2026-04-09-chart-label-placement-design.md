# Chart label placement design

## Problem

Normal chart end labels currently degrade too early. A shorter fallback label can win simply because it fits closer to the anchor, even when the full label would have fit elsewhere.

That creates two user-visible failures:

- the full end label can disappear from the chart even though another side has room
- important leading text such as `[codex 7d]` or `[claude 7d]` can be lost before fallback is truly necessary

## Approved behavior

### Placement priority

Normal chart end labels must use a two-stage placement rule:

1. Try to place the **full end label** first.
2. Search for open space around the anchor in all four directions by varying vertical position and trying both right-side and left-side attachment.
3. Only if the full end label cannot be placed anywhere safely may the renderer try compact or minimal fallback variants.

The practical rule is:

> keep the full label if it fits anywhere around the anchor; only shorten when it fits nowhere

This priority is evaluated **per anchor**. Each label anchor must exhaust its own full-label placement options before that same anchor is allowed to try compact or minimal variants. This spec does **not** require a new global scheduler that forces every anchor in the chart to finish full-label attempts before any other anchor may use a fallback variant.

In this spec, a **safe placement** means a position that satisfies the renderer's existing legal-placement checks for that variant:

- the label stays within graph-area bounds
- the label cells do not collide with occupied plot cells
- the label cells do not collide with blocked or reserved cells
- the connector path satisfies the current legality rules used by end-label placement
- for a full label with an optional reset line, the first-line placement must be legal under these checks; the reset line then follows the existing secondary rule that it may be omitted when vertical room is too tight

If a placement fails any of those checks, it does not count as "fits anywhere safely."

### Search behavior

The renderer should keep the existing collision-avoidance approach of searching nearby rows first, but the search must be organized by **label variant priority first**, not by mixed full-and-compact candidates in one pool.

For this feature, "all four directions" means the renderer must consider placements that end up:

- above-right of the anchor
- below-right of the anchor
- above-left of the anchor
- below-left of the anchor

Within a single variant, it is acceptable to keep the current proximity-based choice:

- prefer smaller vertical displacement from the anchor
- then prefer smaller connector displacement
- then apply the existing left/right candidate ordering

### Fallback behavior

Fallback variants still exist and remain useful, but they are a last resort:

- full label
- compact label
- minimal label

Do not truncate or shorten the full label merely because a shorter label would fit closer.

### Zero-state anchors

No behavior change. Zero-state branch labels such as `reset / no usage yet` keep their current anchor geometry and clipping behavior.

### Reset-line interaction

The existing reset-line behavior remains attached to the full end-label block only.

- If the full label wins placement, it may render its second reset line subject to the existing vertical-room rules.
- If the renderer must fall back to compact or minimal variants, those degraded variants remain single-line only.

## Architecture

The label-placement logic should stay in `src/render/chart.rs`, where end-label candidate generation and placement already live.

Refactor the placement flow so that:

1. candidate generation can be run for exactly one label variant at a time
2. for each anchor, the normal placement pass first exhausts that anchor's full variant before considering any fallback variant for that same anchor
3. fallback variants are only considered after the current anchor's full variant has no legal placement

This keeps the boundary clear:

- label text construction decides what the variants are
- layout decides where a chosen variant can fit
- render draws the chosen result without re-deciding truncation

One useful way to think about the boundary is:

- variant selection input: one anchor plus its ordered variants `[full, compact, minimal]`
- legal placement check output: zero or more in-bounds placements that satisfy the normal collision and connector rules for that variant
- force-placement output: at most one in-bounds placement for the highest-priority remaining variant, even if normal collision rules failed

## Error handling and fallback

- If no safe placement exists for the full variant, try the compact variant.
- If no safe placement exists for the compact variant, try the minimal variant.
- If no safe placement exists for any variant, keep a force-placement fallback rather than dropping the label entirely.
- That force-placement path must still respect variant priority in the same order: full first, then compact, then minimal.
- In this spec, force-placement means reusing the existing fallback pass that may ignore normal collision preferences for occupied, blocked, reserved, or connector-conflict cells, but it must still keep the chosen variant fully within graph-area bounds.
- If a full variant wins only via force-placement, its optional reset second line still follows the existing rule: render it only when vertical room is available; otherwise omit only that second line.
- If even the minimal variant cannot fit fully within graph-area bounds, the label may remain omitted. The renderer must not invent a new truncation mode to squeeze it in.
- Do not introduce partial-prefix truncation such as stripping `[codex 7d]` while pretending the result is still the full variant.

## Testing

Add or update render/layout regressions for:

1. a label whose full text does not fit on the right but does fit on the left, proving the full label is preserved
2. a label whose compact variant would fit closer on the right while the full label still fits on the left, proving full wins before compact
3. a label whose full variant fits nowhere but compact does, proving fallback still works
4. a label whose full and compact variants fit nowhere but minimal does, proving final fallback still works
5. a label whose full, compact, and minimal variants all lack a safe placement in the normal pass, proving the force-placement path still preserves variant priority as full → compact → minimal
6. existing zero-state anchor behavior staying unchanged
7. existing reset-line behavior still attaching only to the full label variant
8. a multi-label conflict case proving variant priority is enforced per anchor without introducing a new global placement phase
9. a label whose minimal variant is wider than graph-area bounds, proving the renderer omits it rather than inventing a new truncation rule

## Scope guardrails

- No new abbreviation scheme beyond the existing compact/minimal variants
- No behavior change to zero-state anchor routing
- No change to chart data derivation
- No new user setting for label-placement strategy
