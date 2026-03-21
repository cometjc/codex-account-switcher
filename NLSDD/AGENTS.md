# NLSDD Directory Rules

- 此處集中管理 `NLSDD` 的實際執行用流程文件、scoreboard、execution docs 與 helper scripts。
- `executions/<execution-id>/` 記錄 execution overview、lane plans、目前狀態與 refill 順序，供實際協作與回放使用。
- `state/<execution-id>/lane-<n>.json` 記錄 execution-aware 的 lane runtime state；`state/scoreboard.runtime.md` 承接 auto-refreshed scoreboard 輸出。這些都是 runtime artifacts，不應納入 tracked tree。
- `state/<execution-id>/execution-insights.ndjson` 用來 append subagent 建議、main agent/coordinator 觀察到的問題，以及執行期間發現的可改善項目；它和 lane state 一樣屬於 runtime artifacts，不應納入 tracked tree。
- `scoreboard.md` 是 repo 內唯一正式的 tracked lane 狀態板；lane phase、latest commit、blocked 狀態與 queued/active set 應優先在此維護，auto-derived 欄位則輸出到 runtime scoreboard。
- `scripts/` 放 `NLSDD` 執行輔助腳本；若路徑或輸出格式變更，需同步更新 `package.json` scripts、tests 與相關文件。
- 若要記錄執行期 insight，優先使用 `NLSDD/scripts/nlsdd-record-insight.cjs`；不要把這類動態觀察只留在 thread history 裡。
- 若要重排某個 execution 的 active/parked lane set，優先使用 `NLSDD/scripts/nlsdd-replan-active-set.cjs`，不要只改 tracked scoreboard 而忘記同步 lane journal。
- 若只是要跑一輪低判斷成本的 lane 調度，優先使用 `NLSDD/scripts/nlsdd-run-cycle.cjs`；它應一次完成 stale lane 收尾、runtime refresh、下一批 lane promotion，並回傳完成/派送/閒置 slot 狀態。
- `NLSDD/` 只放實際執行所需 artefacts；通用定義、規格與不依賴單次 execution 的治理文件應維護在 `spec/NLSDD/`。
- 只要一個 lane-local MVC step 已完成且驗證通過，就應預設立即收斂成 lane-item commit；不要把多個已完成 MVC step 疊在同一個未提交狀態裡。
- 若某個 execution 開始收束成單一 critical lane，導致其他 slot 只是在等它完成，就應視為需要 replan 的訊號；優先切出新的獨立 lane，或改成同時推進 2-3 個 plans/executions，而不是維持假的 4-active 表象。
- commit 責任要依情境區分：
  - main agent 直接在本地工作時，可在驗證後直接 commit
  - NLSDD subagent 在 lane worktree 內工作時，預設不要自己跑 `git commit`
- NLSDD subagent 完成 lane-local MVC step 後，應回報 `READY_TO_COMMIT`、已完成的驗證與 commit-ready 摘要，交由 main agent/coordinator 執行 commit；只有 lane 明確標示 self-commit-safe 時才例外。
