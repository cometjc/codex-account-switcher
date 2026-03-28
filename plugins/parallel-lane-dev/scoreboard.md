# PLD Scoreboard (agent-switch)

> 手動維護欄位；自動衍生狀態見 `plugins/parallel-lane-dev/state/scoreboard.runtime.md`（執行 `npm run pld:scoreboard:refresh` 後產生）。
>
> Runtime journal：`plugins/parallel-lane-dev/state/<execution-id>/lane-<n>.json`（若存在則優先於 heuristics）。

| Execution | Lane | Ownership | Current item | Phase | Item commit | Last verification | Blocked by | Next refill target | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| agent-switch | Lane 1 | Product / Rust TUI | 依 `tasks/todo.md` 與當前迭代收斂 | parked | `n/a` | `npm test` | none | 下一個可派工項 | 新增 execution 列與 lane 文件後再改 phase |

## Recent Codex Threads

> 若本機無 Codex state DB，此區可留空；`pld:scoreboard:refresh` 會嘗試填入。

| Nickname | Role | Thread ID | Updated |
| --- | --- | --- | --- |
