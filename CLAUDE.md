@AGENTS.md
@ai-rules/adoption-workflow.md
@ai-rules/shared-baseline-sync-and-local-adoption.md
@ai-rules/commit-each-minimum-viable-change.md
@ai-rules/choose-agents-skill-script-or-runbook.md
@ai-rules/verify-third-party-module-interface-before-integration.md
@ai-rules/distinguish-rule-suggestions-from-established-process-state.md
@ai-rules/surface-stale-untracked-governance-files-at-stop.md

## Codex workspace 補充

### 開發流程

所有 plan 的實作流程固定為：

```
plan[1..n] -> split to lanes[1..m] -> 4a nlsdd flow
```

- 收到實作任務時，先確認 plan 拆分完成
- 將 plan 切分為獨立 lanes
- 以 4a nlsdd flow 執行各 lane
- 不可跳過 lane 拆分直接實作

### Build Rule

Always use `cargo test` instead of `cargo build` to verify Rust changes — it compiles and runs all tests in one step.
