# NLSDD Directory Rules

- 此處集中管理 `NLSDD` 的實際執行用流程文件、scoreboard、execution docs 與 helper scripts。
- `executions/<execution-id>/` 記錄 execution overview、lane plans、目前狀態與 refill 順序，供實際協作與回放使用。
- `state/<execution-id>/lane-<n>.json` 記錄 execution-aware 的 lane runtime state；`state/scoreboard.runtime.md` 承接 auto-refreshed scoreboard 輸出。這些都是 runtime artifacts，不應納入 tracked tree。
- `scoreboard.md` 是 repo 內唯一正式的 tracked lane 狀態板；lane phase、latest commit、blocked 狀態與 queued/active set 應優先在此維護，auto-derived 欄位則輸出到 runtime scoreboard。
- `scripts/` 放 `NLSDD` 執行輔助腳本；若路徑或輸出格式變更，需同步更新 `package.json` scripts、tests 與相關文件。
- `NLSDD/` 只放實際執行所需 artefacts；通用定義、規格與不依賴單次 execution 的治理文件應維護在 `spec/NLSDD/`。
- 只要一個 lane-local MVC step 已完成且驗證通過，就應預設立即產生 lane-item commit；不要把多個已完成 MVC step 疊在同一個未提交狀態裡。
- 若執行環境對 `git commit` 或其他 lane-finalizing 動作會跳 permission/confirmation prompt，subagent 不應默默卡住；應先回報 `READY_TO_COMMIT`、已完成的驗證與 commit-ready 摘要，交由 main agent/coordinator 執行 commit。
