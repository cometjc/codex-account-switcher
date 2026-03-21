# 2026-03-20 limits 欄位 header 修正

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
