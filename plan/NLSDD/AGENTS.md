# NLSDD Directory Rules

- 此處集中管理 `NLSDD` 的通用協作規則、scoreboard、communication flow 與各 execution 實例。
- `operating-rules.md` 描述 repo 級通用規則；不要把單一 execution 的 lane 細節寫回這份文件。
- `executions/<execution-id>/` 只記錄該 execution 的 overview、lane plans、目前狀態與 refill 順序。
- `scoreboard.md` 是 repo 內唯一正式的 lane 狀態板；lane phase、latest commit、blocked 狀態應優先在這裡維護。
- reviewer / implementer / coordinator 的正式通道以 `communication.md` 為準，不要在各 execution 內各自發明流程。
- 若 `NLSDD` 規則更新會影響現有 execution，先更新通用文件，再同步套用到所有仍在活躍的 execution。
