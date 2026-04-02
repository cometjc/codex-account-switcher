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

## 本輪進行中
- [x] 釐清 `account_id` 變更的產品語意
- [x] 補強 loader 測試，明確驗證 `account_id` 變更視為新帳號
- [x] 更新 lessons，補上「新 account_id 不自動 merge」例外
- [x] 驗證相關測試

## 本輪 review
- 驗證命令：`cargo test codex_account_id_change_is_new_unsaved_account_and_preserves_saved_profile --lib`、`make test`
- 結果：全綠（單測 1/1；整體 73 + 3 + 4，含 install）

## 本輪進行中
- [x] 追 Codex 新登入帳號存成 `comet.jc` 後與舊 `t5-free` 串資料的 root cause
- [x] 先補回歸測試，重現新舊 Codex 帳號資料被視為同一份
- [x] 以最小修正切開不同 `account_id` 的 snapshot / usage / history 關聯
- [x] 驗證目標測試與必要全量測試，補上 review 結果

## 本輪 review
- 驗證命令：`cargo test codex_refresh_ignores_stale_current_name_when_account_id_changed --bin agent-switch`、`cargo test codex_account_id_change_is_new_unsaved_account_and_preserves_saved_profile --lib`、`cargo test --bin agent-switch`、`make test`
- 結果：全綠（新回歸測試 1/1；loader 回歸 1/1；bin tests 4/4；整體 73 + 4 + 4，含 install）

## 本輪進行中
- [x] 重現 chart end-label 靠左下角時消失的條件
- [x] 補回歸測試，鎖住 `comet.jc` 類型 label 在左緣仍可顯示
- [x] 以最小修正調整左側 label 佈局，避免可貼齊左邊界時仍被丟棄
- [x] 驗證目標測試與必要全量測試

## 本輪 review
- 驗證命令：`cargo test layout_end_labels_clamps_left_edge_instead_of_dropping_label --lib`、`cargo test render::chart::tests --lib`、`make test`
- 結果：全綠（新回歸測試 1/1；chart tests 7/7；整體 74 + 4 + 4，含 install）

## 本輪進行中
- [x] 補回歸測試，重現 Copilot monthly quota 明細可見但 chart 無點
- [x] 以 normalized window 正式實作各服務 quota 投影，移除 Copilot 臨時 7d fallback 語意
- [x] 將 chart 文案改為中性，series label 顯示 window tag，detail pane 改用 `Quota:`
- [x] 驗證目標測試與全量測試

## 本輪 review
- 驗證命令：`cargo test map_copilot_usage_response_handles_business_quota_snapshots --lib`、`cargo test render::chart::tests --lib`、`cargo test account_detail_uses_quota_label_for_longer_windows --lib`、`cargo test copilot_monthly_usage_projects_month_window_into_normalized_chart --lib`、`cargo test loader::tests --lib`、`make test`
- 結果：全綠（Copilot adapter 1/1；chart tests 7/7；detail pane 回歸 1/1；Copilot normalized-chart 回歸 1/1；loader tests 13/13；整體 76 + 4 + 4，含 install）


## 本輪進行中
- [x] 釐清 `comet.jc` 在特殊尺寸下 tag 消失是否屬於 layout 問題
- [x] 補回歸測試，鎖住「label 已存在於 buffer 但需有獨立底色才能穩定可讀」的情境
- [x] 以最小修正強化 chart end-label 可讀性，避免特殊終端尺寸下視覺消失
- [x] 驗證目標測試與必要全量測試

## 本輪 review
- 驗證命令：`cargo test render_chart_gives_labels_an_opaque_background_for_readability --lib`、`cargo test render_chart_keeps_comet_label_visible_beside_neighboring_series --lib`、`cargo test render::chart::tests --lib`、`make test`
- 結果：全綠（新回歸測試 1/1；鄰近 series 回歸 1/1；chart tests 10/10；整體 79 + 4 + 4，含 install）


## 本輪進行中
- [x] 用 tmux live pane 重現 `comet` tag 消失，而非只看測試 render
- [x] 確認 root cause 是 label 佈局在 plot/band 佔滿候選位時直接放棄
- [x] 補上強制 fallback label 機制，保證至少顯示最小 profile tag
- [x] 驗證 chart tests、`make test`，並在 `%134` live pane 重啟確認 `comet` 出現

## 本輪 review
- 驗證命令：`cargo test layout_end_labels_forces_minimal_label_when_plot_and_band_claim_every_candidate --lib`、`cargo test render::chart::tests --lib`、`make test`、`tmux capture-pane -p -J -t %134 -S -45 | rg 'comet|teamt5-it'`
- 結果：全綠（新回歸測試 1/1；chart tests 14/14；整體 83 + 4 + 4，含 install；live pane `%134` 已顯示 `comet`）

## 本輪進行中
- [x] 對齊 5h 上緣新語意：以歷史 5h used 最大前三窗口的 `7d_delta / 5h_used` 平均值映射到 100% 5h 高度
- [x] 補 loader 回歸測試，鎖住 top3 average rate 計算
- [x] 最小修改 five_hour subframe 上緣推估
- [x] 驗證目標測試與 `make test`

## 本輪 review
- 驗證命令：`cargo test five_hour_subframe_uses_average_rate_of_top3_largest_5h_windows --lib`、`cargo test loader::tests --lib`、`make test`
- 結果：全綠（新回歸測試 1/1；loader tests 14/14；整體 84 + 4 + 4，含 install）
