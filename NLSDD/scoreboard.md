# NLSDD Scoreboard

> Manual fields: `Current item`, `Phase`, `Item commit`, `Blocked by`, `Next refill target`, `Notes`
>
> Auto-refresh fields: `Effective phase`, `Branch HEAD`, `Last probe`, `Latest event`, `Correction count`, `Last activity`, `Noise`, plus the `Recent Codex Threads` appendix via `npm run nlsdd:scoreboard:refresh`
>
> When `NLSDD/state/<execution-id>/lane-<n>.json` exists, refresh tooling treats that lane journal as the primary runtime state source before falling back to thread/session heuristics.
>
> Recommended manual lane phases for multi-lane scheduling: `queued`, `implementing`, `spec-review-pending`, `quality-review-pending`, `correction`, `refill-ready`, `blocked`, `parked`
>
> `nlsdd-self-hosting` uses lanes 1-4 as the initial 4-thread active set; lanes 5-6 are queued follow-up lanes that only enter the active set when a slot opens.

| Execution | Lane | Ownership | Current item | Phase | Effective phase | Item commit | Branch HEAD | Last verification | Last probe | Latest event | Correction count | Last activity | Blocked by | Next refill target | Noise | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| nlsdd-self-hosting | Lane 1 | Scheduler core | Multi-lane / 4-thread schedule helper | queued | queued | `n/a` | `n/a` | `node --test tests/nlsdd-automation.test.js`; `node NLSDD/scripts/nlsdd-suggest-schedule.cjs --execution nlsdd-self-hosting --json` | n/a | n/a | 0 | n/a | none | Scheduler edge cases | none | Initial active set; dispatch first. |
| nlsdd-self-hosting | Lane 2 | Scoreboard integration | Self-hosting scoreboard rows and schedule-facing refresh | queued | queued | `n/a` | `n/a` | `npm run nlsdd:scoreboard:refresh`; `node NLSDD/scripts/nlsdd-suggest-schedule.cjs --execution nlsdd-self-hosting` | n/a | n/a | 0 | n/a | none | Schedule-facing scoreboard wording polish | none | Initial active set; dispatch second. |
| nlsdd-self-hosting | Lane 3 | Rules and communication | Lane-pool + active-cap rules alignment | queued | queued | `n/a` | `n/a` | `rg -n 'active lane count' spec/NLSDD; rg -n '4LSDD' spec/NLSDD; rg -n '4 active lanes' NLSDD` | n/a | n/a | 0 | n/a | none | Execution-level wording cleanup | none | Initial active set; dispatch third. |
| nlsdd-self-hosting | Lane 4 | Regression and CLI surface | Schedule regression coverage and CLI smoke checks | queued | queued | `n/a` | `n/a` | `node --test tests/nlsdd-automation.test.js`; `node NLSDD/scripts/nlsdd-suggest-schedule.cjs --execution nlsdd-self-hosting` | n/a | n/a | 0 | n/a | none | Scoreboard/schedule cross-check coverage | none | Initial active set; dispatch fourth. |
| nlsdd-self-hosting | Lane 5 | Plot-mode execution migration | Plot-mode docs alignment for lane-pool scheduling | queued | queued | `n/a` | `n/a` | `rg -n '4-thread' NLSDD/executions/plot-mode; rg -n 'queued' NLSDD/executions/plot-mode; rg -n 'lane pool' NLSDD/executions/plot-mode` | n/a | n/a | 0 | n/a | wait-slot | Plot-mode overview wording | none | Queued follow-up lane; do not dispatch until one of the first four slots frees up. |
| nlsdd-self-hosting | Lane 6 | Follow-up and coordinator ergonomics | Post-rollout follow-up capture | queued | queued | `n/a` | `n/a` | `sed -n '1,220p' tasks/todo.md` | n/a | n/a | 0 | n/a | wait-slot | Coordinator workflow follow-up | none | Queued follow-up lane; reserve for after the first multi-lane scheduler pass. |
| plot-mode | Lane 1 | Node contract + handoff | Viewer launch confidence / shell messaging | spec-review-pending | correction | `1d29843` | `0f2e02e` | `npm run build`; `node --test tests/plot-handoff.test.js`; `node --test tests/plot-mode-shell.test.js` | 2026-03-21 02:53:39Z · HEAD 0f2e02e · clean | FAIL · Hubble · 2026-03-21 02:33:07Z | 4 | 2026-03-21 02:36:56Z | none | Tighten snapshot builder semantics when real 7d / 5h math evolves | none | Lane branch head currently includes a separate target-artifact cleanup commit after this item commit. |
| plot-mode | Lane 2 | Rust runtime + boundary | Boundary API stabilization | spec-review-pending | correction | `3b62c5b` | `84595d3` | `cargo test --manifest-path rust/plot-viewer/Cargo.toml`; `cargo check --manifest-path rust/plot-viewer/Cargo.toml` | 2026-03-21 02:53:40Z · HEAD 84595d3 · clean | FAIL · Hubble · 2026-03-21 02:33:07Z | 4 | 2026-03-21 02:36:56Z | none | Tighten typed decoding for nested `usage` payload | none | `PanelSections` and shell/body naming now make the seam explicit; branch head also includes artifact cleanup. |
| plot-mode | Lane 3 | Rust chart surface | ASCII 7d curve surface | spec-review-pending | correction | `907cbc7` | `b48d5a9` | `cargo test --manifest-path rust/plot-viewer/Cargo.toml render::chart -- --nocapture`; `cargo check --manifest-path rust/plot-viewer/Cargo.toml` | 2026-03-21 02:53:41Z · HEAD b48d5a9 · clean | FAIL · Hubble · 2026-03-21 02:33:07Z | 4 | 2026-03-21 02:36:56Z | none | Add 5h band, axis labels, and unavailable-band fallback | none | This is a stronger lane-local surface, not the final `Canvas` plot; branch head also includes artifact cleanup. |
| plot-mode | Lane 4 | Rust panels + docs | Panel docs / README regression | spec-review-pending | correction | `12785d1` | `9c05c56` | `node --test tests/plot-readme.test.js`; `cargo test render_panels_builds_visible_summary_and_compare_blocks --manifest-path rust/plot-viewer/Cargo.toml` | 2026-03-21 02:53:41Z · HEAD 9c05c56 · clean | FAIL · Hubble · 2026-03-21 02:33:07Z | 4 | 2026-03-21 02:36:56Z | none | Add panel-specific regression coverage once wording settles | none | Visible Summary / Compare panel surface is already landed in the prior lane item; branch head also includes artifact cleanup. |

## Recent Codex Threads

> Auto-refreshed from `~/.codex/state_5.sqlite` for this repo cwd.

| Nickname | Role | Thread ID | Updated |
| --- | --- | --- | --- |
| Banach | worker | `019d0e3d-f691-71c0-99eb-243e88f05067` | 2026-03-21 02:36:56Z |
| Tesla | worker | `019d0e3d-f8a6-78f1-8e56-21fd60b4c6b2` | 2026-03-21 02:36:54Z |
| Archimedes | worker | `019d0e3d-f786-7581-ba6c-817ff3cb4ac3` | 2026-03-21 02:36:16Z |
| Feynman | worker | `019d0e3d-f5aa-7000-a7e7-b82f882ff597` | 2026-03-21 02:36:12Z |
| Meitner | worker | `019d0c86-9340-7152-ba50-3b680080ba29` | 2026-03-21 02:33:26Z |
| Erdos | worker | `019d0c86-91d2-7652-aec8-f4e71f51e685` | 2026-03-21 02:33:20Z |
| Hubble | worker | `019d0c86-927d-79f1-a437-8cc0f491e0de` | 2026-03-21 02:33:07Z |
| Russell | worker | `019d0c57-5949-7b21-a586-991b522090bf` | 2026-03-21 02:32:57Z |
