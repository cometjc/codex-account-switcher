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
