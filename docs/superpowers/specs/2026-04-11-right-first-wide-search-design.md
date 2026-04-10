# Right-first wide-search label routing design

## Problem

目前標籤雖然有 A* 與候選排序，但在部分擁擠案例會出現：

- 右側其實有空位，卻提早選到上方或左側
- connector 為了短距離走出較差可讀性路徑
- 一旦局部候選撞到 reserved/blocked，就過早退化到次佳方案

這與預期「右側可放就優先右側、路徑可讀」不一致。

## Goal

在維持現有事件觸發 relayout 架構下，提升 label placement 與 connector routing 的可讀性：

1. 有合法右側位置時，優先右側。
2. 搜尋範圍能覆蓋更大區域，避免局部碰撞導致過早放棄。
3. 路徑成本能表達「少繞、少穿越 blocked 鄰域、少向左回頭」的偏好。
4. CPU 成本受控，不回到每幀高負載。

## Non-goals

- 不改資料來源與 ChartState 版本觸發機制。
- 不引入新使用者設定開關。
- 不改 zero-state label 幾何規則。

## Approved behavior

### 1) Right-first with wider candidate window

對每個 anchor 的候選座標生成改為「右側優先、上下擴窗」：

- 第一層：先掃右側近距離 offset（既有偏好維持）。
- 第二層：擴大同列與鄰近列 offset 搜尋半徑（例如目前 `LABEL_SEARCH_X_LIMIT` 的擴充版）。
- 第三層：若右側仍無合法解，再納入左側候選。

重點是「候選池先擴大，再比較評分」，而不是「撞到就停」。

### 2) Weighted A* routing cost

connector 的 A* 成本新增可調權重（常數）：

- `left_penalty`: 往 anchor 左側方向延伸時加權。
- `turn_penalty`: 每次方向轉折加權。
- `blocked_proximity_penalty`: 經過 blocked/exclusion 鄰近格時加權。
- `detour_penalty`: 路徑總長超過 Manhattan baseline 的超額懲罰。

保留合法性檢查（不可穿越 reserved/label rect），只調整「合法解之間」的排序。

### 3) Placement score alignment

`layout_end_labels_with_reserved` 的 candidate score 調整為：

- 先比 variant priority（full > compact > minimal，既有規則不變）
- 再比 right-side bias（右側 attach 明確加分）
- 再比 weighted connector cost
- 最後比 displacement tie-breaker（dy, dx）

使「右側且乾淨路徑」在同等合法性下穩定勝出。

### 4) Guardrails

維持事件觸發重排，不增加每幀計算：

- 只在 `layout_data_version/layout_viewport_version` 觸發時進入 search/routing。
- 無 trigger 仍直接走 cache。
- 既有 partial relayout 與 global fallback 流程不變。

## Architecture changes

主要修改 `src/render/chart.rs`：

1. `candidate_positions_for_label`
- 拆分右側擴窗與左側補充候選生成。
- 增加可配置搜尋上限常數（含每 variant candidate cap）。

2. `route_connector_path`
- 在既有 A* 上增加 weighted step cost（left/turn/blocked-near/detour）。
- 保持原本邊界與 blocking 條件。

3. `layout_end_labels_with_reserved`
- 調整 score 組合與 tie-break 順序。
- force placement phase 套用同一成本語意。

## Testing

在 `src/render/chart.rs` 測試補強：

1. 右側有合法空間時，不應選上方/左側次佳解。
2. 右側近距離被局部擋住、但較遠右側可放時，應命中較遠右側。
3. 兩條合法 connector 中，應偏好轉折較少、遠離 blocked 鄰域者。
4. 在 score 接近時，右側 attach 應優先於左側 attach。
5. 現有 regression（full/compact/minimal chain、blocked gap、partial/global relayout）不可退化。

驗證流程：

- `cargo test render::chart::tests:: -- --nocapture`
- `make test`
- tmux live pane spot-check（同 `main:0.0`）

## Risks and mitigations

1. 風險：候選擴窗讓單次觸發成本上升。
- 緩解：保留 candidate cap + only-on-trigger gate。

2. 風險：權重不當導致路徑過度繞行。
- 緩解：以小步常數調整，靠回歸測試鎖定行為。

3. 風險：force placement 行為被新評分意外改寫。
- 緩解：保留 force phase 結構，只替換成本函式與排序鍵。

## Acceptance criteria

- 在「右側空間充足」案例，label 不再優先跑到上方/左側。
- connector 路徑在可行解中更少回頭與貼 blocked 邊緣。
- 無 trigger 連續 render 不增加 relayout 次數。
- `make test` 全綠，live chart 無明顯可讀性退化。
