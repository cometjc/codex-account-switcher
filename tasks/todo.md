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
