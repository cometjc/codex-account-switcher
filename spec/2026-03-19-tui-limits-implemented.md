# TUI Limits Visualization Spec (Implemented)

Date: 2026-03-19

## Implemented Scope
- Added display mode cycle on `<B>`:
  - `full` -> `5h-only` -> `weekly-only` -> `full`.
- Added mode indicator in prompt message and cache header.
- Added aligned time-axis rendering for same limit type across profiles.
- Added layered bar semantics:
  - outside-window segment: `-`
  - in-window remaining: `░`
  - elapsed-time segment: ANSI background (fallback `=` when no color)
  - usage overlay: `█`
- Kept multi-line profile rendering in `full` mode.
- Added one-line profile rendering in `5h-only` and `weekly-only` modes.
- Increased limit-line indent depth under profile lines.
- `full` mode now uses legacy usage-only bar (no time-axis, no time labels).
- Updated `can use` wording and formatting:
  - weekly: `Can use XX.X%/day for next XXX.XX days`
  - 5h: `Can use XX.X%/hour for next XXX.XX hours`
  - numeric fields use fixed-width padding.
- Date/time display uses `MM/DD HH:MM` (for weekly) and `HH:MM` (for 5h).
- Aligned bracket column in time-axis views by padding 5h time labels.
- Split the bar-adjacent summary into two explicit fields:
  - `Time to reset`: remaining time only
  - `Usage Left`: remaining quota only, formatted as `XX% left`
- Cache status moved to each profile title suffix:
  - `fresh` when just refreshed by API
  - otherwise `MMm:SSs ago`
- Refresh behavior changed:
  - `u`: refresh selected profile only
  - `U`: refresh all profiles
  - `Space`: redraw only (age/bar updates, no API refresh)
- Added workload-aware routing controls:
  - `W` cycles workload tier: `Auto` -> `Low` -> `Medium` -> `High` -> `Auto`
  - current workload tier is shown in the action/help area
  - routing score adjusts by workload tier while leaving `Auto` as the default path

## Technical Changes
- `src/commands/root.ts`
  - Added `DisplayMode` and `mode` action handling.
  - Added axis computation (`computeAxes`, `computeAxis`) using window start/end.
  - Added mode-specific item decoration (`decorateItemForMode`, `formatCompactLimit`).
  - Replaced simple percent bar with aligned layered bar (`renderAlignedBar`).
  - Added 5h/weekly edge time labels:
    - 5h: `HH:MM`
    - weekly: `MM/DD HH:MM`
  - Added color policy helper (`useColor`) with `NO_COLOR` support.
  - Added refresh scope targeting by selected account id.
  - Added per-profile cache-status suffix formatter.
  - Added manual date formatting helpers (`MM/DD HH:MM`).
  - Added workload-tier state and score-weight switching for routing recommendations.
- `src/lib/prompts/action-select.ts`
  - Uses boundary-follow scroll behavior (no auto-centering).
  - Added stable rendering without private package subpath imports.
  - Added key matcher for `U` (shift), `space`, and normal keys.
- `src/lib/limits/usage-limit-service.ts`
  - Added `cacheOnly` option to prevent implicit TTL-driven API refresh during redraw.

## Verification
- Build: `npm run build` passed.
- CLI bootstrap: `node dist/index.js --help` passed previously after prompt-layer fix.
- Automated checks:
  - `node --test tests/workload-tier.test.js` passed
  - `node --test tests/entrypoints.test.js` passed
- Manual TUI behavior validation remains interactive and should be checked in a real terminal:
  - mode cycling with `<B>`
  - workload tier cycling with `<W>`
  - full/single-line rendering
  - scroll behavior under long lists
  - color fallback with `NO_COLOR=1`
