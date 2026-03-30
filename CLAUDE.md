@AGENTS.md
@ai-rules/adoption-workflow.md
@ai-rules/shared-baseline-sync-and-local-adoption.md
@ai-rules/shared-rules-entry-and-thin-adapters.md
@ai-rules/commit-each-minimum-viable-change.md
@ai-rules/before-new-mvc-assess-existing-dirty-tree.md
@ai-rules/choose-agents-skill-script-or-runbook.md
@ai-rules/verify-third-party-module-interface-before-integration.md
@ai-rules/distinguish-rule-suggestions-from-established-process-state.md
@ai-rules/surface-stale-untracked-governance-files-at-stop.md

## Codex workspace 補充

### 開發流程

所有 plan 的實作流程固定為：

```
plan[1..n] -> split to lanes[1..m] -> 4a parallel-lane flow（`plugins/parallel-lane-dev/`、`npm run pld:*`）
```

- 收到實作任務時，先確認 plan 拆分完成
- 將 plan 切分為獨立 lanes
- 以 4a parallel-lane flow 執行各 lane（腳本 symlink 自 `agent-plugins` 套件；技能 `plugins/parallel-lane-dev/skills/parallel-lane-dev/SKILL.md`）
- 不可跳過 lane 拆分直接實作

### Build Rule

Always use `make test` as the default verification command for Rust changes.  
Use `cargo test` only for explicit quick local checks or when the user explicitly requests it.
