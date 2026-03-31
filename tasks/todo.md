# 待辦（已清理）

此檔僅保留當前可追蹤項目；歷史細節已收斂為摘要。

## 本輪完成
- [x] chart label 改為外移 + 連接線，並避免覆蓋曲線 glyph
- [x] chart x 軸語意改為 reset-aligned（`x=0 = reset_time-7d`, `x=7 = reset_time`）
- [x] 修正 app reload 自激迴圈（忽略 refresh log / lock 事件）
- [x] 修正 profile 切換時 JSON EOF 風險（原子寫入）
- [x] 建立 SQLite 儲存層（schema/migration/WAL/busy_timeout/transaction）
- [x] usage cache/history 改走 DB，並加入 legacy JSON backfill
- [x] `UiState` / `CronStatus` 改為 DB 優先（保留短期 fallback）
- [x] 保留併發安全（history lock + upsert）並通過 app/cron 併發回歸
- [x] 導入 `tokio-rusqlite` async adapter 骨架（不改目前同步流程）

## review
- 驗證命令：`make test`
- 結果：全綠（72 + 3 + 4，含 install）

## 本輪進行中
- [x] 以 `RUSTFLAGS='-D warnings' make build` 重現 warning-as-error 失敗
- [x] 修正 `src/loader.rs` 未使用綁定
- [x] 讓 `Makefile` 預設將 warning 視為 error
- [x] 驗證 `make build` / `make test`
- [x] commit 本輪修正

## 本輪 review
- 驗證命令：`make build`、`make test`
- 結果：全綠（73 + 3 + 4，含 install；warnings-as-errors 已生效）
