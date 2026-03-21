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
| nlsdd-self-hosting | Lane 1 | Scheduler core | Multi-lane / 4-thread schedule helper | parked | `n/a` | `node --test tests/nlsdd-automation.test.js`; `node NLSDD/scripts/nlsdd-suggest-schedule.cjs --execution nlsdd-self-hosting --json` | none | Scheduler edge cases | nlsdd-go: remediation round for reducer drift, read-only helpers, and execution-insights lifecycle |
| nlsdd-self-hosting | Lane 2 | Scoreboard integration | Self-hosting scoreboard rows and schedule-facing refresh | parked | `1613f3c` | `npm run nlsdd:scoreboard:refresh`; `node NLSDD/scripts/nlsdd-suggest-schedule.cjs --execution nlsdd-self-hosting` | none | Schedule-facing scoreboard wording polish | Lane 2 scoreboard integration remains stable on the refreshed baseline; keep it parked until a concrete new gap appears. |
| nlsdd-self-hosting | Lane 3 | Rules and communication | Lane-pool + active-cap rules alignment | parked | `ef5f71e` | `rg -n "projection-only\|execution-insights\|adopted global learnings\|read-only\|tracked scoreboard\|lane-plan status\|graduate\|bounded follow-up" spec/NLSDD NLSDD/AGENTS.md` | none | Execution-level wording cleanup | Lane 3 wording is already converged after ef5f71e; park the lane instead of inventing docs churn. |
| nlsdd-self-hosting | Lane 4 | Regression and CLI surface | Wait for a fresh regression/CLI surface item after the cross-check coverage landed | parked | `71bc61b` | `node --test tests/nlsdd-automation.test.js`; `git diff --check` | none | Re-open only if a fresh regression/CLI surface gap appears beyond the accepted cross-check coverage | Lane 4 accepted the cross-check coverage and is now parked pending a genuinely new regression/CLI surface gap. |
| nlsdd-self-hosting | Lane 5 | Plot-mode execution migration | Plot-mode docs alignment for lane-pool scheduling | parked | `n/a` | `rg -n "4-thread\\|queued\\|lane pool" NLSDD/executions/plot-mode` | wait-slot | Plot-mode overview wording | nlsdd-go: remediation round for reducer drift, read-only helpers, and execution-insights lifecycle |
| nlsdd-self-hosting | Lane 6 | Follow-up and coordinator ergonomics | Post-rollout follow-up capture | parked | `3c2a967` | `sed -n '1,260p' tasks/todo.md` | none | Coordinator workflow follow-up | nlsdd-go: remediation round for reducer drift, read-only helpers, and execution-insights lifecycle |
| nlsdd-self-hosting | Lane 7 | NLSDD meta-optimization | Wait for a fresh scheduler/runtime truth finding after the accepted warning cleanup | parked | `9c391aa` | `node --test tests/nlsdd-automation.test.js`; `npm run nlsdd:scoreboard:refresh`; `node NLSDD/scripts/nlsdd-suggest-schedule.cjs --execution nlsdd-self-hosting`; `node NLSDD/scripts/nlsdd-build-dispatch-plan.cjs --execution nlsdd-self-hosting --dry-run --json`; `npm run build` | none | Re-open only when a new scheduler/runtime truth finding yields a concrete helper, docs delta, or regression | Lane 7 accepted the warning cleanup and is now parked pending a genuinely new finding. |
| plot-mode | Lane 1 | Node contract + handoff | Future shell polish only if Rust viewer launch UX changes again | parked | `baa7b8e` | `npm run build`; `node --test tests/plot-snapshot.test.js tests/plot-handoff.test.js`; `node --test tests/plot-mode-shell.test.js` | none | Re-activate only if plot launch/retry UX changes enough to justify fresh shell work | Current plot-mode shell and handoff tests already align with the recovery baseline, so the truthful next phase is parked rather than pseudo-active. |
| plot-mode | Lane 2 | Rust runtime + boundary | Wait for a fresh runtime-owned item after accepted compare payload seam | parked | `d361653` | `cargo test --manifest-path rust/plot-viewer/Cargo.toml`; `cargo check --manifest-path rust/plot-viewer/Cargo.toml` | none | Re-open only when a concrete post-seam runtime or navigation item exists | The compare-payload seam is already accepted in d361653; keep Lane 2 parked until a fresh runtime-owned item exists. |
| plot-mode | Lane 3 | Rust chart surface | Chart compatibility with richer focus and profile cycling | parked | `c5f6c26` | `cargo test --manifest-path rust/plot-viewer/Cargo.toml render::chart -- --nocapture`; `cargo check --manifest-path rust/plot-viewer/Cargo.toml` | none | Only widen beyond compatibility work if plot UX still needs richer chart interaction after Lane 2 lands | Chart-local behavior and regressions already cover non-Chart focus with divergent selected/current labels; no additional lane-local diff was needed. |
| plot-mode | Lane 4 | Rust panels + docs | Wait for a fresh panel-local item after adopted-target emphasis landed | parked | `6bb1fba` | `cargo test render_panels_builds_visible_summary_and_compare_blocks --manifest-path rust/plot-viewer/Cargo.toml`; `cargo test render_panels_locks_visible_summary_compare_copy_and_shape --manifest-path rust/plot-viewer/Cargo.toml`; `cargo check --manifest-path rust/plot-viewer/Cargo.toml` | none | Extend panel structure only if later plot UX still needs richer side-panel content | That render-boundary payload landed through Lane 2 and Lane 4 consumed it in 6bb1fba, so the older adopted insight is now resolved. |
| plot-mode | Lane 5 | Plot viewer docs + operator flow | Recovery-baseline README and local run instructions | parked | `25ea3c1` | `npm run build`; `node --test tests/plot-readme.test.js`; `node --test tests/plot-mode-shell.test.js` | none | Investigate whether shell/readme regression alignment can be solved inside docs/test ownership before widening scope | README and shell/readme regression coverage already match the tracked recovery-baseline workflow within docs/test ownership. |

## Recent Codex Threads

> Auto-refreshed from `~/.codex/state_5.sqlite` for this repo cwd.

| Nickname | Role | Thread ID | Updated |
| --- | --- | --- | --- |
| Rawls | worker | `019d0e91-8031-7a00-ac29-6db37ccac556` | 2026-03-21 14:23:13Z |
| Ohm | worker | `019d0e91-8179-7190-9ab8-2abf8126ec42` | 2026-03-21 14:22:58Z |
| Bernoulli | worker | `019d0e91-7ef5-79e3-9651-1231a78b2fd5` | 2026-03-21 14:22:14Z |
| Lovelace | worker | `019d0e91-7dd4-7ca0-8435-696ee9922e3c` | 2026-03-21 14:22:00Z |
| Franklin | explorer | `019d0e59-6ff2-70c0-91e8-42802f123ecc` | 2026-03-21 03:59:21Z |
| Hegel | explorer | `019d0e59-7103-7102-99fb-ab87ad0d793e` | 2026-03-21 03:59:08Z |
| Lorentz | explorer | `019d0e59-6ef1-7c11-9fd4-3ecd821ee0c0` | 2026-03-21 03:59:04Z |
| Copernicus | explorer | `019d0e59-6df9-7fb1-8a30-7af54b3df9b5` | 2026-03-21 03:59:02Z |
