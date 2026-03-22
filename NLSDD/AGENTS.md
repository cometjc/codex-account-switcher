# NLSDD Directory Rules

- `NLSDD/scripts/nlsdd-executor.cjs` 與 `.nlsdd/executor.sqlite` 是目前唯一正式的 NLSDD execution 介面與 canonical state。
- 主控端與 subagent 都只能透過 executor CLI/API + worktree branch / result branch 交換資訊；不要再把 markdown、`events.ndjson`、`lane-*.json` 或 thread prose 當成正式狀態通道。
- `plan/` 不應保留 live plan。若 repo 內仍有 `plan/*.md`，先透過 executor import/cleanup，再開始 `nlsdd-go` 或派工。
- 同一種狀態只能有單一權威來源，且目前都在 executor SQLite：
  - plan / decomposition truth: `.nlsdd/executor.sqlite`
  - execution / lane / assignment truth: `.nlsdd/executor.sqlite`
  - result / review / integration truth: `.nlsdd/executor.sqlite`
- worktree branch 與 result branch 是唯一實體交換物：
  - lane assignment 由 executor 提供 worktree path、lane branch、base branch
  - subagent 完成後只回單一 status 與 result branch
  - review / intake / integration 全由主控端透過 executor 收口
- 舊的 `NLSDD/scoreboard.md`、`NLSDD/state/*`、`NLSDD/executions/*/*.md` 與 `nlsdd-*` helper 目前視為 legacy migration surfaces；除非正在做 importer / cleanup / backward-compat 維護，否則不要新增新的流程責任到它們上面。
- `nlsdd-go` 代表 main agent 要透過 executor 持續推進所有相關 plans，直到 executor 內的相關 plan 都完成；不是只做一次 truth scan，也不是只看某個 execution 暫時有沒有 dispatchable lane。
- subagent 正式回報應走 executor 的狀態介面，例如 `claim-assignment`、`report-result`；不要要求 main agent 再從自由文字手動推論 phase。
