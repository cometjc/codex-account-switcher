# 歷史待辦歸檔

此檔為原 `tasks/todo.md` 主體之搬移；**進行中項目**請寫在 [todo.md](todo.md)。

---

# 2026-03-28 startup cache-only loading and refresh tasks panel

- [x] 釐清 app 啟動變慢的根因，將啟動路徑改成只讀 cache / stale cache，不在 UI 載入前同步打 usage API
- [x] 先補 Rust regression tests，鎖住啟動只吃 cache、僅對超過 10 分鐘未更新的 profiles 排入背景 refresh
- [x] 實作背景 refresh task 狀態，並把 cron/status 與背景更新訊息移到左側 Details 下方的新 `Refresh tasks` 區塊
- [x] 執行 `cargo test --manifest-path Cargo.toml` 驗證

## Review

- 根因初查已確認：`App::load()` 啟動時會走 `load_profiles(..., force_refresh=false)`，但 `UsageService::read_usage()` / `read_codex_usage()` 在 cache 缺失或 TTL（目前 300 秒）過期時，仍會同步打 API，所以 profile 一多、或 cache 超過 5 分鐘，啟動就會明顯變慢。
- 這次目標是把啟動策略切成三層 truth：
  - app 啟動與一般 reload 預設只讀 cache / stale cache，不阻塞 UI
  - 真正的同步更新只留給手動 refresh 與 cron
  - 若 profile 超過 10 分鐘未更新，啟動後再以背景 task 方式補刷新，完成後更新畫面與 task 區塊
- 已落地：
  - `loader.rs` 現在支援 `cache_only` 載入模式；一般 app 啟動與非手動 refresh reload 只讀 cache/stale cache，不再因 TTL 過期直接同步打 API。
  - `usage.rs` 修正了舊行為中 `cache_only=true` 卻把過期 cache 標成 `stale=false` 的 bug，現在 stale cache 會誠實標記。
  - `app.rs` 新增背景 refresh worker：若 profile 超過 10 分鐘未更新，會在 UI 啟動後背景逐筆 refresh，完成後用 `Background refresh updated N profiles` 回寫狀態。
  - 原本底部 global cron/status line 已移除；左側 `Details` 下方新增 `Refresh tasks` 區塊，承接背景 refresh 狀態、背景完成訊息與 cron 錯誤摘要。
- 驗證：`cargo test --manifest-path Cargo.toml` 全綠（55 + 1 + 3 tests）。

# 2026-03-26 Claude history alias merge

- [x] 釐清 Claude 歷史 key 漂移的整合點，避免每次 reauth 都開新 bucket
- [x] 先補 regression test，鎖住 current Claude profile 會把舊 pseudo-id history 併到新 key
- [x] 在 usage/history 層新增 merge helper，並接回 Claude current-first / fallback matching 流程
- [x] 執行 `cargo test --manifest-path Cargo.toml` 驗證

## Review

- 根因是 Claude history 目前以 `refreshToken` 派生出的 pseudo `account_id` 當 key；只要 full reauth 讓 refresh token 換掉，之後的 snapshots 就會寫進新 key，舊歷史不會消失，但會碎成多個 bucket。
- 這次在 `UsageService` 新增 `merge_profile_history_aliases()`，將 alias key 的 weekly/5h windows 併進 canonical key，按 window 邊界合併 observation、去重排序後，移除舊 alias key。
- `loader.rs` 的 Claude current-first 路徑現在在兩個時機會自動 merge：
  - current name / pseudo-id 命中 saved profile 時
  - `subscription_type` 唯一匹配 fallback 將 current 對回某個 saved profile 時
- 新增 regression：
  - `merge_profile_history_aliases_moves_alias_history_into_canonical_key`
  - `claude_saved_profile_merges_old_history_into_current_key`
- 驗證：`cargo test --manifest-path Cargo.toml` 全綠（53 + 1 + 3 tests）。

# 2026-03-26 usage history keeps full weekly observations

- [x] 釐清目前 usage history 被截短的真正邊界，分清楚 observation cap 與 window retention
- [x] 先補 Rust regression test，鎖住單一 7d window 的 observation 不再因 256 上限被裁掉
- [x] 將 observation cleanup 改成只依時間範圍保留，不再依筆數裁切
- [x] 執行 `cargo test --manifest-path Cargo.toml` 驗證

## Review

- 根因確認是 `trim_history_windows()` 對每個 window 的 `observations` 有硬編碼 `256` 筆上限，對 weekly window 來說只夠保留大約 42 小時的 10 分鐘採樣點，因此看起來像 7 天歷史一直累積不起來。
- 這次保留既有 window-level retention（weekly 3 windows、5h 34 windows），但移除 observation count cap；window 內的 observation 現在只依 `start_at..=end_at` 時間範圍清理。
- 新增 regression：
  - `trim_history_windows_keeps_full_week_of_observations`
  - `trim_history_windows_removes_observations_outside_window_range`
- 驗證：`cargo test --manifest-path Cargo.toml` 全綠（51 + 1 + 3 tests）。

# 2026-03-26 current-first saved profile reconcile for Codex and Claude

- [x] 將 Codex/Claude 的 saved refresh 流程改成 current-first：若 saved profile 對應 current，就先更新 current，再回寫 saved
- [x] 補上 current-name/account matching 規則，避免 reauth 後 current 被誤判成 unsaved duplicate
- [x] 讓 app 載入與 cron refresh 共用同一條 reconcile truth
- [x] 補 Rust regression tests，鎖住 Codex 與 Claude 的 current-first sync-back 行為
- [x] 執行 `cargo test --manifest-path Cargo.toml` 驗證

## Review

- `Codex` 現在在 app 載入 saved profile 與 `--refresh-all` cron 路徑都會先判斷：該 saved profile 是否等於 current（`account_id` 相同，或 `~/.codex/current` 名稱命中）。若是，就直接用 current `auth.json` 做 stale check / refresh / 401 retry，成功後再用 `AccountStore::update_account()` 覆蓋回 saved snapshot。
- `Claude` 也改成同一個 current-first 流程，但 matching 比 `Codex` 稍寬：優先吃 `current name`，再吃目前 repo 既有的 pseudo `account_id()`。這樣 full reauth 導致 refresh-token-derived id 改變時，仍能靠 current name 對回同一個 saved profile。
- `loader.rs` 也補了 current-name 對 saved 的 truth，避免 current reauth 後因 `account_id` 變動，被 UI 再額外顯示成一筆 `[UNSAVED]` duplicate profile。
- 新增 regression：
  - `codex_saved_profile_uses_current_snapshot_and_syncs_saved_file`
  - `claude_saved_profile_uses_current_snapshot_when_name_matches`
- 驗證：`cargo test --manifest-path Cargo.toml` 全綠（49 + 1 + 3 tests）。

# 2026-03-26 Codex usage 401 refresh recovery

- [x] 釐清 Codex `wham/usage` 401 的根因與本 repo 目前 token lifecycle 缺口
- [x] 先補 Rust 測試，鎖住 Codex usage `401` 會觸發 refresh + retry，並把新 token 寫回 auth snapshot
- [x] 實作 Codex token refresh / persisted auth 更新，避免 saved profile 因過期 token 永久 401
- [x] 執行 `cargo test --manifest-path Cargo.toml` 驗證

## Review

- 根因不是 `wham/usage` 端點本身改掉，而是本 repo 的 Codex 路徑一直只拿 snapshot 裡的 `tokens.access_token` 直接打 usage API，完全沒有跟上官方 Codex client 的 token lifecycle：`last_refresh` 超過約 8 天要先 refresh，且 `401` 要 refresh-and-retry。
- 實際檢查本機 `~/.codex/accounts/example-corp-profile.json` 可見 `last_refresh=2026-03-17...`，已超過 stale 門檻；這解釋了為什麼 saved `example-corp-profile` 會在 usage refresh 時先撞 `401 Unauthorized`。
- `src/usage.rs` 現在新增 Codex auth snapshot 解析、OAuth refresh、`last_refresh` 寫回、stale pre-refresh 與 `401` retry；`loader.rs` 與 `main.rs` 也改成對 Codex profile 傳入實際 auth snapshot path（current `auth.json` 或 saved account file），讓 rotated token 會寫回正確檔案，而不是只修到 current profile。
- 驗證：`cargo test --manifest-path Cargo.toml` 全綠（47 + 1 + 3 tests）。

# 2026-03-26 Claude 401 usage refresh bug

- [x] 釐清 Claude usage 401 與目前 auto-refresh 條件的落差
- [x] 先補測試鎖住 401 應觸發 token refresh 的行為
- [x] 修正 Claude usage refresh 條件與必要文件說明
- [x] 驗證相關 Rust 測試通過

## Review

- 根因是 `fetch_claude_usage_with_auto_refresh()` 原本只把 `429` 視為可透過 refresh 解決的 usage 錯誤；`401 Unauthorized` 會直接往外丟，所以 status line 會顯示 `Claude issue: Claude usage request failed: HTTP status client error (401 ...)`。
- 已先補 regression test，確認 `401` 會在修正前穩定失敗；修正後 `401` 與 `429` 都會觸發一次 Claude OAuth token refresh，再用旋轉後的 token 重試一次 usage request。
- `cargo test --manifest-path Cargo.toml` 全綠（43 + 1 + 3 tests），README 的 Claude live verification 說明也已同步更新為 `429` / `401`。

# 2026-03-26 common-dev-rules adoption workflow relocation

- [x] 確認 shared baseline adoption/sync 流程應從 `ai-rules/` 收斂回 `skills/new-rule/SKILL.md`
- [x] 更新 `common-dev-rules` 索引與 adapter，移除 `ai-rules/adoption-workflow.md` 的正式章節地位
- [x] 將這次使用者修正寫進 `tasks/lessons.md`

## Review

- `common-dev-rules` 已刪除 `ai-rules/adoption-workflow.md`，並把 shared-baseline push/sync/adoption workflow 收斂回 `skills/new-rule/SKILL.md`。
- `AGENTS.md`、`CLAUDE.md`、`.cursor/rules/project-rules.mdc` 與 `.github/copilot-instructions.md` 已同步移除舊章節索引，避免 `ai-rules/` 與 skill 形成雙重權威來源。
- `new-rule` skill 已明寫：shared-baseline adoption item 必須逐條進 ask-style 互動、提供至少四種採納選項並記錄使用者選擇。

# 2026-03-26 baseline adoption diff uses explicit two-ref form

- [x] 將 `new-rule` skill 的 baseline adoption diff 改為雙 ref 形式 `git diff adopted/<project> main -- . ':(exclude)skills/*'`
- [x] 將這次 diff 邊界與命令形式修正寫進 `tasks/lessons.md`

## Review

- `common-dev-rules/skills/new-rule/SKILL.md` 現在要求：已有 adopted baseline 時，用雙 ref diff 比較正式規則面，並預設排除 `skills/*`。
- 只有任務本身明確是更新 workflow skill 時，才把 `skills/` 納入 baseline adoption review 的比較集合。

# 2026-03-22 rust-chart-overlap-cleanup execution truth

- [x] 以 central executor 建立 `rust-chart-overlap-cleanup` execution，將 Rust chart/history、Node cleanup、tests migration 與 final docs 收口拆成 5 條 lanes
- [x] 確認 4-active worker 語意是「4 個真實 active workers」，不是「4 個靜態 lane 名稱」
- [x] 將 Lane 1 收斂成 Rust history/model boundary，並把 Lane 2/3/4/5 留在 executor truth 中等待接續
- [ ] Lane 2：重構 chart renderer 與 plot layout，支援全 profile 疊圖與 5h bounded subframe
- [ ] Lane 3：移除剩餘 Node product CLI/legacy auth，改成 Rust-first npm shim
- [ ] Lane 4：整合 docs/tests/tracking 的最終收口，等 Lane 2/3 回來後做 cherry-pick
- [ ] Lane 5：遷移 regression coverage away from Node CLI/root snapshot assumptions

## Review

- 這次不是只有「開了 5 條 lane」，而是 executor truth 真的要維持 4 個 active workers。Lane 1 已經完成，Lane 2/3/4/5 則是目前真正要推進的 active set。
- 目前可先收口的只有 tracking 層：`tasks/todo.md` 與 `tasks/lessons.md` 可以先把 4-active worker 的定義與 lane 邊界寫清楚；README 與 Node tests 則刻意保留給 Lane 3 / Lane 5 接手，避免現在就跟共享檔案打架。
- `README.md`、`tests/plot-readme.test.js`、`tests/plot-viewer-scaffold.test.js`、`tests/entrypoints.test.js` 這些面向都已經被盤點，但依賴 Lane 2 / Lane 3 / Lane 5 的實際結果，暫不在 Lane 4 這次先改，以免 cherry-pick 時發生反向覆蓋。
- Lane 3 的 cleanup 本身目前看起來自洽：`package.json` 已切到 `bin/codex-auth.cjs` 薄 shim，README 已改成 Rust-first / thin shim，Node product CLI 與 legacy auth entrypoints 也都已移除。
- 目前不需要對 Lane 3 做 correction；真正要注意的是後續 cherry-pick Lane 2 / Lane 3 / Lane 5 時，`README.md`、`package.json`、`tests/entrypoints.test.js`、`tests/plot-readme.test.js`、`tests/plot-viewer-scaffold.test.js`、`tests/plot-mode-shell.test.js`、`tests/plot-rust-model-contract.test.js`、`tests/plot-snapshot.test.js` 的真相順序必須一致。

# 2026-03-22 Rust codex-auth 全面接手 auth 並整合 plot

- [x] 用 central executor 建立 `rust-auth-migration` execution，將剩餘 Rust 遷移 scope 一次拆成 Lane 1-4
- [x] Lane 1：將 Rust crate 提升為正式 `codex-auth` library+binary，補 paths、account store、usage service、binary entrypoint
- [x] Lane 2：在同一個 Rust app 內補 unified auth manager state 與 TUI action flow
- [x] Lane 3：把 plot 整合成同 app view，不再依賴外部 snapshot handoff 當正式 runtime truth
- [x] Lane 4：將 README、package scripts、Node regression tests 收斂到 Rust-first truth
- [x] 跑 Rust 與 Node 驗證，確認新 Rust-first surface 可編譯、測試可通過

## Review

- 這輪不是只在本地把 Rust 程式碼補一補，而是先照使用者要求，把剩餘 scope 全部寫進 central executor：新增 `rust-auth-migration` execution、單一 pending plan、4 條 lanes，並讓 Lane 1 綁目前主工作樹、Lane 2-4 各自有 queue worktree path。
- Lane 1 已實際落地新的 Rust domain/runtime：crate 已上移到 repo 根目錄（`agent-switch`），新增 `lib.rs`、`paths.rs`、`store.rs`、`usage.rs`，`main.rs` 直接啟動 Rust app，不再吃 snapshot path 才能跑。
- Lane 2 / Lane 3 的 visible runtime 也一起落地：`app.rs` 現在承接同一個 Rust TUI 內的 account list、save/rename/delete/refresh dialog、以及 built-in plot view；plot 不再是外部 viewer 的正式 truth，而是同 app 內的 view state。
- Lane 4 同步把 repo truth 切到 Rust-first：README 改成 Rust runtime + built-in plot 說法，`package.json` 的 build / plot scripts 指向 Rust `codex-auth` binary，對應 Node regression tests 也改成守護新的 Cargo metadata 與 README/entrypoint truth。
- 驗證：
  - `cargo test --manifest-path Cargo.toml`
  - `cargo check --manifest-path Cargo.toml`
  - `node --test tests/plot-mode-shell.test.js tests/plot-readme.test.js tests/plot-viewer-scaffold.test.js tests/plot-rust-model-contract.test.js`
  - `npm run build`
  - `git diff --check`


# parallel-lane-dev（executor 已遷出）

並行 subagent / executor 與其自動化測試、規格資產已移至 **`/home/jethro/repo/parallel-lane-dev`**。本 repo 不再內建該工具目錄；流程與口令見該倉庫 Open Plugin 技能 **`parallel-lane-dev`**。


- [x] 確認 `Usage Left`、`Time to reset`、`Drift` 目前被渲染成每列 label 的根因
- [x] 先補回歸測試，鎖住三個欄位名只出現在 header
- [x] 抽出並接回表格排版 helper，讓 row 只顯示值不重複印欄位名
- [x] 執行建置與測試驗證，補上 review 記錄

## Review

- 根因：[`src/commands/root.ts`](/path/to/workspace/agent-switch/src/commands/root.ts) 原本把 `Time to reset`、`Usage Left`、`Drift` 直接寫在每列 `W:` / `5H:` 詳細行裡，導致欄位名變成 row 內容而不是 header。
- 修正：新增 [`src/lib/root-table-layout.ts`](/path/to/workspace/agent-switch/src/lib/root-table-layout.ts) 集中處理 header 與 window detail line，讓 header 擁有欄位名，row 只渲染實際值。
- 接線：[`src/commands/root.ts`](/path/to/workspace/agent-switch/src/commands/root.ts) 改為呼叫 layout helper，而不是手寫帶 label 的詳細列字串。
- 回歸測試：新增 [`tests/root-table-layout.test.js`](/path/to/workspace/agent-switch/tests/root-table-layout.test.js) 驗證三個欄位名只出現在 header，不出現在 window row。
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


- [x] 補齊 schedule helper 與相關共用函式，讓 `queued` / `refill-ready` / active thread usage 可被一致推導
- [x] 將 scoreboard 與 execution docs 對齊新的 scheduling model，能顯示 queued lanes 與下一個 dispatch 建議

## Review

- 驗證：
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

- 新增 4 份 lane plan：
- [`plan/2026-03-20-multi-phase-routing-plan.md`](/path/to/workspace/agent-switch/plan/2026-03-20-multi-phase-routing-plan.md) 也同步記錄這次 lane-based orchestration 決策。


- [x] 明確規範 per-lane worktree、per-item commit、commit-diff review、coordinator-owned tracking
- [x] 將 Lane 4 對 Lane 2 render boundary 的依賴寫回 lane plan，避免再用執行中 scope expansion 硬撐
- [x] 新增 repo 內 scoreboard、communication 通道文件與 autopilot / probe 規則
- [x] 將 Rust `target/` 噪音治理拆成 `.gitignore` + lane-local 去追蹤清理

## Review

- `.gitignore` 現在忽略 `target/`，後續再配合 lane-local cleanup 去掉已進索引的 artifacts。


- [x] 為 Lane 1-4 建立 lane-local worktrees，避免 reviewer 被 shared dirty worktree 汙染
- [x] 讓 Lane 1 完成 real Rust viewer handoff 驗證，並走完 spec / quality review
- [x] 讓 Lane 2 完成 panel boundary seam 擴張與 drawable layout 修正，並走完 spec / quality review
- [x] 讓 Lane 3 完成 `chart.rs` 的 local view-model 抽離，並走完 spec / quality review
- [x] 讓 Lane 4 消費 Lane 2 seam，做出第一個可見 Summary / Compare panel surface，並走完 spec / quality review
- [x] 由 coordinator 回寫 lane plans、overview plan 與主線 progress

## Review

- Lane 1 在 [`.worktrees/lane-1-node`](/path/to/workspace/agent-switch/.worktrees/lane-1-node) 完成 real-binary handoff test，修正後綁回 `RootCommand.prototype.launchPlotViewer`，不再只是旁路測試。
- Lane 2 在 [`.worktrees/lane-2-runtime`](/path/to/workspace/agent-switch/.worktrees/lane-2-runtime) 先做 panel boundary expansion，再補一次 correction，把 `PanelRenderContext` 分清楚 outer `area` 與實際 drawable `content_area/layout`。
- Lane 3 在 [`.worktrees/lane-3-chart`](/path/to/workspace/agent-switch/.worktrees/lane-3-chart) 將 `chart.rs` 抽成私有 `ChartViewModel`，把 focused profile、7d bounds、5h band availability 的推導邏輯留在 chart lane 內。
- Lane 4 在 [`.worktrees/lane-4-panels`](/path/to/workspace/agent-switch/.worktrees/lane-4-panels) 消費 Lane 2 seam，補上 `Selected / Current / Focus / Pairing` 這類真正可見的 Summary / Compare body 內容。
- 驗證：
  - `node --test tests/plot-handoff.test.js` in `.worktrees/lane-1-node`
  - `cargo test --manifest-path Cargo.toml` in `.worktrees/lane-2-runtime`
  - `cargo check --manifest-path Cargo.toml` in `.worktrees/lane-2-runtime`
  - `cargo test --manifest-path Cargo.toml` in `.worktrees/lane-3-chart`
  - `cargo test --manifest-path Cargo.toml` in `.worktrees/lane-4-panels`


- [x] 加入 `autopilot refill`、`lane status probe`、`coordinator sidecar` 溝通規則
- [x] 以 `.gitignore` 管理未追蹤的 Rust `target/` 輸出
- [x] 在含有 tracked artifacts 的 lane worktree 上做 lane-local 去追蹤清理

## Review

- 噪音治理現在明確分成兩層：`.gitignore` 處理未追蹤 `target/`，lane worktree 再各自清掉已進索引的 `target/**`。


- [x] 新增 scoreboard refresh script，從 lane worktrees 自動回填 `Branch HEAD`、`Last probe`、`Noise`
- [x] 從 `~/.codex/state_5.sqlite` 補 recent Codex threads 附錄，作為 agent activity sidecar
- [x] 將 scoreboard 調整成手動欄位與自動欄位並存
- [x] 補 npm script 接線並完成實際 refresh 驗證

## Review

- scoreboard 現在保留 `Current item`、`Phase`、`Item commit`、`Blocked by`、`Next refill target`、`Notes` 這些人工決策欄位，同時自動更新 `Branch HEAD`、`Last probe`、`Noise`。
- 自動附錄的 `Recent Codex Threads` 會列出最近在這個 repo cwd 活動的 subagent nickname、role、thread id 與 updated time，讓 coordinator 不用手動去 `.codex` 查。
- 驗證：


- [x] 升級 scoreboard refresh，從 `sessions/*.jsonl` 推導 lane-level `Effective phase`、`Latest event`、`Correction count`、`Last activity`
- [x] 新增 lane probe helper，將 source diff、artifact noise 與 lane-local verification 集中輸出
- [x] 新增 refill assistant，為 `refill-ready` lane 建議下一個 lane-local item
- [x] 新增 sidecar message helper，產生 implementer / reviewer / correction 模板草稿

## Review

- `autopilot refill` 在 v2 仍維持 assistive 模式：腳本只建議下一個 item，不直接派工或改 checklist。


- [x] 補上 scheduler helper，讓 coordinator 能看到 active thread 使用量、可用 slot 與下一批 dispatch 建議
- [x] 更新 scoreboard / overview 文件，明確區分 queued / parked lane 與 active lane

## Review



- [x] 補 linked worktree 測試，鎖住 worktree-local scoreboard / lane plan 會優先被讀取

## Review

- 根因是 `resolveProjectRoot()` 先用 git common-dir 把 linked worktree 收斂回主 repo root，導致 recovery branch 內的 manual scoreboard 與 execution docs 即使不同，也會被 `refresh` / `schedule` / `refill` 忽略。


- [x] 找出為什麼 recovery branch 讀到自己的 lane docs 後，`.worktrees/...` 仍會被誤判成 `missing-worktree`
- [x] 將 lane plan 的 worktree 路徑解析改成使用 canonical worktree pool root，而不是 execution root
- [x] 補 linked worktree 測試，鎖住「讀當前 branch 的 lane docs，但 worktree path 仍指向共用 repo `.worktrees/`」的行為

## Review

- 修正後新增 `resolveWorktreePoolRoot()`，讓 execution docs 仍取自當前 worktree，但 `.worktrees/...` 會回到 git common-dir 對應的 canonical repo root 解析。
- 這讓 linked worktree / recovery branch 具備正確的雙 root 模型：`execution root` 負責文件與 scoreboard，`worktree pool root` 負責 lane worktree 實體位置。


- [x] 找出為什麼 4a execution 仍可能在實務上收束成單一 critical lane，讓其他 slot 只是在等待

## Review

- coordinator 遇到這種情況時，優先順序改成：
  - 切出新的獨立 lane
  - 或改成同時推進 2-3 個 plans/executions
  - 只有在沒有 truthful parallel work 時，才接受暫時降到低併行度。
- 這條 guardrail 直接來自本輪 `plot-mode` 觀察：Lane 4 一度成為唯一有真實前進的 lane，而 Lane 1/3/5 多次回到 `NOOP` 或 clean probe。


- [x] 在 scheduler / probe 邏輯中辨識「lane journal 還是 implementing，但 worktree clean 且 HEAD == latestCommit」的 stale 狀態
- [x] 讓 stale lane 不再被算入 active thread usage
- [x] 補 regression test，鎖住 stale lane 會進入 `staleRows` 而不是 `activeRows`

## Review



- [x] 將 deterministic coordinator work 收斂成單一 cycle helper
- [x] 讓 cycle helper 一次完成 stale lane reconcile、runtime refresh 與下一批 lane promotion
- [x] 讓 cycle helper 回傳 completed lanes、promoted lanes 與 idle slots
- [x] 補 regression test，鎖住 cycle 會先收 stale lane 再 promote 下一條 queued lane

## Review

- cycle helper 現在會先把可由 tracked phase 解決的 `stale-implementing` lane 收回 truthful phase，再按既有 plan/journal 自動 promote 下一批 dispatchable lanes。
- 輸出會明確列出：
  - `Reconciled lanes`
  - `Promoted lanes`
  - `Idle slots`
  - `No dispatch reason`


- [x] 在 cycle helper 之上新增一層 launch bridge，讓單次指令除了 reconcile/promote，也能回傳 coordinator-ready assignment bundles
- [x] 讓 assignment bundle 直接吃 lane plan 的 ownership family、verification commands 與 next actionable item
- [x] 補 regression test，鎖住 promoted lane 會產出 implementer-assignment 文案與對應 scope/verification

## Review



- [x] 新增 review helper，根據 lane 當前 phase 自動整理下一步 coordinator action
- [x] 支援 `spec-review-pending`、`quality-review-pending`、`correction` 與 `coordinator-commit-pending`
- [x] 補 regression test，鎖住 review helper 會產出正確的 review / correction bundle

## Review

- helper 目前會優先吃 lane journal phase，再回退到 runtime/tracked scoreboard phase，並重用既有的 `spec-review`、`quality-review`、`correction-loop` 模板；若 lane 已進 `coordinator-commit-pending`，則輸出 commit intake bundle。


- [x] 將 commit-ready handoff 的 title/body/verification 結構化寫進 lane journal
- [x] 新增 intake helper，讓 coordinator 可直接收 `coordinator-commit-pending` lanes 的 commit bundle
- [x] 補 regression test，鎖住 intake helper 會回傳 proposed commit title/body、scope、verification 與 note

## Review



- [x] 將 launch / review / intake 三條 coordinator 助手收成單一 autopilot loop
- [x] 讓單次指令回傳 promoted lanes、review actions、commit intake 與 idle slots
- [x] 補 regression test，鎖住 autopilot 會同時看見 dispatch、review 與 READY_TO_COMMIT 收單

## Review

- autopilot 目前是 assistive automation：它會執行 cycle/launch 的狀態收斂，並彙整 review/intake 結果，但不直接代 main agent 呼叫 subagent 工具。


- [x] 在 autopilot 之上新增 dispatch-plan helper，輸出 main agent 可直接照單執行的 action queue
- [x] 將 queue 預設優先順序定成 commit intake -> review/correction -> launch assignment
- [x] 補 regression test，鎖住 dispatch plan 會依優先順序輸出 queue

## Review



- [x] 將 `plot-mode Lane 2` 改成 parked，停止從已滿足的 compare seam wording 重派 runtime lane
- [x] 將 `plot-mode Lane 4` 改寫為 adopted-target emphasis 的 honest next item

## Review

- 根因不是 scheduler 算錯，而是 tracked lane docs 還保留舊的 current/refill wording，導致 autopilot 會把已被 `noop-satisfied` 的 lane 再次 promote 回 implementing。
- 這次先把 tracked planning surface 對齊 runtime truth：已完成的 item 直接打勾，只留下真正的下一個 reviewable item。這樣 `dispatch-plan`、人工 review 與 execution insights 看到的 lane intent 才會一致。


- [x] 新增 canonical `lane handoff envelope` 與 `events.ndjson` event log
- [x] 新增 reducer，從 envelope 投影 lane journal、execution insights、tracked scoreboard 與 lane plan `Current Lane Status`
- [x] 將 message helper 改成要求 subagent 回傳 strict envelope JSON
- [x] 補 automation tests，鎖住 envelope -> projection 鏈路與 active-set/review 既有行為

## Review

- 根因不是單一 script 出錯，而是 `record-lane-state`、`record-insight`、tracked scoreboard、lane plan status 都能各自落筆，導致同一個 execution 同時存在多套可寫 state surface。
- 目前 `launch/review/intake/autopilot` 還是讀既有 projection surfaces，但因為這些 surfaces 已改成同一條 reducer 產物，狀態差異已大幅收斂；後續可以再把讀取端逐步改成直接吃 reducer snapshot。

# 2026-03-21 plot-mode Lane 4 stale runtime state cleanup

- [x] 檢查 `plot-mode Lane 4` 為何在 envelope/reducer 上線後仍殘留 `stale-implementing`
- [x] 將 tracked lane plan / scoreboard 收斂成 honest parked state，不再把已完成的 adopted-target emphasis 當成 current implementing item
- [x] 以 canonical envelope 寫入 Lane 4 的 parked transition，重建 reducer 投影
- [x] 驗證 `dispatch-plan` 不再把 Lane 4 列為 stale implementing

## Review

- 根因不是 reducer 壞掉，而是 `plot-mode Lane 4` 的歷史 bootstrapped event 仍把 `6bb1fba` 投影成 `implementing`，而 tracked lane plan / scoreboard 也還停留在舊 wording，導致 reducer 每次重播都忠實重建同一個假 active state。
- 這次先把 tracked planning surface 收斂成 honest truth：`6bb1fba` 已完成 adopted-target emphasis，所以 Lane 4 的 current item 改成等待下一個 fresh panel-local item，phase 改為 `parked`。


- [x] 確認 `6d6c4e8` 的實際 diff 是否已完成 Lane 4 的 regression/CLI cross-check item
- [x] 以 canonical envelope 寫入 Lane 4 的 parked transition，重建 reducer 投影
- [x] 將 Lane 4 舊的 adopted issues 收斂成 resolved，避免 insight summary 繼續重播已解決問題
- [x] 驗證 `dispatch-plan` / `schedule` / `insight summary` 不再把 Lane 4 列成 actionable stale work

## Review

- 根因和 `plot-mode Lane 4` 很像：`6d6c4e8` 其實已經完成 CLI smoke 與 scoreboard/schedule cross-check coverage，但 tracked wording、lane journal、insight lifecycle 都還停在 correction 當下，所以 reducer 忠實地把它重播成 `stale-implementing`。
- 這次不再試圖替 Lane 4 硬找下一個 item，而是先把 truth 收乾淨：cross-check coverage 已落地，Lane 4 回到 `parked`，之後只在真的出現新 regression/CLI surface 缺口時再重開。
- 也把兩條舊的 adopted insight 收成 resolved，避免 `review/autopilot/dispatch-plan` 還把它們當成目前的 actionable 問題。
- 接著再用 canonical envelope 寫入同一個 parked transition，讓 runtime journal、tracked scoreboard、lane plan `Current Lane Status` 與後續 `dispatch-plan` 全部由同一條事件鏈收斂到一致狀態。


- [x] 確認 `e853688` 已完成 Lane 7 當前 scheduler/runtime truth audit item
- [x] 以 canonical envelope 寫入 Lane 7 的 parked transition，避免 quality pass 後又把同一 item 重派成 refill-ready
- [x] 驗證 `dispatch-plan` / `schedule` 不再把 Lane 7 當成可立即補位的假 refill

## Review

- 根因不是 Lane 7 還沒做完，而是 `quality pass -> refill-ready` 的通用 phase 在沒有 fresh next item 時，會把同一個已完成 item 又推出來一次。
- `e853688` 已經完成這輪 scheduler/runtime truth audit 的具體 delta：移除過度保守的 queued-only anti-convergence warning，並保留真正需要人工介入的警示條件。
- 因此這次和 Lane 4 一樣，先把 honest truth 收乾淨：Lane 7 回到 `parked`，之後只在真的出現新的 scheduler/runtime truth finding 時再重開。


- [x] 將 self-hosting overview 的 Lane 2 / 3 / 4 / 7 狀態收斂成 parked truth
- [x] 將 Lane 7 已落地的 reducer / insight integrity checklist 標成完成

## Review

- runtime 已經先由 canonical envelope 收斂成全 execution idle；如果 overview 和 lane checklist 還停在早期的 active/remediation 語氣，之後讀 tracked docs 仍會以為 lane 還在進行中。


- [x] 將 remaining self-hosting lane docs 同步成 honest parked/current-status wording
- [x] 將 remaining plot-mode lane docs 同步成 honest parked/current-status wording
- [x] 保持這批只收 tracked execution 狀態同步，不混入 runtime tooling 或產品線變更

## Review

- 這批變更沒有新增功能，目的是把舊的 lane-plan 文案從早期的 `Active lane item` / initial-active-set 敘述，收斂成現在 envelope/reducer 投影後的 honest `Current Lane Status`。
- 內容上主要是把已經 no-op、已經完成、或已經 parked 的 lanes 寫成目前真實狀態，避免未來再讀 tracked docs 時以為它們還有未完成的 active item。

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
  - `cargo check --manifest-path Cargo.toml`
- `npm run plot:viewer:build`


- [x] 修正 reducer 在 replay 時錯讀 canonical root，避免 linked worktree / recovery branch 吃到錯的 lane plan
- [x] 修正 parked / noop / resolved-blocker transition 無法清掉舊 `Current item` / `Next refill target` 的問題
- [x] 將 `review` / `schedule` / `dispatch-plan` 等讀取入口改成不會順手重寫 tracked scoreboard / lane plan
- [x] 補 regression tests，鎖住 root replay、state clearing、read-only helper 三條行為

## Review

- 這批 remediation 已收斂完成，不再只是 review findings 清單。reducer replay 現在會吃 execution `projectRoot`，而不是偷回 `resolveProjectRoot()`；linked worktree replay 也有 regression coverage。
- `parked / noop-satisfied / resolved-blocker` 會顯式清掉 stale projected fields，空白 scoreboard cells 也會視為無值，避免舊 `Current item` / `Next refill target` 長回來。
- `review / schedule / dispatch-plan / cycle / launch / autopilot / intake / refill` 等讀取入口現在都會先 reduce canonical envelope state，再以 observational/read-only 模式讀 projection，不再把單純觀察變成 tracked mutation。
- 驗證：
  - `git diff --check`

# 2026-03-21 execution-insights journal remediation

- [x] 收斂 execution-insights lifecycle，避免已解決 insight 仍長期停在 adopted/open
- [x] 補 supersession / resolution 規則，讓較新的 resolved insight 能正確覆蓋舊的 adopted/open insight
- [x] 區分 execution-local actionable insights 與可升級成 tracked lesson/spec 的全域 learnings
- [x] 補 regression tests，鎖住 duplicate/resolved insight replay 與 summary 行為

## Review

- 這輪把 insight summary 正式拆成三層：actionable execution-local insights、durable global learnings、resolved history。`dispatch-plan` / `review` / `autopilot` 因此不再把全域 adopted learning 誤當成本輪待辦。
- 驗證：


- [x] 將 Lane 4 的 stale-field clearing regression commit `655aa39` 整合回主線
- [x] 修正 reducer replay 仍使用錯誤 project root 的剩餘 root cause
- [x] 修正 parked / noop / resolved-blocker 仍會讓 stale projected fields 長回來的剩餘 root cause
- [x] 補上空字串 scoreboard cell 應視為無值、可回退到 lane-plan fallback item 的邊界

## Review

- 這次不是再加 workaround，而是直接在 reducer 裡修根因：讓 replay 全程吃 execution `projectRoot`、對 clearing transitions 明確寫 null、並把空字串 scoreboard cell 視為無值，避免 fallback 鏈被空字串截斷。
- 驗證結果現在是：
  - `git diff --check`
