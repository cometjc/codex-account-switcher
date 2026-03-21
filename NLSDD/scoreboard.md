# NLSDD Scoreboard

> This tracked scoreboard keeps only coordinator-owned manual fields. Auto-derived lane state lives in `NLSDD/state/scoreboard.runtime.md`.
>
> Runtime scoreboard output: `NLSDD/state/scoreboard.runtime.md` via `npm run nlsdd:scoreboard:refresh`
>
> When `NLSDD/state/<execution-id>/lane-<n>.json` exists, refresh tooling treats that lane journal as the primary runtime state source before falling back to thread/session heuristics.
>
> Recommended manual lane phases for multi-lane scheduling: `queued`, `implementing`, `spec-review-pending`, `quality-review-pending`, `correction`, `refill-ready`, `blocked`, `parked`
>
> `nlsdd-self-hosting` uses lanes 1-4 as the initial 4-thread active set; lanes 5-6 are queued follow-up lanes that only enter the active set when a slot opens.

| Execution | Lane | Ownership | Current item | Phase | Item commit | Last verification | Blocked by | Next refill target | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| nlsdd-self-hosting | Lane 1 | Scheduler core | Multi-lane / 4-thread schedule helper | queued | `n/a` | `node --test tests/nlsdd-automation.test.js`; `node NLSDD/scripts/nlsdd-suggest-schedule.cjs --execution nlsdd-self-hosting --json` | none | Scheduler edge cases | Initial active set; dispatch first. |
| nlsdd-self-hosting | Lane 2 | Scoreboard integration | Self-hosting scoreboard rows and schedule-facing refresh | queued | `n/a` | `npm run nlsdd:scoreboard:refresh`; `node NLSDD/scripts/nlsdd-suggest-schedule.cjs --execution nlsdd-self-hosting` | none | Schedule-facing scoreboard wording polish | Initial active set; dispatch second. |
| nlsdd-self-hosting | Lane 3 | Rules and communication | Lane-pool + active-cap rules alignment | queued | `n/a` | `rg -n 'active lane count' spec/NLSDD; rg -n '4LSDD' spec/NLSDD; rg -n '4 active lanes' NLSDD` | none | Execution-level wording cleanup | Initial active set; dispatch third. |
| nlsdd-self-hosting | Lane 4 | Regression and CLI surface | Schedule regression coverage and CLI smoke checks | queued | `n/a` | `node --test tests/nlsdd-automation.test.js`; `node NLSDD/scripts/nlsdd-suggest-schedule.cjs --execution nlsdd-self-hosting` | none | Scoreboard/schedule cross-check coverage | Initial active set; dispatch fourth. |
| nlsdd-self-hosting | Lane 5 | Plot-mode execution migration | Plot-mode docs alignment for lane-pool scheduling | queued | `n/a` | `rg -n '4-thread' NLSDD/executions/plot-mode; rg -n 'queued' NLSDD/executions/plot-mode; rg -n 'lane pool' NLSDD/executions/plot-mode` | wait-slot | Plot-mode overview wording | Queued follow-up lane; do not dispatch until one of the first four slots frees up. |
| nlsdd-self-hosting | Lane 6 | Follow-up and coordinator ergonomics | Post-rollout follow-up capture | queued | `n/a` | `sed -n '1,220p' tasks/todo.md` | wait-slot | Coordinator workflow follow-up | Queued follow-up lane; reserve for after the first multi-lane scheduler pass. |
| plot-mode | Lane 1 | Node contract + handoff | Viewer launch confidence / shell messaging | spec-review-pending | `1d29843` | `npm run build`; `node --test tests/plot-handoff.test.js`; `node --test tests/plot-mode-shell.test.js` | none | Tighten snapshot builder semantics when real 7d / 5h math evolves | Lane branch head currently includes a separate target-artifact cleanup commit after this item commit. |
| plot-mode | Lane 2 | Rust runtime + boundary | Boundary API stabilization | spec-review-pending | `3b62c5b` | `cargo test --manifest-path rust/plot-viewer/Cargo.toml`; `cargo check --manifest-path rust/plot-viewer/Cargo.toml` | none | Tighten typed decoding for nested `usage` payload | `PanelSections` and shell/body naming now make the seam explicit; branch head also includes artifact cleanup. |
| plot-mode | Lane 3 | Rust chart surface | ASCII 7d curve surface | spec-review-pending | `907cbc7` | `cargo test --manifest-path rust/plot-viewer/Cargo.toml render::chart -- --nocapture`; `cargo check --manifest-path rust/plot-viewer/Cargo.toml` | none | Add 5h band, axis labels, and unavailable-band fallback | This is a stronger lane-local surface, not the final `Canvas` plot; branch head also includes artifact cleanup. |
| plot-mode | Lane 4 | Rust panels + docs | Panel docs / README regression | spec-review-pending | `12785d1` | `node --test tests/plot-readme.test.js`; `cargo test render_panels_builds_visible_summary_and_compare_blocks --manifest-path rust/plot-viewer/Cargo.toml` | none | Add panel-specific regression coverage once wording settles | Visible Summary / Compare panel surface is already landed in the prior lane item; branch head also includes artifact cleanup. |
