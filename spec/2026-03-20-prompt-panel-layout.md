# Prompt Panel Layout Spec

Date: 2026-03-20

## Implemented Scope
- Reworked the interactive profile manager into a split layout:
  - a prompt-level detail panel showing all profiles
  - a minimal selectable options list below it
- The prompt-level detail panel renders one block per profile.
- In each profile block:
  - the first line shows `indicator + profile` and `last update: ... ago`
  - later lines use indented aligned weekly and 5h detail rows
- The prompt-level detail rows now render as:
  - `W:  📊 ... left  🔄 in ...  (...) Pacing ...`
  - `5H: 📊 ... left  🔄 in ...  (...) Pacing ...`
- The `(...)` value after `🔄 in` is the remaining reset-time percentage for that window.
- The compact panel uses whitespace alignment instead of `|` separators.
- Numeric values inside `📊`, `🔄 in`, and `Pacing` are right-aligned for easier scanning.
- This numeric field-splitting rule also applies to `Quota` rows: values such as `6.7d`, `97% left`, `+1.1%`, and `-55.4%` are aligned as separate numeric fields instead of being treated as one raw phrase.
- In `Delta` mode, `Pacing` is also split into separate fields for the prefix, numeric delta, and qualitative label so `%` values and labels align independently across rows.
- Reset information in `Delta` mode is split into `🔄 in`, time, and remaining-percent fields with tightened spacing, rather than padded as one loose phrase.
- In `Delta` mode, `Pacing` includes the qualitative suffix again, such as `Overuse` or `Under`.
- In `Delta` mode, the colored emphasis is moved from the whole `Pacing` label to the bracketed pacing payload: `Pacing [+76.6% Overuse]`.
- The colored pacing payload is padded to a fixed visible width so the highlighted block stays visually uniform.
- Overuse backgrounds now use darker warning tones with explicit foreground color so they stay readable on both light and dark terminal themes.
- `last update: ... ago` is rendered as a dimmed suffix when ANSI color is enabled.
- Rows for unavailable limits are hidden instead of printing `N/A`.
- Profile text in the panel is no longer recommendation-colored.
- The old `Bar ...  Workload ...` separator line is no longer rendered above the options.
- The compact textual panel is only used in `Delta` mode.
- In `Quota` mode, the prompt panel switches back to the bar-based multi-line rows so quota bars remain visible.
- In `Quota` mode, the prompt panel omits the delta/drift comparison block and keeps only quota-oriented fields: bar, time to reset, and usage left.
- `Pacing` background color is only applied to the adopted bottleneck row, not every rendered row.
- The selectable options list no longer duplicates the full profile detail block.
- Each selectable option now only shows:
  - indicator
  - profile name
  - delta
- The action area exposes an explicit `[Q]uit` shortcut.

## UX Notes
- Reading and choosing are now separated:
  - the detail panel is for scanning all profiles
  - the options list is for fast navigation and action targeting
- The detail panel uses a vertical layout rather than a single wide row table.
- Weekly and 5h values are still aligned across profile blocks, but the first line is intentionally non-tabular.
- `Pacing` is visually emphasized with background color intensity when ANSI color is enabled.
- The option list is shorter and easier to scan on narrow remote terminals because it no longer embeds the full detail table.

## Technical Changes
- `src/lib/root-panel-layout.ts`
  - Added prompt-level multi-profile detail panel rendering.
  - Added indented aligned weekly/5h row formatting for panel blocks.
  - Added hidden-row behavior for unavailable limits.
  - Added ANSI-safe width measurement so colored `Pacing` labels stay aligned.
- `src/lib/root-option-layout.ts`
  - Added minimal selectable option label rendering.
- `src/lib/prompts/action-select.ts`
  - Added `panelText` support so prompt-level detail content can render above the paged options list.
- `src/commands/root.ts`
  - Switched from multi-line option rows to prompt panel + minimal options architecture.
  - Keeps selection and action flow unchanged while moving detail rendering into the prompt panel.
  - Converts reset-time remaining percentage into prompt detail text.
  - Removes recommendation coloring from the profile header while keeping `Pacing` color emphasis.
  - Drops the old bar/workload status separator from the prompt choice list.
  - Routes `Delta` mode through the compact text panel and `Quota` mode through the bar-based rows.
  - Restores a dedicated `Q` quit action in the prompt.

## Verification
- Build: `npm run build` passed.
- Automated checks:
  - `node --test tests/root-panel-layout.test.js` passed
  - `node --test tests/root-option-layout.test.js` passed
  - `node --test tests/root-table-layout.test.js` passed
  - `node --test tests/workload-tier.test.js` passed
  - `node --test tests/entrypoints.test.js` passed
- Manual verification is still recommended in a real remote terminal to confirm the new panel/list split feels good during interactive navigation.
