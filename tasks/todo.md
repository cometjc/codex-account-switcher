# 待辦（進行中）

此檔保留**當前迭代**的核取清單與簡短備註。

- [x] 針對 app 常駐 + cron 併發寫入建立可重現測試
- [x] 驗證 history 不互蓋且 observation 持續累積
- [x] 驗證 chart 仍可顯示 rolling 7d 內點位
- [x] 執行 cargo test 並整理證據

## review
- 以並行 writer 測試重現「晚到舊時間戳寫入會回滾刪除新點」根因。
- 修正 prune 錨點為 `max(now, 現存最新 observation)`，避免 app/cron 交錯時互相回滾。
- 新增 chart 端併發寫入可見性測試，確認 rolling 7d 仍可投影出點位。
- `cargo test` 全綠（69 + 1 + 4）。
