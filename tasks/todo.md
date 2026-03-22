# 2026-03-22 NLSDD worker telemetry 與並行下降診斷設計

- [x] 盤點現有 NLSDD event / lane state / insight surfaces，確認可直接投影每分鐘 worker 指標的基礎資料
- [x] 將雙軌 worker 指標、總運作時間、command-blocked 沉默 worker 納入正式設計
- [x] 設計 execution 結束後的自動回顧輸出，包含每段並行下降原因與資訊不足時的補資訊建議
- [x] 將實驗結果收斂成 implementation plan，寫入 `plan/2026-03-22-nlsdd-worker-telemetry-implementation-plan.md`
- [x] 依 plan 進入 TDD 實作與驗證

## Review

- 設計已收斂成 event-first telemetry：主資料源是 `events.ndjson`，不是 coordinator 側的全域 `ps` / `pstree`。
- 實驗確認了三種不同情境都要分流：正常完成、快速失敗、以及可能的 blocked/waiting；不能把所有「無回覆」都當成 prompt gate。
- 已落地 command lifecycle telemetry event、worker-local record helper、`telemetry-summary.json` 聚合，以及 `telemetry-review.md` renderer。
- coordinator loop 現在會在 telemetry summary/review 存在時把 minute bucket 與 drop segment 摘要帶出來，`NLSDD/AGENTS.md` 也補上 worker 何時記 command events / probes 的規則。
- 驗證已通過：
  - `node --test tests/nlsdd-automation.test.js`
  - `npm run nlsdd:scoreboard:refresh`
  - `npm run build`

# 2026-03-22 `nlsdd-go` runtime truth sync

- [x] 依既有 `nlsdd-self-hosting` execution 重新跑 dispatch-plan / coordinator / insight summary dry-run
- [x] 確認目前沒有誠實可派工 lane、review action 或 commit intake
- [x] 將已完成的 telemetry / meta plans checkbox 同步回 runtime truth，避免下次 `nlsdd-go` 被過期 checklist 誤導

## Review

- `nlsdd-self-hosting` 目前的 coordinator truth 是 `idleSlots=4`、`promotedLanes=[]`、`reviewActions=[]`、`commitIntake=[]`、`actionableCount=0`，所以這次 `nlsdd-go` 的正確結果是 no-op，而不是硬開新 lane。
- 剩餘不一致主要來自計畫文件的 checkbox drift，而不是 runtime 還有未收斂的 worker 工作；這次已把 `plan/2026-03-22-nlsdd-worker-telemetry-implementation-plan.md` 與 `plan/2026-03-21-nlsdd-meta-optimization-plan.md` 勾回真相。
- 重新檢查後，`NLSDD/state/nlsdd-self-hosting/telemetry-summary.json` 與 `telemetry-review.md` 已可由 coordinator dry-run 正常投影與引用。
- 驗證：
  - `node NLSDD/scripts/nlsdd-build-dispatch-plan.cjs --execution nlsdd-self-hosting --dry-run --json`
  - `node NLSDD/scripts/nlsdd-run-coordinator-loop.cjs --execution nlsdd-self-hosting --dry-run --json`
  - `node NLSDD/scripts/nlsdd-summarize-insights.cjs --execution nlsdd-self-hosting --json`

# 2026-03-22 plot-mode replan

- [x] 盤點所有 `plan/`，區分 checkbox drift 與真正未完成的產品工作
- [x] 重新跑 `plot-mode` execution 的 dispatch-plan / coordinator / insight summary dry-run
- [x] 確認 `plot-mode` 產品仍未完成，但 execution 目前被收斂成 `no-dispatchable-lane`
- [x] 針對 `plot` 沒真正顯示的缺口，設計新的 replan 與 lane 邊界
- [x] 將 replan 寫入 `plan/2026-03-22-plot-mode-reactivation-plan.md`

## Review

- `plot-mode` 現況不是功能完成，而是 tracked lanes 把剩餘缺口都停在 parked：Node handoff 已存在、Rust binary 也能 build，但 Rust viewer 目前仍以 scaffold/placeholder 文字為主，沒有真正的 7d/5h plot。
- 直接重跑既有 `nlsdd-go` 不會產生 honest work，因為 coordinator dry-run 的 truth 是 `idleSlots=4`、`promotedLanes=[]`、`reviewActions=[]`、`commitIntake=[]`、`noDispatchReason="no-dispatchable-lane"`。
- 這次 replan 的核心是把剩餘真工作重新切成可 dispatch 的 lane：
  - Lane 2：runtime interaction / shared render state
  - Lane 3：real chart rendering
  - Lane 4：summary/compare panel refresh
  - Lane 5：只在 visible behavior 真的改變後再補 docs
- 下一個 honest NLSDD 動作不是再按一次舊 execution，而是先把 `plot-mode` tracking surfaces 改成這個新 active set，之後再 `nlsdd-go`。
- 驗證：
  - `node NLSDD/scripts/nlsdd-build-dispatch-plan.cjs --execution plot-mode --dry-run --json`
  - `node NLSDD/scripts/nlsdd-run-coordinator-loop.cjs --execution plot-mode --dry-run --json`
  - `node NLSDD/scripts/nlsdd-summarize-insights.cjs --execution plot-mode --json`

# 2026-03-22 plot-mode Lane 2 runtime contract

- [x] 為 Rust viewer runtime 新增 selected/current/focus 的結構化 render-state contract
- [x] 以 Rust 單元測試鎖住 profile cycling 與 focus cycling 後的共享 state truth
- [x] 讓 chart / panels 改吃同一份 shared `selection_state()`，不再各自重組 runtime state
- [x] 清掉舊的 label helper / fallback API 漂移，讓 Lane 2 只剩單一 render-state truth

## Review

- 根因不是 left/right 或 tab/shift-tab 本身失效，而是 runtime 雖然有 `selected_profile_index` 與 `focus`，render boundary 卻只暴露零散 label，導致 chart / panels 沒有正式的 shared state contract 可依賴。
- 這次在 Rust viewer 內新增 `SelectionState`、`RenderProfile`、`FocusTarget`，讓 `AppRenderState` 直接提供 `selection_state()`；chart 與 panels 也改成從這個 contract 讀 `selected/current/focus`，不再各自偷讀不同來源。
- 兩個 Rust 測試現在直接鎖住這個行為：初始 selected/current/focus 必須一致，且在 profile cycling + focus cycling 後，selected/current/focus 仍要一起正確更新。
- 驗證：
  - `cargo test --manifest-path rust/plot-viewer/Cargo.toml`
  - `cargo check --manifest-path rust/plot-viewer/Cargo.toml`
  - `node --test tests/plot-viewer-scaffold.test.js`

# 2026-03-22 plot-mode Lane 3 real chart rendering

- [x] 將 shared render-state boundary 擴成 chart 可直接消費的 7d points / 5h band payload
- [x] 把 chart scaffold placeholder 換成真正的 ratatui 7d line chart 與 axis labels
- [x] 在 5h band 可用時畫出 overlay，無資料時顯示 truthful fallback reason
- [x] 以 buffer-based Rust regression tests 鎖住可見 chart 與 band 文案

## Review

- 根因不是 Rust viewer 沒有 chart area，而是它一直停在 placeholder paragraph，render boundary 也沒有正式提供 chart payload，所以不可能靠 later polishing 自然長出真正的圖。
- 這次在 `render/mod.rs` 補上 `ChartState` / `ChartPoint` / `FiveHourBandState`，由 runtime 把 selected profile 的 7d points 與 5h band 正式交給 chart renderer。
- `chart.rs` 現在會真的畫出 ratatui line chart、x/y axis labels、band overlay 與 band summary；同時用兩個 Rust tests 鎖住「有 band」與「band unavailable」兩條 visible path，避免再退回 placeholder copy。
- 驗證：
  - `cargo test --manifest-path rust/plot-viewer/Cargo.toml render::chart -- --nocapture`
  - `cargo check --manifest-path rust/plot-viewer/Cargo.toml`

# 2026-03-22 plot-mode Lane 4 panel refresh

- [x] 讓 Summary / Compare 改吃 shared selection/chart state，而不是 panel-local placeholder heuristic
- [x] 補上 focused profile、snapshot current、7d sample count、5h band status 的可見 summary
- [x] 讓 Compare 清楚區分 adopted target、current route，以及 selected==current 時的 no-op 狀態
- [x] 以 Rust regression tests 鎖住 panel copy 與結構，並補 README smoke

## Review

- 根因不是 panel 完全沒有資料，而是它一直停在 skeleton wording，沒有把 Lane 2 / Lane 3 已經存在的 runtime 與 chart truth 消化進來，所以 compare 內容始終像 placeholder。
- `render/panels.rs` 現在直接吃 shared `selection_state()` + `chart_state()`：Summary 會顯示 focused/current/focus、7d sample 數與 5h band 狀態；Compare 會清楚標出 adopted target、current route，以及當 target 已經是 current 時的 `Routing delta: none`。
- 這讓 Lane 4 不再重組 label heuristic，也讓先前 reviewer 指出的「compare panel still re-derives label heuristics」有正式修正路徑。
- 驗證：
  - `cargo test render_panels_builds_visible_summary_and_compare_blocks --manifest-path rust/plot-viewer/Cargo.toml`
  - `cargo test render_panels_locks_visible_summary_compare_copy_and_shape --manifest-path rust/plot-viewer/Cargo.toml`
  - `node --test tests/plot-readme.test.js`

# 2026-03-22 proceed 後自動 finishing step 規則

- [x] 找出為什麼 main agent 在成功 commit 後還停下來等下一次 `proceed`
- [x] 將 post-commit auto-advance 邊界寫進本地治理規則
- [x] 將這次修正收斂進 `tasks/lessons.md`

## Review

- 根因不是缺少 `proceed` 口令，而是本地規則只明寫到「治理變更驗證後直接 commit」，沒有把「若使用者已經授權進入持續收斂流程，commit 後的單一路徑 finishing step 也應直接接續」寫清楚。
- 現在 `AGENTS.md` 已補上邊界：只有當 main agent 已在使用者明確授權的 `proceed` 流程中，且 commit 後只剩單一、低風險、可逆的 finishing 動作時，才自動往下走；若還有 merge / PR / push / release 等多路徑決策，仍必須停下來對齊。
- 這次也把同一個 pattern 寫進 `tasks/lessons.md`，避免之後又回到「commit 完就先停住等下一句 `proceed`」的機械式行為。

# 2026-03-22 nlsdd-go for all plans together

- [x] 盤點 `plan/` 下所有 plans，區分真正未完成工作與 checkbox drift
- [x] 對 `nlsdd-self-hosting` 與 `plot-mode` 一起重跑 dispatch / coordinator / insights truth
- [x] 收斂 `plot-mode` 的 stale implementing lane journal，避免 execution 看起來還在跑其實已 landed on main
- [x] 將 reactivation plan 與 plot-mode tracked docs 補回 honest no-op 狀態

## Review

- `nlsdd-self-hosting` 的 truth 仍然是完全收斂：`idleSlots=4`、`promotedLanes=[]`、`reviewActions=[]`、`commitIntake=[]`、`actionableCount=0`，所以這條 execution 依舊是 honest no-op。
- `plot-mode` 的產品面其實也已經透過 `5c2d643` 落地完 Lane 2/3/4，但 execution tracking 還留著一組 stale implementing state，導致 `dispatch-plan` 看起來像還有 1 條 active lane、2 條 stale implementing lane。這不是新工作，而是 runtime journal / tracked docs 漂移。
- 這次已用 `nlsdd-replan-active-set` 把 `plot-mode` 全部 lanes 收回 parked，並同步更新 scoreboard、lane docs、overview 與 reactivation plan，讓「所有 plans together」回到一致真相：目前沒有 honest dispatchable lane。
- 驗證：
  - `node NLSDD/scripts/nlsdd-build-dispatch-plan.cjs --execution nlsdd-self-hosting --dry-run --json`
  - `node NLSDD/scripts/nlsdd-run-coordinator-loop.cjs --execution nlsdd-self-hosting --dry-run --json`
  - `node NLSDD/scripts/nlsdd-summarize-insights.cjs --execution nlsdd-self-hosting --json`
  - `node NLSDD/scripts/nlsdd-build-dispatch-plan.cjs --execution plot-mode --dry-run --json`
  - `node NLSDD/scripts/nlsdd-run-coordinator-loop.cjs --execution plot-mode --dry-run --json`
  - `node NLSDD/scripts/nlsdd-summarize-insights.cjs --execution plot-mode --json`

# 2026-03-22 NLSDD execution / dev-flow review

- [x] 盤點 NLSDD execution docs、runtime scoreboard、lane journal、coordinator helper 與 lessons
- [x] 找出目前最傷 dev flow 的 runtime hard-fail 與 tracking drift 問題
- [x] 將改善路線寫入 `plan/2026-03-22-nlsdd-dev-flow-improvement-plan.md`

## Review

- 主要 friction 不是單一 helper 壞掉，而是目前 NLSDD 還有三個高槓桿缺口：
  - coordinator loop 對 runtime scoreboard 過度脆弱，輔助 surface 缺表時會直接崩掉
  - stale implementing lane 雖然能被 schedule 偵測，卻沒有標準 reconcile path 直接收斂回 honest parked/no-op
  - tracked lane docs / overview / plan checkbox 仍常需要人工同步，和 reducer/projection-only 的治理方向不完全一致
- 這次 review 已把改善路線拆成 3 個 chunk：
  - fail-soft coordinator path
  - execution-truth sync helper
  - lane-status tracked surface sync
- 下一步應直接照 `plan/2026-03-22-nlsdd-dev-flow-improvement-plan.md` 進入 TDD，而不是再手動靠一次次 scoreboard / plan / lane-doc 回寫來維持 execution truth。

# 2026-03-22 `nlsdd-go` 語意補強

- [x] 將 `nlsdd-go` 語意從「盤點 truth」補強成「補 truth 後繼續推進」
- [x] 將這次使用者修正寫進 `AGENTS.md`、`NLSDD/AGENTS.md` 與 `tasks/lessons.md`

## Review

- 根因不是單一回覆 wording 不夠清楚，而是我把 `nlsdd-go` 錯切成「先盤點 execution truth、再停下來等待下一句 `proceed`」；這和使用者期待的固定口令語意不一致。
- 現在規則已補清楚：`nlsdd-go` 代表 main agent 要先同步 execution/runtime truth，然後直接續推 active lanes、review/correction、commit intake，或下一批 honest dispatchable lanes；只有遇到需要使用者決策的分岔時才停。

# 2026-03-22 nlsdd-go 推進 dev-flow improvement Chunk 1

- [x] 先在 automation tests 補上 malformed runtime scoreboard regression，鎖住 commit-intake / coordinator fail-soft 行為
- [x] 在 NLSDD shared helpers 補上 preferred scoreboard fallback，讓 runtime scoreboard 壞掉時能降級吃 tracked scoreboard
- [x] 讓 coordinator / cycle 路徑共用同一個 fallback，而不是只在 commit-intake 局部 try/catch
- [x] 重跑 `node --test tests/nlsdd-automation.test.js`，確認 Chunk 1 維持全綠

## Review

- 這次推進的根因不是單純 `intakeReadyToCommit()` 缺 try/catch，而是 launch/schedule 與 commit-intake 都各自直接吃 preferred scoreboard；只修 intake 會留下 coordinator 前半段和後半段用不同 truth 的風險。
- 現在 `NLSDD/scripts/nlsdd-lib.cjs` 已新增 shared preferred-scoreboard fallback：優先讀 runtime scoreboard，若 runtime artifact 缺表或 malformed，就降級讀 tracked scoreboard。
- `NLSDD/scripts/nlsdd-intake-ready-to-commit.cjs` 與 `NLSDD/scripts/nlsdd-run-cycle.cjs` / `NLSDD/scripts/nlsdd-run-coordinator-loop.cjs` 都已吃同一套 fallback，讓 malformed runtime scoreboard 不再直接把 coordinator flow 炸掉。
- 驗證：
  - `node --test tests/nlsdd-automation.test.js`

# 2026-03-20 limits 欄位 header 修正

# 2026-03-21 plot-mode integration branch 收斂

- [x] 找出 accepted Lane 1 / 3 / 4 commits 的 shared plot baseline
- [x] 改用 whole-lane merge，而不是在太新的 `main` 文書線上做單顆 cherry-pick
- [x] 建立 `.worktrees/plot-integration-base`，從 `d19d319` 整合 Lane 1 / 3 / 4 的 accepted stacks
- [x] 驗證整合分支上的 Node handoff / snapshot / README 測試與 Rust chart/panels 驗證
- [x] 將 Lane 3 / Lane 4 的最新 accepted commits 與 integration branch 狀態回寫到 NLSDD tracking

## Review

- 前一條 `plot-integration` 失敗，不是因為 lane commits 本身壞掉，而是把它們硬套到過新的 `main` 文書線，造成單顆 cherry-pick 和 lane stack 依賴脫鉤。
- 這次改從 shared baseline `d19d319` 建立 `.worktrees/plot-integration-base`，並直接 merge whole-lane branches，Lane 1 / 3 / 4 就能保留 provenance 並乾淨整合。
- Lane 3 也趁這輪多完成了一個小切片 `35c8351`，把 focus 狀態更明確地帶進 chart header；Lane 4 則完成 `b24f12a`，將 Summary / Compare 的欄位建構收斂成 helper，但維持可見輸出不變。
- 驗證：
  - in `.worktrees/plot-integration-base`
    - `npm run build`
    - `npm run plot:viewer:build`
    - `node --test tests/plot-handoff.test.js tests/plot-readme.test.js tests/plot-snapshot.test.js`
    - `cargo test --manifest-path rust/plot-viewer/Cargo.toml render::chart -- --nocapture`
    - `cargo test render_panels_builds_visible_summary_and_compare_blocks --manifest-path rust/plot-viewer/Cargo.toml`
    - `cargo test render_panels_locks_visible_summary_compare_copy_and_shape --manifest-path rust/plot-viewer/Cargo.toml`
    - `cargo check --manifest-path rust/plot-viewer/Cargo.toml`

# 2026-03-21 NLSDD commit gate 誤判修正

- [x] 找出 subagent 在 lane 尾端看似沒反應的根因，確認是 commit permission prompt 而不是純 stalled thread
- [x] 將 commit gate 場景寫入 `spec/NLSDD/guardrails.md` 與 `spec/NLSDD/communication.md`
- [x] 將 `READY_TO_COMMIT` 納入 NLSDD 的正式狀態語彙與 handoff 要求
- [x] 在 `NLSDD/AGENTS.md` 與 `tasks/lessons.md` 補上對應守則，避免 coordinator 再誤判 agent 無回應

# 2026-03-21 NLSDD commit 情境分流

- [x] 找出 commit 規則在 main agent 與 NLSDD subagent 之間的情境衝突
- [x] 將 `spec/NLSDD`、`NLSDD/AGENTS.md`、repo `AGENTS.md` 改成情境分流規則
- [x] 更新 message helper，明確要求 subagent 預設不要自己跑 `git commit`
- [x] 補測試鎖住新的 implementer assignment 文案

## Review

- 根因不是單一 subagent 忘了規則，而是 repo 內同時存在兩條都合理但適用情境不同的 commit 規則：main agent 的「規則變更驗證後直接 commit」與 NLSDD 的「lane-local MVC 完成就要收斂成 commit」。
- 若不把情境切開，新的 worker 很容易把 main agent 的自動 commit 規則誤套到 lane worktree，結果卡在 permission prompt，然後 coordinator 又以為 thread 沒反應。
- 這次修正後的邊界是：
  - main agent 直接做本地工作：驗證後直接 commit
  - NLSDD subagent：預設不自己 `git commit`，而是交 `READY_TO_COMMIT`
  - 只有 lane 明確標示 self-commit-safe 才能例外

# 2026-03-21 NLSDD execution insights journal

- [x] 新增 execution-level insights journal，記錄 subagent 建議、coordinator/main agent 觀察到的問題，以及執行期間發現的改善機會
- [x] 新增 `nlsdd-record-insight` helper，將 insight 以 append-only runtime artifact 寫入 `NLSDD/state/<execution>/execution-insights.ndjson`
- [x] 更新 `spec/NLSDD` 與 `NLSDD/AGENTS.md`，明確區分 lane state 與 execution insights 的責任
- [x] 補測試，確認 subagent 與 coordinator 都能正確 append insight entries

## Review

- 目前 lane journal 只適合放「現在這條 lane 是什麼狀態」，不適合承接動態建議、觀察到的流程問題、或執行中才浮現的改善方向。
- 這次補上的 `execution-insights.ndjson` 讓兩種資訊都能留下來：
  - subagent 的 remediation suggestion
  - main agent/coordinator 在執行期觀察到的流程或治理問題
- 它是 append-only runtime artifact，不會和 tracked docs 的穩定定義混在一起，也不會讓 lane state JSON 膨脹成半個事件流。

# 2026-03-21 NLSDD execution insights review integration

- [x] 將 execution-insights schema 與實際 runtime 用法對齊，補上 blocker / noop 類型
- [x] 新增 `nlsdd-summarize-insights` helper，整理 open / adopted insights 給 coordinator 快速檢視
- [x] 讓 `nlsdd:review` / `nlsdd:autopilot` / `nlsdd:dispatch-plan` surface execution-insights 摘要
- [x] 新增規則：NLSDD flow 遇到 `review` prompt 時，要同步檢視並規劃處理 execution-insights journal

## Review

- 根因不是 journal 沒有資料，而是它只有 append path，幾乎沒有 read/use path，導致很多 runtime learnings 雖然被記下來，卻沒有真的進入 coordinator 的日常判斷面。
- 這次先把 schema 和真實使用收斂：`kind` 正式納入 `noop-finding`、`blocker`、`resolved-blocker`，避免 spec 和 runtime state 繼續漂移。
- 接著新增 `nlsdd-summarize-insights`，讓 coordinator 可以快速看某個 execution 的 open / adopted insights，而不是直接掃 `ndjson`。
- 最後把 insight summary 接進 `review`、`autopilot`、`dispatch-plan`，讓 execution insights 不再只是埋在 runtime artifact 裡，而會成為 review 時自然可見的一部分。

# 2026-03-21 plot-mode 4a 執行追蹤同步

- [x] 將 Lane 2 runtime navigation regression `1fd4db4` 的接受結果同步回 tracked execution docs
- [x] 將 Lane 5 docs/operator-flow commits `888e2d9`、`25ea3c1` 的接受結果同步回 tracked execution docs
- [x] 將 Lane 4 的真 blocker 收斂成「需要 Lane 2 提供 render-boundary compare payload」，並回寫 scoreboard / lane docs
- [x] 將這輪 4a execution 的動態 learning append 到 execution insights journal

## Review

- 這輪最大的 planning 收斂不是某一顆 commit，而是把「誰真的還在 active、誰其實只是 stale implementing state、誰有真 blocker」從 thread 記憶拉回到 tracked execution surface。
- Lane 4 的 correction 已經證明：光把 model seam cherry-pick 進 panel lane 還不夠，真正缺的是 `render/mod.rs` 的 compare payload handoff；因此現在應由 Lane 2 先補 boundary seam，而不是讓 Lane 4 持續假性 implementing。
- Lane 5 也已經不是「新 lane 尚未開始」，而是有三個連續 docs-only MVC steps 落地；下一步該查的是 shell/readme regression alignment 是否仍在 docs ownership 內，而不是再重複描述 recovery baseline。

## Review

- 根因不是 subagent 卡死，而是 lane item 已經完成到只剩 `git commit`，但執行環境會跳 permission prompt；若這時 subagent 沒先回報，coordinator 只看 thread 會像是「人突然不動了」。
- 修正方向不是放寬 probe，而是把這種狀態正式命名成 `READY_TO_COMMIT`，並要求 implementer 在 commit 前主動回報 commit scope、驗證結果與預期 gate。
- 這樣 coordinator 之後在 probe lane 時，就會把「可能在等 commit gate」當成一級判斷，而不是直接把 lane 標成 stalled / unresponsive。

# 2026-03-21 plot-mode 4a NLSDD execution round

- [x] 以 4 個 active subagents 啟動 plot-mode 的下一輪 lane-local refill / review
- [x] 完成 Lane 1 的 snapshot semantics tightening，並通過 spec + quality review
- [x] 完成 Lane 3 的 chart surface 5h band / axis labels / fallback，並通過 spec + quality review
- [x] 完成 Lane 4 的 panel-specific regression coverage，並通過 spec + quality review
- [x] 以 lane-local evidence 關閉 Lane 2 的 correction loop，確認目前不需要更強的 nested `usage` decode
- [x] 更新 NLSDD manual scoreboard、plot-mode lane docs 與主 plan，反映這輪 execution 結果

## Review

- 這輪 4a NLSDD 的實作面已收斂：Lane 1 (`baa7b8e`)、Lane 3 (`585317d`)、Lane 4 (`abd8b10`) 都完成 lane-local commit，並先後通過 spec + quality review。
- Lane 2 沒有產出新 code；lane-local probe、cargo 驗證與 reviewer 結論一致指出：目前 chart/panels lane 還沒有證明 nested `usage` decode 是真 blocker，因此這條 lane 應改為 `parked`，而不是為了維持 4 active 而硬找工作。
- 這也讓 plot-mode execution docs 與 scoreboard 回到同一個真相來源：Lane 1/3/4 是已接受的第三輪 refill 成果，Lane 2 是條件式 parked lane，之後只在 runtime blocker 真正出現時再喚醒。
- 驗證：
  - `npm run build` in `.worktrees/lane-1-node`
  - `node --test tests/plot-snapshot.test.js tests/plot-handoff.test.js` in `.worktrees/lane-1-node`
  - `cargo test --manifest-path rust/plot-viewer/Cargo.toml render::chart -- --nocapture` in `.worktrees/lane-3-chart`
  - `cargo check --manifest-path rust/plot-viewer/Cargo.toml` in `.worktrees/lane-3-chart`
  - Lane 4 agent verification:
    - `cargo test render_panels_builds_visible_summary_and_compare_blocks --manifest-path rust/plot-viewer/Cargo.toml`
    - `cargo test render_panels_locks_visible_summary_compare_copy_and_shape --manifest-path rust/plot-viewer/Cargo.toml`

# 2026-03-21 plot-mode 4-active-lane re-plan

- [x] 盤點 plot-mode lane docs、manual scoreboard、runtime scoreboard 與 lane worktrees 的目前狀態
- [x] 用 4 條 lane 平行分析下一個 reviewable item 與 active/queued/parked 建議
- [x] 將 plot-mode execution docs、manual scoreboard 與上層 plans 收斂到新的 4-active-lane 規劃
- [x] 驗證 NLSDD schedule / refill tooling 在新規劃語意下仍可正常讀取

## Review

- 目前 plot-mode 的 manual scoreboard 與 lane docs 都還殘留 `spec-review-pending` 的第二輪文案，但 runtime scoreboard 顯示四條 lane 實際都還停在 `correction` 相關狀態；這次先把 manual planning surface 校正成能反映現在的協調真相。
- 4 條 lane 的平行分析收斂出一致結論：Lane 1、Lane 3、Lane 4 都還有清楚的 lane-local refill item，應維持為下一輪 active/refill priority；Lane 2 的下一步則是條件式 model deepening，應在當前 correction 關閉後改為 `parked`，除非 chart/panels lane 證明它已成真 blocker。
- 因此目前不新增 Lane 5；先耗盡既有 4-lane pool 比較乾淨，也更符合 lane-local refill 原則。
- 驗證：
  - `node NLSDD/scripts/nlsdd-suggest-schedule.cjs --execution plot-mode --json`
  - `node NLSDD/scripts/nlsdd-suggest-refill.cjs --execution plot-mode --json`

# 2026-03-21 NLSDD runtime scoreboard 邊界收斂

- [x] 將 auto-refreshed scoreboard 輸出改到 ignored runtime 檔，不再覆寫 tracked `NLSDD/scoreboard.md`
- [x] 將 `NLSDD/state/` 納入 ignore 邊界，讓 lane journal 與 runtime scoreboard 都視為執行期狀態
- [x] 更新 NLSDD 定義與執行文件，明確區分 tracked scoreboard 與 runtime scoreboard
- [x] 補回歸測試，確認 refresh 會保留 tracked scoreboard 並改寫 runtime scoreboard
- [x] 讓 schedule / refill helper 優先讀取 runtime scoreboard，缺席時才回退到 tracked scoreboard
- [x] 將 tracked scoreboard 收斂成 manual-only 表格，讓 runtime scoreboard 承接完整 auto 欄位

## Review

- 根因：`npm run nlsdd:scoreboard:refresh` 原本直接覆寫 tracked 的 [`NLSDD/scoreboard.md`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scoreboard.md)，導致每次 refresh 都把主工作樹弄髒，也讓 merge / cherry-pick 容易被 runtime 狀態阻塞。
- 修正：新增 ignored runtime scoreboard 邊界，refresh 現在改寫 `NLSDD/state/scoreboard.runtime.md`；tracked 的 `NLSDD/scoreboard.md` 只保留 coordinator 維護的 canonical row set 與說明。
- 一併把 [`NLSDD/state/`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/state/) 納入 [`.gitignore`](/home/jethro/repo/side-projects/codex-account-switcher/.gitignore)，讓 lane journal 與 runtime scoreboard 同屬執行期狀態，不再污染 tracked tree。
- 補上 `schedule` / `refill` 讀取鏈路，現在會優先使用 runtime scoreboard；只有 runtime 檔不存在時，才回退到 tracked scoreboard，避免 source of truth 再次分裂。
- 再進一步將 tracked scoreboard 表格收斂為 manual-only 欄位；runtime refresh 會從這份人工欄位表擴張出完整的 derived table，讓 tracked 與 runtime 的責任分界更乾淨。
- 驗證：
  - `node --test tests/nlsdd-automation.test.js`
  - `npm run nlsdd:scoreboard:refresh`
  - `npm run build`
  - `node NLSDD/scripts/nlsdd-suggest-schedule.cjs --execution plot-mode`
  - `node NLSDD/scripts/nlsdd-suggest-refill.cjs --execution plot-mode --lane 2`

# 2026-03-21 NLSDD meta-optimization lane

- [x] 新增一條 `nlsdd-self-hosting` lane，專門 review 並優化 `NLSDD` 本身
- [x] 併行檢視 `spec/NLSDD/`、`NLSDD/`、scripts、scoreboard 與 execution flow
- [x] 結合 `plan/2026-03-20-multi-phase-routing-plan.md` 與 `plan/2026-03-20-ratatui-plot-mode-implementation-plan.md` 的後續工作一起評估
- [x] 觀察 subagents 的實際執行動態，不只看靜態文件
- [x] 選出單一最高槓桿改善點，與 main agent 對齊後落地實作
- [ ] 驗證、提交 branch，最後 merge 回 `main`

## Review

- 靜態 review 與動態觀察收斂出同一個核心問題：NLSDD 缺少 execution-aware 的 lane runtime state source of truth，導致 phase drift、cross-execution lane number bleed，以及從 linked worktree 執行時的 root/path 解析錯位。
- 已補上 `NLSDD/state/<execution>/lane-<n>.json` journal 機制，並讓 `nlsdd-lib`、`scoreboard refresh`、`schedule suggest` 優先吃 journal，再回退到 thread/session heuristics。
- 同步修正 canonical project-root 解析，讓從 linked worktree 執行 `probe` / `schedule` / `refresh` 時仍會回到同一個 repo root 找 lane plans、state 與 scoreboard。
- 新增 `NLSDD/scripts/nlsdd-record-lane-state.cjs`，讓 coordinator 可用正式 helper 寫入 lane journal，而不是手改 JSON。
- 驗證：
  - `node --test tests/nlsdd-automation.test.js`
  - `node NLSDD/scripts/nlsdd-suggest-schedule.cjs --execution nlsdd-self-hosting`
  - `npm run nlsdd:scoreboard:refresh`
  - `npm run build`

# 2026-03-21 plot-mode next 4a NLSDD re-plan

- [x] 重新盤點 `main` 上的 plot-mode lane docs、manual scoreboard、runtime schedule/refill 與 lane worktrees
- [x] 確認 `main` 尚未正式承接 recovery branch 的 Lane 5 docs/operator-flow family
- [x] 將主線 plot-mode execution 收斂成下一個 4a active set：Lane 2 + Lane 3 + Lane 4 + Lane 5
- [x] 將 Lane 1 改為 parked，避免沒有新 reviewable item 時只靠慣性佔住 active slot
- [x] 在主線新增 Lane 5 plan，讓 docs/operator flow 成為正式 lane family
- [x] 記錄 lane journal 與 manual plan 目前存在漂移，要求下輪 dispatch 前先 refresh / rewrite journal

## Review

- 這次不是直接沿用 recovery branch 的舊 dispatch，而是重新比對了 `main` 上的 manual scoreboard、lane docs、runtime `schedule/refill` 輸出與 lane worktree probe。結果很明確：主線目前還沒有 Lane 5，但 Lane 1 也已經沒有新的 reviewable item，不適合再把它硬塞進 4a active set。
- 因此新的 4a 規劃改成 `Lane 2 + Lane 3 + Lane 4 + Lane 5`。Lane 2 回到 runtime navigation/focus-flow family，Lane 3 負責 chart 對 richer focus/profile cycling 的相容性，Lane 4 負責 Compare panel 的 recommendation-rich content，Lane 5 則正式承接 README/operator flow/run instructions。
- 另外也確認了一個執行面風險：目前 `NLSDD/state/plot-mode/lane-2..4.json` 還留著 recovery-branch dispatch 狀態，所以 runtime tooling 會顯示 `implementing`，但那不等於主線新的 manual 4a 計畫。這次先把風險寫進 overview 與 todo，要求下輪真正 dispatch 前先 refresh 或 rewrite lane journal。
- 補充：原本替 Lane 5 寫的 `tests/plot-readme.test.js tests/plot-mode-shell.test.js` 驗證命令在目前主線 worktree 還不是穩定可跑，因此先把 lane-local required verification 收斂成 `npm run build`，等 docs 測試檔正式納入這條 workflow 再升回 required verification。
- 已新增 `nlsdd-replan-active-set` helper，讓下次重排 `active/parked` lane set 時可以原子同步 tracked `Phase` 與 lane journal，不再只靠人工記得兩邊都要改。
- 驗證：
  - `node NLSDD/scripts/nlsdd-suggest-schedule.cjs --execution plot-mode --json`
  - `node NLSDD/scripts/nlsdd-suggest-refill.cjs --execution plot-mode --json`
  - `node NLSDD/scripts/nlsdd-probe-lane.cjs --execution plot-mode --lane 1`
  - `node NLSDD/scripts/nlsdd-probe-lane.cjs --execution plot-mode --lane 2`
  - `node NLSDD/scripts/nlsdd-probe-lane.cjs --execution plot-mode --lane 3`
  - `node NLSDD/scripts/nlsdd-probe-lane.cjs --execution plot-mode --lane 4`

# 2026-03-21 NLSDD active-set atomic replan helper

- [x] 先補 failing test，鎖住 active-set replan 會同時更新 tracked scoreboard phase 與 lane journal
- [x] 新增 `NLSDD/scripts/nlsdd-replan-active-set.cjs`
- [x] 補 `package.json` script，讓 coordinator 可用固定命令重排 active set
- [x] 將 helper 納入 `spec/NLSDD` 與 `NLSDD/AGENTS.md`
- [x] 用 helper 將 `plot-mode` 的 `Lane 1 parked / Lane 2-5 queued` 同步到 journal，收掉目前的 planning drift

## Review

- 根因是 manual scoreboard / lane docs 與 lane journal 是分兩步更新，導致 coordinator 一旦先改 tracked 文件，`schedule/refill` 仍會被舊 journal 拉回上一輪 dispatch truth。
- 這次新增 `nlsdd-replan-active-set`，把 replan 收斂成一個原子操作：它會改 tracked scoreboard 的 `Phase`，同步重寫指定 lanes 的 journal phase / nextExpectedPhase，最後再 refresh runtime scoreboard。
- helper 目前專注解決最痛的 drift 邊界：`active` lanes 會被寫成 `queued -> implementing` 的下一步，`parked` lanes 會被寫成 `parked`。lane plan 文字內容仍維持手動，因為那屬於規劃敘述，不適合自動改寫。
- 也已把 `plot-mode` 目前的 4a 計畫實際套用一次，讓 `Lane 1` journal 不再維持 `refill-ready`，`Lane 2-5` 則與 manual scoreboard 對齊成 queued/parked truth。
- 驗證：
  - `node --test tests/nlsdd-automation.test.js`
  - `node NLSDD/scripts/nlsdd-replan-active-set.cjs --execution plot-mode --active 2,3,4,5 --parked 1 --note "manual 4a replan from tracked scoreboard"`
  - `node NLSDD/scripts/nlsdd-suggest-schedule.cjs --execution plot-mode --json`

- [x] 確認 `Usage Left`、`Time to reset`、`Drift` 目前被渲染成每列 label 的根因
- [x] 先補回歸測試，鎖住三個欄位名只出現在 header
- [x] 抽出並接回表格排版 helper，讓 row 只顯示值不重複印欄位名
- [x] 執行建置與測試驗證，補上 review 記錄

## Review

- 根因：[`src/commands/root.ts`](/home/jethro/repo/side-projects/codex-account-switcher/src/commands/root.ts) 原本把 `Time to reset`、`Usage Left`、`Drift` 直接寫在每列 `W:` / `5H:` 詳細行裡，導致欄位名變成 row 內容而不是 header。
- 修正：新增 [`src/lib/root-table-layout.ts`](/home/jethro/repo/side-projects/codex-account-switcher/src/lib/root-table-layout.ts) 集中處理 header 與 window detail line，讓 header 擁有欄位名，row 只渲染實際值。
- 接線：[`src/commands/root.ts`](/home/jethro/repo/side-projects/codex-account-switcher/src/commands/root.ts) 改為呼叫 layout helper，而不是手寫帶 label 的詳細列字串。
- 回歸測試：新增 [`tests/root-table-layout.test.js`](/home/jethro/repo/side-projects/codex-account-switcher/tests/root-table-layout.test.js) 驗證三個欄位名只出現在 header，不出現在 window row。
- 驗證：
  `npm run build`
  `node --test tests/root-table-layout.test.js`
  `node --test tests/entrypoints.test.js`
  `HOME="$(mktemp -d)" ... node dist/index.js`

# 2026-03-20 workload-aware routing MVP

- [x] 先補 workload tier 測試，鎖住 help 區顯示與分數權重變化
- [x] 在 routing UI 加入 workload tier 循環切換
- [x] 讓 recommendation score 依 `Auto` / `Low` / `Medium` / `High` 調整權重
- [x] 更新 spec 與 plan 狀態，記錄這個 MVP 已完成

## Review

- 新增 `W` workload tier 切換，help 與 status line 會顯示目前 tier。
- `Auto` 保持既有預設權重；`Low` 更保守、`High` 更積極，`Medium` 作為中間值。
- 驗證：
  `npm run build`
  `node --test tests/workload-tier.test.js`
  `node --test tests/entrypoints.test.js`

# 2026-03-20 prompt panel layout MVP

- [x] 先補 panel / option layout 測試，鎖住上方詳情面板與下方極簡選項
- [x] 把 all-profile 詳情改成 prompt-level panel 輸出
- [x] 把 selectable options 收斂成 indicator、profile name、delta
- [x] 補上已實作且已驗證的 prompt panel layout spec

## Review

- prompt 上方現在會輸出全部 profiles 的 detail panel。
- 每組第一行只放 profile 與 last，後續行放對齊過的 weekly / 5h rows。
- selectable options 不再承載完整表格，而是只保留 indicator、profile name、delta。
- 驗證：
  `npm run build`
  `node --test tests/root-panel-layout.test.js`
  `node --test tests/root-option-layout.test.js`
  `node --test tests/root-table-layout.test.js`
  `node --test tests/workload-tier.test.js`
  `node --test tests/entrypoints.test.js`

# 2026-03-20 prompt panel 文案與資訊節奏微調

- [x] 先補 panel 測試，鎖住 `last update: ... ago`、縮排 detail rows、emoji 分隔與隱藏缺席 limit row
- [x] 將 prompt panel 首行改成 `profile + last update`，移除 profile recommendation 顏色
- [x] 將 weekly / 5H rows 改成 `📊 ... | 🔄 in ... (...) | Pacing ...`，並補上 reset 時間剩餘百分比
- [x] 移除語意不清楚的 `Bar ... Workload ...` 狀態列，保留下方極簡選項
- [x] 執行建置與測試驗證，並更新已落地 spec

## Review

- prompt panel 第一行現在固定為 `indicator + profile + last update: ... ago`，不再混入 recommendation profile 色塊。
- weekly / 5H detail rows 改為縮排式直排資訊，格式使用 `📊`、`|`、`🔄 in` 與 `Pacing`，便於手機遠端掃讀。
- `🔄 in (...)` 的括號百分比現在表示該 reset window 的剩餘時間百分比。
- 缺席的 limit window 會直接隱藏，不再印出 `N/A` row。
- `Pacing` 在 ANSI 開啟時會保留背景色強弱，並在 panel layout 以 ANSI-safe 長度對齊。
- 原本的 `Bar Quota  Workload Auto` 狀態列已移除，避免不明語意佔用版面。
- 驗證：
  `npm run build`
  `node --test tests/root-panel-layout.test.js`
  `node --test tests/root-option-layout.test.js`
  `node --test tests/root-table-layout.test.js`
  `node --test tests/workload-tier.test.js`
  `node --test tests/entrypoints.test.js`

# 2026-03-20 prompt panel mode 修正

- [x] 先補回歸測試，鎖住 `[Q]uit`、`Quota` mode bar panel、以及只有 bottleneck row 的 `Pacing` 上色
- [x] 補回 `[Q]uit` action 與 help button
- [x] 讓 prompt panel 僅在 `Delta` mode 使用精簡資訊排版，`Quota` mode 回到 bar rows
- [x] 將 `Pacing` 顏色限制在實際採用的 bottleneck row
- [x] 執行完整建置與測試驗證，並更新落地記錄

## Review

- `Q` 現在重新出現在 help buttons，也能作為明確退出動作使用。
- `Delta` mode 保留目前的 `📊 | 🔄 in | Pacing` 精簡 panel。
- `Quota` mode 不再被精簡版覆蓋，prompt panel 會回到含 bar 的 rows，保留 quota 視覺資訊。
- `Pacing` 背景色只會出現在被採用的 bottleneck row，其他 row 保持純文字，避免整塊視覺噪音。
- 驗證：
  `npm run build`
  `node --test tests/root-panel-layout.test.js`
  `node --test tests/root-option-layout.test.js`
  `node --test tests/root-table-layout.test.js`
  `node --test tests/workload-tier.test.js`
  `node --test tests/entrypoints.test.js`

# 2026-03-20 prompt panel 對齊細節微調

- [x] 先補 panel 測試，鎖住移除 `|`、純空白對齊、數字靠右與 dimmed `last update`
- [x] 將 delta panel detail rows 改成純空白欄距，不再輸出 `|` 分隔
- [x] 將 `📊`、`🔄 in`、`Pacing` 內的數值格式統一成右對齊掃讀節奏
- [x] 將 `last update: ... ago` 改成淡化尾註
- [x] 執行完整建置與測試驗證，更新落地 spec

## Review

- delta panel 現在不再使用 `|`，而是靠欄位寬度與至少兩格空白做對齊。
- `📊  94% left`、`🔄 in 6.7d  (95%)`、`Pacing  +1.3%` 這類數值都會先整理成固定掃讀寬度，再交給 panel 做整欄對齊。
- `last update: ... ago` 在 ANSI 開啟時會用 dim 樣式顯示，讓 profile 名稱仍是主要視覺焦點。
- 驗證：
  `npm run build`
  `node --test tests/root-panel-layout.test.js`
  `node --test tests/root-option-layout.test.js`
  `node --test tests/root-table-layout.test.js`
  `node --test tests/workload-tier.test.js`
  `node --test tests/entrypoints.test.js`

# 2026-03-20 delta pacing 文案回補

- [x] 先補 panel 測試，鎖住 `Pacing [+76.6% Overuse]` 樣式、括號內上色與固定寬度色塊
- [x] 將 delta panel 的 pacing 文案補回 `% + Overuse/Under` 描述
- [x] 將顏色從整段 `Pacing` 移到括號內 payload，並維持 bottleneck-only 上色
- [x] 讓括號內 payload 以固定可視寬度補齊，保持色塊等寬
- [x] 執行建置與測試驗證，更新落地 spec

## Review

- delta panel 現在會顯示 `Pacing [+x.x% Overuse]` / `Pacing [-x.x% Under]`，不再只剩數字。
- 顏色只套在 `[...]` 內的 payload，`Pacing` 標籤本身保持中性。
- payload 會先補齊到固定可視寬度再上色，因此高亮區塊在不同 row 之間會維持同寬節奏。
- 驗證：
  `npm run build`
  `node --test tests/root-panel-layout.test.js`
  `node --test tests/root-option-layout.test.js`
  `node --test tests/root-table-layout.test.js`
  `node --test tests/workload-tier.test.js`
  `node --test tests/entrypoints.test.js`

# 2026-03-20 數值拆欄對齊規則落地

- [x] 將「所有數值拆欄並右對齊格式化」提升為 spec core rule
- [x] 補 Quota mode 測試，鎖住 drift 數值欄與描述欄分開對齊
- [x] 將共享 table layout 的 drift 從單字串改成數值欄 + 描述欄
- [x] 讓 Quota mode 也遵守數值拆欄規則，不再把 `+x.x% Overuse` 當成單一不可控字串
- [x] 執行建置與測試驗證，更新落地 spec

## Review

- `spec/AGENTS.md` 現在明確要求：可比較的數值資訊要先拆成欄位，再做右對齊格式化；描述文字應與數值分欄處理。
- 這條規則已套用到 Quota mode 的 drift 顯示，像 `+1.1% Overuse`、`-55.4% Under` 會拆成數值欄與描述欄。
- Quota rows 現在和 Delta panel 一樣遵守「數值先拆欄、再對齊」的核心原則，不再只修單一模式。
- 驗證：
  `npm run build`
  `node --test tests/root-table-layout.test.js`
  `node --test tests/root-option-layout.test.js`
  `node --test tests/root-panel-layout.test.js`
  `node --test tests/workload-tier.test.js`
  `node --test tests/entrypoints.test.js`

# 2026-03-20 pace 色盤對比修正

- [x] 將背景色可讀性規則加入 spec core rule，要求 light/dark theme 都可讀
- [x] 補測試鎖住 overuse 使用較深的背景色階與明確前景色
- [x] 將 pace 色盤改為深色底搭配顯式前景色，降低亮色底在不同主題下的對比風險
- [x] 更新落地 spec 並完成驗證

## Review

- `spec/AGENTS.md` 現在明確要求：背景色需採 light/dark theme 都通用的色調，必要時指定前景色；overuse 應優先使用較深警示色。
- overuse 色盤已從偏亮的紅/橘色改成較深的紅棕色階，並固定搭配白字。
- 這條色盤同時影響 pacing 文字與 drift bar 的嚴重度顏色，避免同一套語意出現不一致的亮度。
- 驗證：
  `npm run build`
  `node --test tests/root-option-layout.test.js`
  `node --test tests/root-panel-layout.test.js`
  `node --test tests/root-table-layout.test.js`
  `node --test tests/workload-tier.test.js`
  `node --test tests/entrypoints.test.js`

# 2026-03-21 NLSDD self-hosting 多 lane / 4 active threads

- [x] 為 `NLSDD` 自身建立一個 self-hosting execution，使用 lane pool > 4 與 active cap = 4 的模型
- [x] 補齊 schedule helper 與相關共用函式，讓 `queued` / `refill-ready` / active thread usage 可被一致推導
- [x] 將 scoreboard 與 execution docs 對齊新的 scheduling model，能顯示 queued lanes 與下一個 dispatch 建議
- [x] 補 NLSDD scheduler regression tests 與 CLI smoke checks
- [x] 用 NLSDD 派出 4 個 active subagents，沿著 self-hosting lanes 實作上述變更
- [x] 執行建置、NLSDD tooling tests 與 schedule/scoreboard 指令驗證

## Review

- 新增 `nlsdd-self-hosting` execution，lane pool 共有 6 條，但 scheduler 與 scoreboard 以 4 個 active thread slots 為上限；Lane 1-4 是 initial active set，Lane 5-6 保持 queued follow-up。
- `NLSDD/scripts/nlsdd-lib.cjs` 現在支援 schedule-aware phase 推導、4-thread dispatch suggestion，以及 markdown table row 在 backtick code span 內含 `|` 時的安全解析。
- `NLSDD/scripts/nlsdd-refresh-scoreboard.cjs` 會在重寫 scoreboard row 前 escape table cell 內的 `|`，避免 refresh 後再把 scheduler parser 餵壞。
- `NLSDD/scoreboard.md` 現在明確標示 self-hosting execution 的 initial active set、queued follow-up lanes 與 dispatch 順序。
- `NLSDD/executions/nlsdd-self-hosting/` 與 `NLSDD/executions/plot-mode/` 都已改成 lane-pool + active-cap 語言，不再把 lane 數量和 active thread 數量綁死。
- 這輪實際用 4 個 active subagents 跑了 scheduler、scoreboard、rules/docs、tests 4 條線，並在 slot 釋放後用 queued lanes 做 refill，驗證 `NLSDD` 可以用來開發 `NLSDD` 本身。
- 驗證：
  `node --test tests/nlsdd-automation.test.js`
  `node NLSDD/scripts/nlsdd-suggest-schedule.cjs --execution nlsdd-self-hosting`
  `npm run nlsdd:scoreboard:refresh`
  `npm run build`

# 2026-03-20 delta 欄距與 pacing 拆欄修正

- [x] 收緊 `🔄 in` 區段欄距，避免時間值前方出現過寬空白
- [x] 將 Delta mode 的 `Pacing` 拆成 prefix / 數值 / 描述三欄對齊
- [x] 保持 adopted row 僅對數值欄與描述欄上色，不擴散到 `Pacing` prefix
- [x] 補 panel 測試，鎖住 reset 欄位與 pacing 數值欄、描述欄的對齊
- [x] 執行建置與測試驗證，更新落地 spec

## Review

- `Delta` mode 現在不再把 `Pacing +0.9% Overuse` 當成整段字串補空白，而是拆成 `Pacing`、`+0.9%`、`Overuse` 三欄分別對齊。
- adopted row 的高亮只套在 `+0.9%` 與 `Overuse` 兩欄，prefix 保持中性。
- `🔄 in` 後方時間欄與括號百分比欄的距離已縮緊，避免像 `   6.6d  ` 這種過寬間距。
- full panel 與 quota 欄位對齊現在使用當前資料集的實際寬度，不再用 `999.9d`、`+100.0%` 這種固定樣板把欄位撐太鬆。
- 驗證：
  `npm run build`
  `node --test tests/root-option-layout.test.js`
  `node --test tests/root-panel-layout.test.js`
  `node --test tests/root-table-layout.test.js`
  `node --test tests/workload-tier.test.js`
  `node --test tests/entrypoints.test.js`

# 2026-03-20 quota prompt 移除 delta 比較區塊

- [x] 補 Quota prompt 測試，鎖住首行 header 保留、下方只剩 quota 欄位
- [x] 將 Quota prompt block 改成只渲染 bar、time to reset、usage left
- [x] 移除 Quota prompt 中的 drift / bottleneck 比較資訊
- [x] 更新已落地 spec 並完成驗證

## Review

- Quota mode 現在只保留 quota 視角資訊，不再顯示 `% Overuse/Under` 或 `<- Bottleneck` 這類 delta 比較內容。
- 這讓 Quota mode 和 Delta mode 的職責更清楚：Quota 看剩餘量與重置時間，Delta 看 pacing 比較。
- 驗證：
  `npm run build`
  `node --test tests/root-option-layout.test.js`

# 2026-03-20 prompt row 移除 emoji 標記

- [x] 將 Delta prompt row 的 `📊`、`🔄 in` 改為寬度穩定的文字標籤
- [x] 將「避免 emoji 影響對齊」提升為 spec core rule
- [x] 補 panel / option 測試，鎖住 `...% left` 與 `reset ... (...)` 文案
- [x] 修正 Node 25 下明確 `index` 路徑的目錄匯入，恢復 prompt tests 驗證

## Review

- Delta prompt row 現在改用 `94% left` 與 `reset 6.6d (95%)`，不再依賴 emoji 當欄位標記。
- 這讓 JuiceSSH 這類對 Unicode cell width 較敏感的終端，不會因 emoji 造成同列後續欄位位移。
- `spec/AGENTS.md` 現在明確要求：需要嚴格欄位對齊的 prompt row 應避免 emoji 或其他寬度不穩定字元。
- 驗證：
  `npm run build`
  `node --test tests/root-panel-layout.test.js`
  `node --test tests/root-option-layout.test.js`
  `node --test tests/root-table-layout.test.js`
  `node --test tests/workload-tier.test.js`
  `node --test tests/entrypoints.test.js`

# 2026-03-20 quota option 清除 pacing 殘留

- [x] 補 option 測試，鎖住 `Quota` mode 選項不再重用 pacing delta
- [x] 將 option renderer 改成 mode-aware，`Delta` 顯示 delta、`Quota` 只留 indicator 與 profile
- [x] 更新已落地 spec，反映 `Quota` mode 選項不含 pacing/delta

## Review

- 根因：下方 option label 原本不看 `barStyle`，無論 `Delta` 或 `Quota` 都固定走 `optionDeltaValue()`。
- 修正：`renderSelectionOption(...)` 現在接收 `barStyle`，只有 `Delta` mode 才顯示 delta；`Quota` mode 會收斂成純 `indicator + profile name`。
- 驗證：
  `npm run build`
  `node --test tests/root-option-layout.test.js`
  `node --test tests/root-panel-layout.test.js`
  `node --test tests/root-table-layout.test.js`
  `node --test tests/workload-tier.test.js`
  `node --test tests/entrypoints.test.js`
  `node --test tests/root-panel-layout.test.js`
  `node --test tests/root-table-layout.test.js`
  `node --test tests/workload-tier.test.js`
  `node --test tests/entrypoints.test.js`

# 2026-03-20 adaptive prompt density MVP

- [x] 補 density 決策測試，鎖住 `full` / `condensed` 切換條件
- [x] 補 condensed Delta / Quota prompt 測試，鎖住每個 profile 從 3 行壓到 2 行
- [x] 新增 prompt density helper，依 profile 數量、終端高度與 bar mode 決定 density
- [x] 實作 condensed Delta 與 condensed Quota renderer，維持模式語意但壓低垂直高度
- [x] 更新已落地 spec 並完成驗證

## Review

- 新增 `PromptDensity` 決策，當 profile 數量與終端高度造成過高垂直壓力時，prompt panel 會從 `full` 切到 `condensed`。
- `Delta` condensed mode 仍保留 `profile + last update` 與 `W:` / `5H:` 語意，但把兩個 window 摘要壓到同一行。
- `Quota` condensed mode 同樣壓成單一 detail line，並維持 quota-only 視角，不回帶 delta/drift 比較內容。
- density trigger 現在用實際可見 detail lines 估算，不再固定每組一律當成 3 行。
- 缺少 `5H` 的 profile 在 condensed 模式下仍維持穩定的兩行 block，不會留下多餘分隔符。
- option list 行為沒有改變，adaptive density 只影響上方 prompt panel。
- 驗證：
  `npm run build`
  `node --test tests/root-option-layout.test.js`
  `node --test tests/root-panel-layout.test.js`
  `node --test tests/root-table-layout.test.js`
  `node --test tests/workload-tier.test.js`
  `node --test tests/entrypoints.test.js`

# 2026-03-21 workload tier influence hint MVP

- [x] 補 workload tier 測試，鎖住 tier 對應的簡短說明文案
- [x] 選定共享 status line 作為 hint surface，不把說明塞回 option list
- [x] 實作 workload tier hint helper 與 status line 文案
- [x] 更新已落地 spec 並完成驗證

## Review

- 新增一個低噪音的 workload tier hint，會顯示在共享 status line，例如 `Workload Low: conserve short-window capacity`。
- 這個 MVP 只補充目前 tier 對 routing 偏向的說明，不改動任何 ranking 權重。
- option list 維持既有極簡設計；`Delta` mode 仍顯示 delta，`Quota` mode 仍只顯示 indicator 與 profile。
- 驗證：
  `npm run build`
  `node --test tests/workload-tier.test.js`
  `node --test tests/root-option-layout.test.js`
  `node --test tests/root-panel-layout.test.js`
  `node --test tests/root-table-layout.test.js`
  `node --test tests/entrypoints.test.js`

# 2026-03-21 workload influence indicator MVP

- [x] 補 option 測試，鎖住選項列出現 compact influence marker
- [x] 將 option renderer 接上 `[W]` / `[5H]` influence indicator
- [x] 保持 `Delta` / `Quota` mode 的 option 語意簡潔，不回帶長說明文字
- [x] 更新已落地 spec 並完成驗證

## Review

- option list 現在會帶一個 compact `[W]` / `[5H]` marker，讓目前推薦主要受哪個 window 影響更容易掃到。
- `Delta` mode 仍保留 delta；`Quota` mode 仍不回帶 pacing 百分比，只額外帶 influence marker。
- 這個 indicator 採極短標記，不會把下方選項重新膨脹成解說區。
- 驗證：
  `npm run build`
  `node --test tests/root-option-layout.test.js`
  `node --test tests/workload-tier.test.js`
  `node --test tests/root-panel-layout.test.js`
  `node --test tests/root-table-layout.test.js`
  `node --test tests/entrypoints.test.js`

# 2026-03-21 workload tier persistence MVP

- [x] 補 persistence 測試，鎖住 UI state 檔讀寫與 root command 讀取/寫回 tier
- [x] 新增本機 UI state 檔，將 workload tier 存在 `~/.codex`
- [x] 啟動時恢復上次 workload tier，切換 `W` 時即時寫回
- [x] 更新已落地 spec 並完成驗證

## Review

- 新增 `~/.codex/codex-auth-ui-state.json` 作為輕量 UI state 檔，目前先只存 `workloadTier`。
- root command 啟動時會讀取上次 tier；如果檔案缺失、格式錯誤或值無效，會安全回退到 `Auto`。
- 使用者按 `W` 切換 tier 時，新的 workload tier 會立即寫回，所以下次啟動會延續上次選擇。
- 驗證：
  `npm run build`
  `node --test tests/ui-state.test.js`
  `node --test tests/workload-tier.test.js`
  `node --test tests/root-option-layout.test.js`
  `node --test tests/root-panel-layout.test.js`
  `node --test tests/root-table-layout.test.js`
  `node --test tests/entrypoints.test.js`

# 2026-03-21 table-body influence indicator MVP

- [x] 補 panel / option 測試，鎖住 detail row 的 `W:*` / `5H:*` bottleneck marker
- [x] 將 full / condensed、Delta / Quota 的 prompt window label 接上 influence marker
- [x] 保持 marker 極短，不把 detail row 與 quota row 重新拉寬成說明句
- [x] 更新已落地 spec 並完成驗證

## Review

- detail panel 與 quota rows 現在都會在被採用的影響來源列上顯示 `W:*` 或 `5H:*`。
- 這讓 option list 的 `[W]` / `[5H]` marker 和 body/panel 內的視覺訊號一致，使用者在上下兩區都能快速對上影響來源。
- 這次只加最短的 `*` 標記，不增加額外顏色或長文案，所以欄位對齊仍可維持。
- 驗證：
  `npm run build`
  `node --test tests/root-panel-layout.test.js`
  `node --test tests/root-option-layout.test.js`
  `node --test tests/workload-tier.test.js`
  `node --test tests/ui-state.test.js`
  `node --test tests/root-table-layout.test.js`
  `node --test tests/entrypoints.test.js`

# 2026-03-21 plot mode 固定 lane orchestration 規劃

- [x] 盤點目前 plot mode roadmap 與已落地的 sub-agent workstream
- [x] 將並行協作收斂成 4 條固定、不重疊的 lane family
- [x] 為每條 lane 寫出 lane-local MVC checklist 與 refill 順序
- [x] 更新 sub-agent 守則，要求後續 agent 沿 lane plan 指派並回報 MVC 進度

## Review

- 新增 [`NLSDD/executions/plot-mode/overview.md`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/executions/plot-mode/overview.md) 作為總覽，固定 plot-mode execution 的 lanes 與各自 ownership family。
- 新增 4 份 lane plan：
  `NLSDD/executions/plot-mode/lane-1.md`
  `NLSDD/executions/plot-mode/lane-2.md`
  `NLSDD/executions/plot-mode/lane-3.md`
  `NLSDD/executions/plot-mode/lane-4.md`
- [`spec/NLSDD/guardrails.md`](/home/jethro/repo/side-projects/codex-account-switcher/spec/NLSDD/guardrails.md) 現在明確要求：sub-agent 應先從 lane plan 指派，回報要帶 `Lane + MVC step`，且 refill 應優先消耗同 lane 的下一個 unchecked item。
- [`plan/2026-03-20-multi-phase-routing-plan.md`](/home/jethro/repo/side-projects/codex-account-switcher/plan/2026-03-20-multi-phase-routing-plan.md) 也同步記錄這次 lane-based orchestration 決策。

# 2026-03-21 NLSDD operating rules 落地

- [x] 將 lane-based workflow 正式收斂為 repo-native `NLSDD`
- [x] 新增 `NLSDD` 核心 operating rules 文件
- [x] 將 NLSDD 定義收斂到 `spec/NLSDD/`，並將 execution、scoreboard、scripts 與執行流程文件集中到 `NLSDD/`
- [x] 明確規範 per-lane worktree、per-item commit、commit-diff review、coordinator-owned tracking
- [x] 將 Lane 4 對 Lane 2 render boundary 的依賴寫回 lane plan，避免再用執行中 scope expansion 硬撐
- [x] 新增 repo 內 scoreboard、communication 通道文件與 autopilot / probe 規則
- [x] 將 Rust `target/` 噪音治理拆成 `.gitignore` + lane-local 去追蹤清理

## Review

- 新增 [`spec/NLSDD/operating-rules.md`](/home/jethro/repo/side-projects/codex-account-switcher/spec/NLSDD/operating-rules.md)，作為 repo 內建的 NLSDD workflow 定義。
- 新增 [`spec/NLSDD/guardrails.md`](/home/jethro/repo/side-projects/codex-account-switcher/spec/NLSDD/guardrails.md)、[`spec/NLSDD/communication.md`](/home/jethro/repo/side-projects/codex-account-switcher/spec/NLSDD/communication.md)、[`NLSDD/scoreboard.md`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scoreboard.md)。
- plot-mode lane docs 現在集中在 `NLSDD/executions/plot-mode/`，不再散落於 `plan/` 根目錄。
- `.gitignore` 現在忽略 `rust/plot-viewer/target/`，後續再配合 lane-local cleanup 去掉已進索引的 artifacts。

# 2026-03-21 plot mode NLSDD 第一輪執行

- [x] 為 Lane 1-4 建立 lane-local worktrees，避免 reviewer 被 shared dirty worktree 汙染
- [x] 讓 Lane 1 完成 real Rust viewer handoff 驗證，並走完 spec / quality review
- [x] 讓 Lane 2 完成 panel boundary seam 擴張與 drawable layout 修正，並走完 spec / quality review
- [x] 讓 Lane 3 完成 `chart.rs` 的 local view-model 抽離，並走完 spec / quality review
- [x] 讓 Lane 4 消費 Lane 2 seam，做出第一個可見 Summary / Compare panel surface，並走完 spec / quality review
- [x] 由 coordinator 回寫 lane plans、overview plan 與主線 progress

## Review

- Lane 1 在 [`.worktrees/lane-1-node`](/home/jethro/repo/side-projects/codex-account-switcher/.worktrees/lane-1-node) 完成 real-binary handoff test，修正後綁回 `RootCommand.prototype.launchPlotViewer`，不再只是旁路測試。
- Lane 2 在 [`.worktrees/lane-2-runtime`](/home/jethro/repo/side-projects/codex-account-switcher/.worktrees/lane-2-runtime) 先做 panel boundary expansion，再補一次 correction，把 `PanelRenderContext` 分清楚 outer `area` 與實際 drawable `content_area/layout`。
- Lane 3 在 [`.worktrees/lane-3-chart`](/home/jethro/repo/side-projects/codex-account-switcher/.worktrees/lane-3-chart) 將 `chart.rs` 抽成私有 `ChartViewModel`，把 focused profile、7d bounds、5h band availability 的推導邏輯留在 chart lane 內。
- Lane 4 在 [`.worktrees/lane-4-panels`](/home/jethro/repo/side-projects/codex-account-switcher/.worktrees/lane-4-panels) 消費 Lane 2 seam，補上 `Selected / Current / Focus / Pairing` 這類真正可見的 Summary / Compare body 內容。
- 這輪證明 `NLSDD` 可實際運作：每個 lane item 都有 lane-local commit，reviewer 只看該 item diff，correction loop 也能留在原 lane 內處理。
- 驗證：
  - `node --test tests/plot-handoff.test.js` in `.worktrees/lane-1-node`
  - `cargo test --manifest-path rust/plot-viewer/Cargo.toml` in `.worktrees/lane-2-runtime`
  - `cargo check --manifest-path rust/plot-viewer/Cargo.toml` in `.worktrees/lane-2-runtime`
  - `cargo test --manifest-path rust/plot-viewer/Cargo.toml` in `.worktrees/lane-3-chart`
  - `cargo test --manifest-path rust/plot-viewer/Cargo.toml` in `.worktrees/lane-4-panels`

# 2026-03-21 NLSDD 集中化與 noise cleanup

- [x] 建立 `NLSDD` 專屬執行區，將 execution、scoreboard、scripts 與流程文件集中到 `NLSDD/`
- [x] 將 plot-mode lane docs 搬進 `NLSDD/executions/plot-mode/`
- [x] 加入 `autopilot refill`、`lane status probe`、`coordinator sidecar` 溝通規則
- [x] 以 `.gitignore` 管理未追蹤的 Rust `target/` 輸出
- [x] 在含有 tracked artifacts 的 lane worktree 上做 lane-local 去追蹤清理

## Review

- `spec/NLSDD/` 現在只承接已完成且已驗證的 NLSDD 定義；`NLSDD/` 承接 execution、scoreboard、scripts 與執行流程文件，避免把 runtime artifacts 混進 spec。
- [`NLSDD/scoreboard.md`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scoreboard.md) 集中記錄 execution、lane、phase、latest commit、next refill target 與 noise 狀態。
- [`spec/NLSDD/communication.md`](/home/jethro/repo/side-projects/codex-account-switcher/spec/NLSDD/communication.md) 將 reviewer / implementer / coordinator 的 sidecar 通道與 arbitration 規則固定化。
- 噪音治理現在明確分成兩層：`.gitignore` 處理未追蹤 `target/`，lane worktree 再各自清掉已進索引的 `target/**`。

# 2026-03-21 NLSDD scoreboard 半自動化第一版

- [x] 新增 scoreboard refresh script，從 lane worktrees 自動回填 `Branch HEAD`、`Last probe`、`Noise`
- [x] 從 `~/.codex/state_5.sqlite` 補 recent Codex threads 附錄，作為 agent activity sidecar
- [x] 將 scoreboard 調整成手動欄位與自動欄位並存
- [x] 補 npm script 接線並完成實際 refresh 驗證

## Review

- 新增 [`NLSDD/scripts/nlsdd-refresh-scoreboard.cjs`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scripts/nlsdd-refresh-scoreboard.cjs)，會讀取 `NLSDD/scoreboard.md`、lane plans、lane worktrees，以及 `~/.codex/state_5.sqlite`。
- scoreboard 現在保留 `Current item`、`Phase`、`Item commit`、`Blocked by`、`Next refill target`、`Notes` 這些人工決策欄位，同時自動更新 `Branch HEAD`、`Last probe`、`Noise`。
- 自動附錄的 `Recent Codex Threads` 會列出最近在這個 repo cwd 活動的 subagent nickname、role、thread id 與 updated time，讓 coordinator 不用手動去 `.codex` 查。
- 驗證：
  - `node NLSDD/scripts/nlsdd-refresh-scoreboard.cjs`

# 2026-03-21 NLSDD v2 自動化

- [x] 升級 scoreboard refresh，從 `sessions/*.jsonl` 推導 lane-level `Effective phase`、`Latest event`、`Correction count`、`Last activity`
- [x] 新增 lane probe helper，將 source diff、artifact noise 與 lane-local verification 集中輸出
- [x] 新增 refill assistant，為 `refill-ready` lane 建議下一個 lane-local item
- [x] 新增 sidecar message helper，產生 implementer / reviewer / correction 模板草稿
- [x] 更新 NLSDD docs 與 scoreboard 欄位，明確區分人工欄位與自動欄位

## Review

- 新增 [`NLSDD/scripts/nlsdd-lib.cjs`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scripts/nlsdd-lib.cjs) 集中處理 scoreboard 表格、lane plan、thread/session event、phase 推導與 refill item 抽取。
- 新增 [`NLSDD/scripts/nlsdd-probe-lane.cjs`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scripts/nlsdd-probe-lane.cjs)、[`NLSDD/scripts/nlsdd-suggest-refill.cjs`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scripts/nlsdd-suggest-refill.cjs)、[`NLSDD/scripts/nlsdd-compose-message.cjs`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scripts/nlsdd-compose-message.cjs)。
- [`NLSDD/scoreboard.md`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scoreboard.md) 現在同時承載 manual `Phase` 與 auto-derived `Effective phase`，避免 automation 直接覆寫 coordinator 意圖。
- `autopilot refill` 在 v2 仍維持 assistive 模式：腳本只建議下一個 item，不直接派工或改 checklist。

# 2026-03-21 NLSDD 多 lane / 4 active threads 模型

- [x] 將 NLSDD 規則改成 lane pool 可超過 4，但同時只維持 4 個 active subagents
- [x] 補上 scheduler helper，讓 coordinator 能看到 active thread 使用量、可用 slot 與下一批 dispatch 建議
- [x] 更新 scoreboard / overview 文件，明確區分 queued / parked lane 與 active lane

## Review

- [`spec/NLSDD/operating-rules.md`](/home/jethro/repo/side-projects/codex-account-switcher/spec/NLSDD/operating-rules.md) 現在以 `lane pool size + active subagent cap` 取代舊的固定 `active lane count`。
- [`NLSDD/scripts/nlsdd-suggest-schedule.cjs`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scripts/nlsdd-suggest-schedule.cjs) 會彙整 active lanes、refill-ready lanes、queued lanes 與 dispatch suggestions。
- [`NLSDD/scoreboard.md`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scoreboard.md) 補了建議 phase vocabulary，讓超過 4 條 lane 時仍能用 `queued` / `parked` 管理非 active lanes。

# 2026-03-21 NLSDD worktree-local root 解析修正

- [x] 找出為什麼在 recovery branch 執行 `nlsdd-refresh-scoreboard` / `schedule` / `refill` 仍會吃到主 repo root 的 NLSDD surface
- [x] 將 root 解析改成先找目前 worktree 自己的 `NLSDD` surface，再回退到 canonical repo root
- [x] 補 linked worktree 測試，鎖住 worktree-local scoreboard / lane plan 會優先被讀取

## Review

- 根因是 `resolveProjectRoot()` 先用 git common-dir 把 linked worktree 收斂回主 repo root，導致 recovery branch 內的 manual scoreboard 與 execution docs 即使不同，也會被 `refresh` / `schedule` / `refill` 忽略。
- 修正後改成優先從 `cwd` 往上找最近的 `NLSDD/scoreboard.md` 或 `NLSDD/executions/`；只有找不到時才回退到原本的 canonical repo root 邏輯。
- 這樣 `NLSDD` tooling 在 recovery branch / linked worktree 中會先吃當前 branch 的 execution surface，不再跨 branch 漂移。

# 2026-03-21 NLSDD worktree pool root 解析修正

- [x] 找出為什麼 recovery branch 讀到自己的 lane docs 後，`.worktrees/...` 仍會被誤判成 `missing-worktree`
- [x] 將 lane plan 的 worktree 路徑解析改成使用 canonical worktree pool root，而不是 execution root
- [x] 補 linked worktree 測試，鎖住「讀當前 branch 的 lane docs，但 worktree path 仍指向共用 repo `.worktrees/`」的行為

## Review

- 根因是 `loadLanePlan()` 先正確讀到了 recovery branch 自己的 lane docs，但仍把 `NLSDD worktree: .worktrees/...` 相對於 execution root 解析，導致 recovery branch 下的 probe/schedule 看到 `missing-worktree`。
- 修正後新增 `resolveWorktreePoolRoot()`，讓 execution docs 仍取自當前 worktree，但 `.worktrees/...` 會回到 git common-dir 對應的 canonical repo root 解析。
- 這讓 linked worktree / recovery branch 具備正確的雙 root 模型：`execution root` 負責文件與 scoreboard，`worktree pool root` 負責 lane worktree 實體位置。

# 2026-03-21 NLSDD anti-convergence guardrail

- [x] 找出為什麼 4a execution 仍可能在實務上收束成單一 critical lane，讓其他 slot 只是在等待
- [x] 將「避免單一 lane blocking 偽飽和」寫進 `spec/NLSDD/operating-rules.md`
- [x] 將 convergence warning、replan trigger 與多 plan/execution 並行策略寫進 `spec/NLSDD/guardrails.md`
- [x] 將執行側提醒同步到 `NLSDD/AGENTS.md` 與 `tasks/lessons.md`

## Review

- `NLSDD` 現在明確把「單一 lane 收束導致 2-3 個 slot 空等」定義成 smell，而不是可接受的 4-active 狀態。
- coordinator 遇到這種情況時，優先順序改成：
  - 切出新的獨立 lane
  - 或改成同時推進 2-3 個 plans/executions
  - 只有在沒有 truthful parallel work 時，才接受暫時降到低併行度。
- 這條 guardrail 直接來自本輪 `plot-mode` 觀察：Lane 4 一度成為唯一有真實前進的 lane，而 Lane 1/3/5 多次回到 `NOOP` 或 clean probe。

# 2026-03-21 NLSDD stale-implementing detection

- [x] 在 scheduler / probe 邏輯中辨識「lane journal 還是 implementing，但 worktree clean 且 HEAD == latestCommit」的 stale 狀態
- [x] 讓 stale lane 不再被算入 active thread usage
- [x] 補 regression test，鎖住 stale lane 會進入 `staleRows` 而不是 `activeRows`

## Review

- [`NLSDD/scripts/nlsdd-lib.cjs`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scripts/nlsdd-lib.cjs) 新增 `inspectLaneWorktree()` 與 `detectStaleImplementing()`，讓 schedule 在 lane journal 之外也會看 worktree truth。
- [`NLSDD/scripts/nlsdd-suggest-schedule.cjs`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scripts/nlsdd-suggest-schedule.cjs) 現在會額外輸出 `Stale implementing lanes`。
- regression 在 [tests/nlsdd-automation.test.js](/home/jethro/repo/side-projects/codex-account-switcher/tests/nlsdd-automation.test.js)；驗證上 `node --test tests/nlsdd-automation.test.js` 與 `node NLSDD/scripts/nlsdd-suggest-schedule.cjs --execution plot-mode` 已通過。

# 2026-03-21 NLSDD dispatch cycle helper

- [x] 將 deterministic coordinator work 收斂成單一 cycle helper
- [x] 讓 cycle helper 一次完成 stale lane reconcile、runtime refresh 與下一批 lane promotion
- [x] 讓 cycle helper 回傳 completed lanes、promoted lanes 與 idle slots
- [x] 補 regression test，鎖住 cycle 會先收 stale lane 再 promote 下一條 queued lane

## Review

- 新增 [`NLSDD/scripts/nlsdd-run-cycle.cjs`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scripts/nlsdd-run-cycle.cjs) 與 `npm run nlsdd:cycle`。
- cycle helper 現在會先把可由 tracked phase 解決的 `stale-implementing` lane 收回 truthful phase，再按既有 plan/journal 自動 promote 下一批 dispatchable lanes。
- 輸出會明確列出：
  - `Reconciled lanes`
  - `Promoted lanes`
  - `Idle slots`
  - `No dispatch reason`
- regression 在 [tests/nlsdd-automation.test.js](/home/jethro/repo/side-projects/codex-account-switcher/tests/nlsdd-automation.test.js)；驗證上 `node --test tests/nlsdd-automation.test.js` 與 `node NLSDD/scripts/nlsdd-run-cycle.cjs --execution plot-mode --json` 已通過。

# 2026-03-21 NLSDD launch-active-set bridge

- [x] 在 cycle helper 之上新增一層 launch bridge，讓單次指令除了 reconcile/promote，也能回傳 coordinator-ready assignment bundles
- [x] 讓 assignment bundle 直接吃 lane plan 的 ownership family、verification commands 與 next actionable item
- [x] 補 regression test，鎖住 promoted lane 會產出 implementer-assignment 文案與對應 scope/verification

## Review

- 新增 [`NLSDD/scripts/nlsdd-launch-active-set.cjs`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scripts/nlsdd-launch-active-set.cjs) 與 `npm run nlsdd:launch`，讓 coordinator 用一個指令就能拿到 `completed lanes`、`promoted lanes`、`idle slots` 與每條 promoted lane 的 handoff message。
- [`NLSDD/scripts/nlsdd-lib.cjs`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scripts/nlsdd-lib.cjs) 的 lane plan parser 現在會保留 `Ownership family`，避免 assignment bundle 只能回 generic scope。
- regression 在 [tests/nlsdd-automation.test.js](/home/jethro/repo/side-projects/codex-account-switcher/tests/nlsdd-automation.test.js)；驗證上 `node --test tests/nlsdd-automation.test.js` 與 `node NLSDD/scripts/nlsdd-launch-active-set.cjs --execution plot-mode --json --dry-run` 已通過。

# 2026-03-21 NLSDD review-loop driver

- [x] 新增 review helper，根據 lane 當前 phase 自動整理下一步 coordinator action
- [x] 支援 `spec-review-pending`、`quality-review-pending`、`correction` 與 `coordinator-commit-pending`
- [x] 補 regression test，鎖住 review helper 會產出正確的 review / correction bundle

## Review

- 新增 [`NLSDD/scripts/nlsdd-drive-review-loop.cjs`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scripts/nlsdd-drive-review-loop.cjs) 與 `npm run nlsdd:review`，讓 coordinator 不必逐條掃 lane docs/journal，就能拿到下一步 review action。
- helper 目前會優先吃 lane journal phase，再回退到 runtime/tracked scoreboard phase，並重用既有的 `spec-review`、`quality-review`、`correction-loop` 模板；若 lane 已進 `coordinator-commit-pending`，則輸出 commit intake bundle。
- regression 在 [tests/nlsdd-automation.test.js](/home/jethro/repo/side-projects/codex-account-switcher/tests/nlsdd-automation.test.js)；驗證上 `node --test tests/nlsdd-automation.test.js` 與 `node NLSDD/scripts/nlsdd-drive-review-loop.cjs --execution plot-mode --json` 已通過。

# 2026-03-21 NLSDD READY_TO_COMMIT intake helper

- [x] 將 commit-ready handoff 的 title/body/verification 結構化寫進 lane journal
- [x] 新增 intake helper，讓 coordinator 可直接收 `coordinator-commit-pending` lanes 的 commit bundle
- [x] 補 regression test，鎖住 intake helper 會回傳 proposed commit title/body、scope、verification 與 note

## Review

- [`NLSDD/scripts/nlsdd-record-lane-state.cjs`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scripts/nlsdd-record-lane-state.cjs) 現在支援 `--commit-title` 與 `--commit-body`，讓 `READY_TO_COMMIT` handoff 不只剩 phase 與 note。
- 新增 [`NLSDD/scripts/nlsdd-intake-ready-to-commit.cjs`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scripts/nlsdd-intake-ready-to-commit.cjs) 與 `npm run nlsdd:intake`，可直接列出目前可由 coordinator 收單提交的 lanes。
- [`NLSDD/scripts/nlsdd-drive-review-loop.cjs`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scripts/nlsdd-drive-review-loop.cjs) 的 `coordinator-commit-needed` 輸出也會帶 proposed commit title/body，避免 review helper 和 intake helper 出現兩套 commit 資訊。

# 2026-03-21 NLSDD coordinator autopilot loop

- [x] 將 launch / review / intake 三條 coordinator 助手收成單一 autopilot loop
- [x] 讓單次指令回傳 promoted lanes、review actions、commit intake 與 idle slots
- [x] 補 regression test，鎖住 autopilot 會同時看見 dispatch、review 與 READY_TO_COMMIT 收單

## Review

- 新增 [`NLSDD/scripts/nlsdd-run-coordinator-loop.cjs`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scripts/nlsdd-run-coordinator-loop.cjs) 與 `npm run nlsdd:autopilot`，讓 coordinator 可一次取得整輪 deterministic 狀態摘要。
- autopilot 目前是 assistive automation：它會執行 cycle/launch 的狀態收斂，並彙整 review/intake 結果，但不直接代 main agent 呼叫 subagent 工具。
- regression 在 [tests/nlsdd-automation.test.js](/home/jethro/repo/side-projects/codex-account-switcher/tests/nlsdd-automation.test.js)；驗證上 `node --test tests/nlsdd-automation.test.js` 與 `node NLSDD/scripts/nlsdd-run-coordinator-loop.cjs --execution plot-mode --json` 已通過。

# 2026-03-21 NLSDD dispatch plan helper

- [x] 在 autopilot 之上新增 dispatch-plan helper，輸出 main agent 可直接照單執行的 action queue
- [x] 將 queue 預設優先順序定成 commit intake -> review/correction -> launch assignment
- [x] 補 regression test，鎖住 dispatch plan 會依優先順序輸出 queue

## Review

- 新增 [`NLSDD/scripts/nlsdd-build-dispatch-plan.cjs`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scripts/nlsdd-build-dispatch-plan.cjs) 與 `npm run nlsdd:dispatch-plan`，讓 main agent 可直接取得本輪 action queue。
- helper 不重新計算 lane truth，而是直接吃 [`NLSDD/scripts/nlsdd-run-coordinator-loop.cjs`](/home/jethro/repo/side-projects/codex-account-switcher/NLSDD/scripts/nlsdd-run-coordinator-loop.cjs) 的輸出，避免再引入第四套 state。
- regression 在 [tests/nlsdd-automation.test.js](/home/jethro/repo/side-projects/codex-account-switcher/tests/nlsdd-automation.test.js)；驗證上 `node --test tests/nlsdd-automation.test.js` 與 `node NLSDD/scripts/nlsdd-build-dispatch-plan.cjs --execution plot-mode --json` 已通過。

# 2026-03-21 NLSDD honest next-item rewrite

- [x] 檢視 `plot-mode` 與 `nlsdd-self-hosting` 的 insights、lane docs、scoreboard 是否仍重派已被現有 commit 吃完的 item
- [x] 將 `plot-mode Lane 2` 改成 parked，停止從已滿足的 compare seam wording 重派 runtime lane
- [x] 將 `plot-mode Lane 4` 改寫為 adopted-target emphasis 的 honest next item
- [x] 將 `nlsdd-self-hosting Lane 4` 改寫為 scoreboard/schedule cross-check coverage 的 honest next item
- [x] 將 `nlsdd-self-hosting Lane 7` 改寫為 scheduler/runtime truth audit 的 honest next item

## Review

- 根因不是 scheduler 算錯，而是 tracked lane docs 還保留舊的 current/refill wording，導致 autopilot 會把已被 `noop-satisfied` 的 lane 再次 promote 回 implementing。
- 這次先把 tracked planning surface 對齊 runtime truth：已完成的 item 直接打勾，只留下真正的下一個 reviewable item。這樣 `dispatch-plan`、人工 review 與 execution insights 看到的 lane intent 才會一致。
- 其中 `plot-mode Lane 2` 最適合直接 park，因為 `d361653` 已滿足 compare seam，而下一個 runtime item 尚未切成 concrete step；`plot-mode Lane 4`、`nlsdd-self-hosting Lane 4`、`Lane 7` 則各自留下新的 honest refill item。

# 2026-03-21 NLSDD 單一 handoff envelope 與 reducer 投影

- [x] 新增 canonical `lane handoff envelope` 與 `events.ndjson` event log
- [x] 將 `nlsdd-record-lane-state` / `nlsdd-record-insight` 改成相容 wrapper，先轉成 envelope 再寫入
- [x] 新增 reducer，從 envelope 投影 lane journal、execution insights、tracked scoreboard 與 lane plan `Current Lane Status`
- [x] 將 message helper 改成要求 subagent 回傳 strict envelope JSON
- [x] 補 automation tests，鎖住 envelope -> projection 鏈路與 active-set/review 既有行為

## Review

- 根因不是單一 script 出錯，而是 `record-lane-state`、`record-insight`、tracked scoreboard、lane plan status 都能各自落筆，導致同一個 execution 同時存在多套可寫 state surface。
- 這次先把「寫入」集中到單一 canonical interface：`lane handoff envelope`。不論是 lane state、insight、READY_TO_COMMIT、review result，最後都先進 `NLSDD/state/<execution>/events.ndjson`，再由 reducer 投影到其他 surfaces。
- 目前 `launch/review/intake/autopilot` 還是讀既有 projection surfaces，但因為這些 surfaces 已改成同一條 reducer 產物，狀態差異已大幅收斂；後續可以再把讀取端逐步改成直接吃 reducer snapshot。

# 2026-03-21 plot-mode Lane 4 stale runtime state cleanup

- [x] 檢查 `plot-mode Lane 4` 為何在 envelope/reducer 上線後仍殘留 `stale-implementing`
- [x] 將 tracked lane plan / scoreboard 收斂成 honest parked state，不再把已完成的 adopted-target emphasis 當成 current implementing item
- [x] 以 canonical envelope 寫入 Lane 4 的 parked transition，重建 reducer 投影
- [x] 驗證 `dispatch-plan` 不再把 Lane 4 列為 stale implementing

## Review

- 根因不是 reducer 壞掉，而是 `plot-mode Lane 4` 的歷史 bootstrapped event 仍把 `6bb1fba` 投影成 `implementing`，而 tracked lane plan / scoreboard 也還停留在舊 wording，導致 reducer 每次重播都忠實重建同一個假 active state。
- 這次先把 tracked planning surface 收斂成 honest truth：`6bb1fba` 已完成 adopted-target emphasis，所以 Lane 4 的 current item 改成等待下一個 fresh panel-local item，phase 改為 `parked`。

# 2026-03-21 nlsdd-self-hosting Lane 4 stale runtime state cleanup

- [x] 確認 `6d6c4e8` 的實際 diff 是否已完成 Lane 4 的 regression/CLI cross-check item
- [x] 將 `NLSDD/executions/nlsdd-self-hosting/lane-4.md` 與 `NLSDD/scoreboard.md` 收斂成 honest parked wording
- [x] 以 canonical envelope 寫入 Lane 4 的 parked transition，重建 reducer 投影
- [x] 將 Lane 4 舊的 adopted issues 收斂成 resolved，避免 insight summary 繼續重播已解決問題
- [x] 驗證 `dispatch-plan` / `schedule` / `insight summary` 不再把 Lane 4 列成 actionable stale work

## Review

- 根因和 `plot-mode Lane 4` 很像：`6d6c4e8` 其實已經完成 CLI smoke 與 scoreboard/schedule cross-check coverage，但 tracked wording、lane journal、insight lifecycle 都還停在 correction 當下，所以 reducer 忠實地把它重播成 `stale-implementing`。
- 這次不再試圖替 Lane 4 硬找下一個 item，而是先把 truth 收乾淨：cross-check coverage 已落地，Lane 4 回到 `parked`，之後只在真的出現新 regression/CLI surface 缺口時再重開。
- 也把兩條舊的 adopted insight 收成 resolved，避免 `review/autopilot/dispatch-plan` 還把它們當成目前的 actionable 問題。
- 接著再用 canonical envelope 寫入同一個 parked transition，讓 runtime journal、tracked scoreboard、lane plan `Current Lane Status` 與後續 `dispatch-plan` 全部由同一條事件鏈收斂到一致狀態。

# 2026-03-21 nlsdd-self-hosting Lane 7 pseudo-refill cleanup

- [x] 確認 `e853688` 已完成 Lane 7 當前 scheduler/runtime truth audit item
- [x] 將 `NLSDD/executions/nlsdd-self-hosting/lane-7.md` 與 `NLSDD/scoreboard.md` 收斂成 honest parked wording
- [x] 以 canonical envelope 寫入 Lane 7 的 parked transition，避免 quality pass 後又把同一 item 重派成 refill-ready
- [x] 驗證 `dispatch-plan` / `schedule` 不再把 Lane 7 當成可立即補位的假 refill

## Review

- 根因不是 Lane 7 還沒做完，而是 `quality pass -> refill-ready` 的通用 phase 在沒有 fresh next item 時，會把同一個已完成 item 又推出來一次。
- `e853688` 已經完成這輪 scheduler/runtime truth audit 的具體 delta：移除過度保守的 queued-only anti-convergence warning，並保留真正需要人工介入的警示條件。
- 因此這次和 Lane 4 一樣，先把 honest truth 收乾淨：Lane 7 回到 `parked`，之後只在真的出現新的 scheduler/runtime truth finding 時再重開。

# 2026-03-21 nlsdd-self-hosting execution idle sync

- [x] 將 self-hosting overview 的 Lane 2 / 3 / 4 / 7 狀態收斂成 parked truth
- [x] 將 Lane 7 已落地的 reducer / insight integrity checklist 標成完成
- [x] 驗證 `schedule` / `dispatch-plan` 對 `nlsdd-self-hosting` 都回到 `idleSlots: 4`

## Review

- 這一步不是新功能，而是把 `nlsdd-go` remediation round 跑完後的真相收進 tracked docs。
- runtime 已經先由 canonical envelope 收斂成全 execution idle；如果 overview 和 lane checklist 還停在早期的 active/remediation 語氣，之後讀 tracked docs 仍會以為 lane 還在進行中。
- 現在 `nlsdd-self-hosting` 的 honest state 是：沒有 active lane、沒有 dispatchable lane、沒有 actionable insight，之後只有在出現 fresh gap 時才重新喚醒對應 lane。

# 2026-03-21 NLSDD tracked lane status projection sync

- [x] 將 remaining self-hosting lane docs 同步成 honest parked/current-status wording
- [x] 將 remaining plot-mode lane docs 同步成 honest parked/current-status wording
- [x] 保持這批只收 tracked execution 狀態同步，不混入 runtime tooling 或產品線變更

## Review

- 這批變更沒有新增功能，目的是把舊的 lane-plan 文案從早期的 `Active lane item` / initial-active-set 敘述，收斂成現在 envelope/reducer 投影後的 honest `Current Lane Status`。
- 內容上主要是把已經 no-op、已經完成、或已經 parked 的 lanes 寫成目前真實狀態，避免未來再讀 tracked docs 時以為它們還有未完成的 active item。
- 這次刻意不碰 `nlsdd-refresh-scoreboard.cjs` 與產品線 WIP，避免把純 docs sync 和其他未驗證中的改動混在同一顆 commit。

# 2026-03-21 plot-mode phase-1 scaffold and handoff

- [x] 新增 TypeScript plot snapshot contract 與 barrel export
- [x] 在 root command 加入 `plot` mode、snapshot temp file 輸出與 Rust viewer handoff
- [x] 新增 Rust `plot-viewer` scaffold，包含 model / app / input / render 模組
- [x] 新增 plot-mode 專屬 regression tests 與 README 說明
- [x] 驗證 Node build、Rust compile、plot tests 與 cargo-backed viewer build script

## Review

- 這批目前是「Phase 1 scaffold + handoff」而不是完整 chart/panel product。Node 仍是 auth/cache/API 的 source of truth，Rust 目前先承接 plot viewer runtime 與 render scaffold。
- `src/commands/root.ts` 已能在 mode cycle 中切到 `plot`，建立 snapshot 並嘗試啟動 Rust viewer；若 viewer binary 尚未可用，會保留 snapshot path 並回退到 `delta`，不會把互動 flow 弄壞。
- `src/lib/plot/plot-snapshot.ts` 與 Rust `model.rs` 已建立跨 runtime 合約，對應測試也鎖住 TypeScript/Rust schema 與 package scripts。
- 目前 Rust render 還是 scaffold/placeholder，但這次已提供可編譯、可啟動、可持續擴充的骨架，並用 README 清楚標示它仍是 developer-facing scaffolding。
- 驗證：
  - `npm run build`
  - `node --test tests/plot-handoff.test.js tests/plot-mode-shell.test.js tests/plot-readme.test.js tests/plot-rust-model-contract.test.js tests/plot-snapshot.test.js tests/plot-viewer-scaffold.test.js`
  - `cargo check --manifest-path rust/plot-viewer/Cargo.toml`
- `npm run plot:viewer:build`

# 2026-03-21 NLSDD review findings remediation

- [x] 修正 reducer 在 replay 時錯讀 canonical root，避免 linked worktree / recovery branch 吃到錯的 lane plan
- [x] 修正 parked / noop / resolved-blocker transition 無法清掉舊 `Current item` / `Next refill target` 的問題
- [x] 將 `review` / `schedule` / `dispatch-plan` 等讀取入口改成不會順手重寫 tracked scoreboard / lane plan
- [x] 補 regression tests，鎖住 root replay、state clearing、read-only helper 三條行為
- [x] 跑 NLSDD automation tests 與 helper smoke checks 驗證修正

## Review

- 這批 remediation 已收斂完成，不再只是 review findings 清單。reducer replay 現在會吃 execution `projectRoot`，而不是偷回 `resolveProjectRoot()`；linked worktree replay 也有 regression coverage。
- `parked / noop-satisfied / resolved-blocker` 會顯式清掉 stale projected fields，空白 scoreboard cells 也會視為無值，避免舊 `Current item` / `Next refill target` 長回來。
- `review / schedule / dispatch-plan / cycle / launch / autopilot / intake / refill` 等讀取入口現在都會先 reduce canonical envelope state，再以 observational/read-only 模式讀 projection，不再把單純觀察變成 tracked mutation。
- 驗證：
  - `node --test tests/nlsdd-automation.test.js`
  - `node NLSDD/scripts/nlsdd-build-dispatch-plan.cjs --execution nlsdd-self-hosting --dry-run --json`
  - `node NLSDD/scripts/nlsdd-suggest-schedule.cjs --execution nlsdd-self-hosting --json`
  - `git diff --check`

# 2026-03-21 execution-insights journal remediation

- [x] 收斂 execution-insights lifecycle，避免已解決 insight 仍長期停在 adopted/open
- [x] 補 supersession / resolution 規則，讓較新的 resolved insight 能正確覆蓋舊的 adopted/open insight
- [x] 區分 execution-local actionable insights 與可升級成 tracked lesson/spec 的全域 learnings
- [x] 調整 `nlsdd-summarize-insights`，讓 coordinator 可直接看出哪些 insight 仍需規劃，哪些只是歷史/長期 learnings
- [x] 補 regression tests，鎖住 duplicate/resolved insight replay 與 summary 行為
- [x] 跑 NLSDD insight/automation 驗證

## Review

- 這輪把 insight summary 正式拆成三層：actionable execution-local insights、durable global learnings、resolved history。`dispatch-plan` / `review` / `autopilot` 因此不再把全域 adopted learning 誤當成本輪待辦。
- `plot-mode` 原本 lingering 的兩筆 global adopted learnings 已確認都已 graduate 到 tracked NLSDD guardrails / lessons，因此 runtime copies 已補寫 resolved event；現在 summary 顯示 `actionable 0 / durable 0 / resolved 14`。
- `nlsdd-summarize-insights` 也補了 grouped output，coordinator 現在能直接分辨哪些 runtime learnings 還需要 lane planning，哪些只是 durable policy，哪些已經只是歷史。
- 驗證：
  - `node --test tests/nlsdd-automation.test.js`
  - `node NLSDD/scripts/nlsdd-summarize-insights.cjs --execution plot-mode`
  - `node NLSDD/scripts/nlsdd-drive-review-loop.cjs --execution plot-mode`
  - `node NLSDD/scripts/nlsdd-build-dispatch-plan.cjs --execution plot-mode --dry-run --json`

# 2026-03-21 nlsdd-go remediation round integration

- [x] 將 Lane 4 的 stale-field clearing regression commit `655aa39` 整合回主線
- [x] 修正 reducer replay 仍使用錯誤 project root 的剩餘 root cause
- [x] 修正 parked / noop / resolved-blocker 仍會讓 stale projected fields 長回來的剩餘 root cause
- [x] 補上空字串 scoreboard cell 應視為無值、可回退到 lane-plan fallback item 的邊界
- [x] 驗證 `tests/nlsdd-automation.test.js` 與 `nlsdd-self-hosting` dispatch-plan 都回到綠燈

## Review

- 這輪 `nlsdd-go` 的真實產出只有一條 code lane：Lane 4 的 regression test；Lane 2、Lane 3、Lane 7 都經 honest probe 收回 `parked`，沒有硬做虛假的 active work。
- 當 `655aa39` 整合回主線後，`tests/nlsdd-automation.test.js` 立刻揭露：`NLSDD/scripts/nlsdd-envelope.cjs` 仍有兩個未收乾淨的 root cause，分別是 reducer replay 還會偷回 `resolveProjectRoot()`，以及 `parked/noop/resolved-blocker` 仍會讓舊 `currentItem/nextRefillTarget` 被投影長回來。
- 這次不是再加 workaround，而是直接在 reducer 裡修根因：讓 replay 全程吃 execution `projectRoot`、對 clearing transitions 明確寫 null、並把空字串 scoreboard cell 視為無值，避免 fallback 鏈被空字串截斷。
- 驗證結果現在是：
  - `node --test tests/nlsdd-automation.test.js`
  - `node NLSDD/scripts/nlsdd-build-dispatch-plan.cjs --execution nlsdd-self-hosting --dry-run --json`
  - `node NLSDD/scripts/nlsdd-suggest-schedule.cjs --execution nlsdd-self-hosting --json`
  - `git diff --check`
