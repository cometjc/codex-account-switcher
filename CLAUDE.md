# Project Rules

## 語言

所有回覆與思考過程使用**繁體中文（zh-TW）**。

## 開發流程

所有 plan 的實作流程固定為：

```
plan[1..n] -> split to lanes[1..m] -> 4a nlsdd flow
```

- 收到實作任務時，先確認 plan 拆分完成
- 將 plan 切分為獨立 lanes
- 以 4a nlsdd flow 執行各 lane
- 不可跳過 lane 拆分直接實作

## Build Rule

Always use `cargo test` instead of `cargo build` to verify Rust changes — it compiles and runs all tests in one step.
