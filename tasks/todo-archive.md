# 歷史待辦歸檔

**進行中**請只維護 [todo.md](todo.md)。本檔為已結案里程碑之**精簡索引**；長篇 Review、舊 Node/TS 產品線敘述已自本檔刪除以降低噪音，必要時請用 **`git log` / `git show`** 追溯舊版 `todo-archive.md`。

驗證慣例以** repo 根目錄** `cargo test` 為準（不再使用已移除的 `rust/plot-viewer` manifest-path）。

---

## 2026-03-28 — 啟動 cache-only 與 Refresh tasks 面板

- [x] 啟動只讀 cache / stale cache，避免 UI 載入前同步打 usage API  
- [x] 背景 refresh（逾 10 分鐘未更新）、`Refresh tasks` 區塊與 regression tests  

**結果摘要：** `loader` cache-only、`usage` stale 標記修正、`app` 背景 worker；底部 cron 列移除。

---

## 2026-03-26 — Claude / Codex / history / adoption

- [x] Claude history alias merge（`merge_profile_history_aliases`）  
- [x] Weekly usage history 移除 observation 筆數上限，改依時間窗清理  
- [x] Codex + Claude current-first saved refresh reconcile  
- [x] Codex usage 401 → refresh + retry 寫回 snapshot  
- [x] Claude usage 401 與 429 同層級觸發 OAuth refresh + 重試  
- [x] common-dev-rules adoption / baseline diff 流程收斂（見當時 `tasks/lessons.md`）  

---

## 2026-03-22 — rust-chart-overlap-cleanup（executor lanes）

- [x] 建立 execution、4-active worker 語意、Lane 1（history/model boundary）  
- [x] **Lane 2–5（原開放項）** — 已由後續 **Rust-first TUI、內建 plot、根目錄 `agent-switch` crate、`npm` 薄 shim、合約測試與 README 遷移**吸收並結案（疊圖與 5h subframe、移除 Node 產品 CLI、docs/tests 收口、regression 遠離舊 Node CLI 假設）。  

---

## 2026-03-22 — rust-auth-migration

- [x] Library + binary、`paths` / `store` / `usage`、TUI 內 plot、README + tests 收斂至 Rust-first  

---

## 2026-03-20 以前 — TypeScript CLI、prompt UI、plot-mode / NLSDD 協調

此時期 checklist 與長篇 Review 曾涵蓋：**Node `root` command、plot snapshot handoff、scoreboard / envelope / reducer、多 lane worktree**。該 **runtime 已移除**，敘述僅供考古；並行開發現用 **`plugins/parallel-lane-dev/`**（見 [CONTRIBUTING.md](../CONTRIBUTING.md)），舊「僅存 `~/repo/parallel-lane-dev`」說法已過時。

---

## 表格欄位重複（舊 TS `root.ts`）

- [x] `Usage Left` / `Time to reset` / `Drift` 等欄位名回到 header-only（舊測試與 layout helper 已隨 TS 刪除）  

---

## PLD / parallel-lane

- 本 repo：**`plugins/parallel-lane-dev/`**（`scripts` + `skills` symlink 至上游套件；**`.pld/executor.sqlite`**）。口令 **pld-go** → `npm run pld:executor:go`。
