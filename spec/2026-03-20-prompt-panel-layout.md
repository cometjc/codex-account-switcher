# Prompt Panel Layout Spec

Date: 2026-03-20

## Implemented Scope
- Reworked the interactive profile manager into a split layout:
  - a prompt-level detail panel showing all profiles
  - a minimal selectable options list below it
- The prompt-level detail panel renders one block per profile.
- In each profile block:
  - the first line shows `profile` and `last`
  - later lines show aligned weekly and 5h detail rows
- The selectable options list no longer duplicates the full profile detail block.
- Each selectable option now only shows:
  - indicator
  - profile name
  - delta

## UX Notes
- Reading and choosing are now separated:
  - the detail panel is for scanning all profiles
  - the options list is for fast navigation and action targeting
- The detail panel uses a vertical layout rather than a single wide row table.
- Weekly and 5h values are still aligned across profile blocks, but the first line is intentionally non-tabular.
- The option list is shorter and easier to scan on narrow remote terminals because it no longer embeds the full detail table.

## Technical Changes
- `src/lib/root-panel-layout.ts`
  - Added prompt-level multi-profile detail panel rendering.
  - Added aligned weekly/5h row formatting for panel blocks.
- `src/lib/root-option-layout.ts`
  - Added minimal selectable option label rendering.
- `src/lib/prompts/action-select.ts`
  - Added `panelText` support so prompt-level detail content can render above the paged options list.
- `src/commands/root.ts`
  - Switched from multi-line option rows to prompt panel + minimal options architecture.
  - Keeps selection and action flow unchanged while moving detail rendering into the prompt panel.

## Verification
- Build: `npm run build` passed.
- Automated checks:
  - `node --test tests/root-panel-layout.test.js` passed
  - `node --test tests/root-option-layout.test.js` passed
  - `node --test tests/root-table-layout.test.js` passed
  - `node --test tests/workload-tier.test.js` passed
  - `node --test tests/entrypoints.test.js` passed
- Manual verification is still recommended in a real remote terminal to confirm the new panel/list split feels good during interactive navigation.
