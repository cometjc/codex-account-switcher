# 待辦（進行中）

此檔保留**當前迭代**的核取清單與簡短備註。

- [x] 規劃 chart label 外移 + 連接線配置規則（避免與曲線重疊）
- [x] 實作 end label 外移與連接線繪製
- [x] 新增/調整測試以覆蓋連接線與避讓行為
- [x] 執行 cargo test 並整理結果
- [x] 對齊驗證規則：`CLAUDE.md` Build Rule 改為 `make test` 預設
- [x] 執行 `make test` 並記錄結果
- [x] 修正 chart x 軸語意為 reset-aligned（x=0=start, x=7=reset）
- [x] 更新 x 軸右側標籤文字（now -> reset）
- [x] 執行 `make test` 驗證 reset-aligned 座標行為
- [x] 修正 file watcher 忽略 refresh log，避免 app reload 自激迴圈
- [x] 執行 `make test` 驗證 reload loop 修正

- [x] 針對 app 常駐 + cron 併發寫入建立可重現測試
- [x] 驗證 history 不互蓋且 observation 持續累積
- [x] 驗證 chart 仍可顯示 rolling 7d 內點位
- [x] 執行 cargo test 並整理證據

## review
- `CLAUDE.md` 的 Build Rule 已與 `AGENTS.md` 對齊，預設驗證改為 `make test`，`cargo test` 僅保留快速檢查用途。
- 已改用 `make test` 完整驗證（含 install），結果全綠。
- chart x 軸語意已改為 reset-aligned：`x=0` 對應 `reset_time-7d`，`x=7` 對應 `reset_time`；右側刻度文字改為 `reset`。
- `reload_profiles` 會寫入 `agent-switch-refresh.log`，原先 watcher 會把此檔案變更視為外部更新導致連續 reload；現已忽略該檔案事件。
- end label 先嘗試外移 3 格並畫 `-`/`|` 連接線，空間不足時退回外移 1 格，減少標籤貼點位造成混淆。
- 連接線避開 band 背景與已保留 label 區，允許跨越曲線避免過度嚴格導致 label 消失。
- `cargo test` 全綠（72 + 3 + 4）。
- 以並行 writer 測試重現「晚到舊時間戳寫入會回滾刪除新點」根因。
- 修正 prune 錨點為 `max(now, 現存最新 observation)`，避免 app/cron 交錯時互相回滾。
- 新增 chart 端併發寫入可見性測試，確認 rolling 7d 仍可投影出點位。
- `cargo test` 全綠（69 + 1 + 4）。
