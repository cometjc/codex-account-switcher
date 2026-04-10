# Event-Driven Label Relayout Design

## 背景
現行 label 佈局使用較大範圍候選與 A* 路由，若每次 render 都重算，會造成 CPU 偏高。目標是改為事件觸發重算，並盡量局部重排，維持視覺品質與效能平衡。

## 目標
- 不再每幀做 cheap check 或全量重排。
- 僅在「新資料點」或「chart 視窗調整（pan/zoom/bounds）」時觸發檢查。
- 優先重排受影響 labels；局部失敗時自動升級一次全域重排。
- 保留既有評分模型（variant penalty + connector cost + overlap）與右側優先語意。

## 非目標
- 不重寫 A* 演算法本身。
- 不改動資料來源或更新頻率。
- 不引入多執行緒/背景 worker。

## 設計概覽
### 1) 事件觸發
建立 `RelayoutTrigger`：
- `DataPointUpdated`：有新資料點進入目前 chart state。
- `ViewportChanged`：x/y bounds、pan/zoom 改變。

只有觸發事件時，才執行後續 cheap check。

### 2) 快取結構
新增 `LabelLayoutCache`（以 profile id 或 series id 索引）：
- `placement`: x/y/variant/connector_path
- `baseline_score`: 上次成功佈局分數
- `anchor_snapshot`: 上次 anchor 座標
- `fingerprint`: blocked hash + bounds version

### 3) Cheap Check（事件觸發時）
每個 label 檢查：
- `hard_invalidation`: 目前 label box 或 connector path 與 blocked/reserved 相交。
- `endpoint_drift`: anchor 位移超過門檻（例如 `abs(dx)+abs(dy) > drift_threshold`）。
- `score_drift`: 以既有 placement 重算分數，若 `current_score > baseline_score + 10`。

命中任一條件即標記為 dirty label。

### 4) 局部重排與升級策略
- 先只對 dirty labels 執行重排（其餘沿用 cache placement）。
- 若任一 dirty label 找不到可行解，或局部結果產生新衝突：
  - 立即執行一次全域重排（所有 labels）。
- 全域成功後刷新所有 cache 與 baseline。

### 5) 既有演算法整合
- 保留候選上限（`MAX_CANDIDATES_PER_VARIANT`）。
- 保留 A* 路由成本：水平 1、垂直 4。
- 保留 variant penalty 與 right-side preference。

## 資料流
1. Render 入口收到 chart state。
2. 判斷是否有 `RelayoutTrigger`。
3. 若無 trigger：直接使用 cache placements。
4. 若有 trigger：對每 label 跑 cheap check。
5. dirty labels 局部重排；失敗則全域重排。
6. 寫回 cache 並繪製 connector/labels。

## 錯誤與回退
- 局部重排失敗：自動升級全域重排。
- 全域仍失敗：回退到「上次可用 placement + 最小化文字變體」避免畫面空白。
- cache 缺失或 fingerprint 不一致：視為 trigger，走正常重排流程。

## 測試策略
### 單元測試
- trigger 判斷：無事件不重排；新資料/視窗改變才重排。
- cheap check：
  - blocked 壓到 label/path 會 dirty
  - endpoint drift 超閥值會 dirty
  - score drift 超過 `+10` 會 dirty
- 局部重排成功時不影響未 dirty labels。
- 局部失敗會自動進入全域重排。

### 整合/行為測試
- live-like chart 案例下，label 仍可穩定往右優先。
- 無事件連續 render，layout 計算次數顯著下降。
- 高密度 blocked 案例下仍可得到可讀輸出（含 fallback）。

### 效能驗證
- 比較事件前後 CPU 佔用：
  - 目標：在無事件穩態下，layout 計算趨近 0 次/幀。
  - 事件發生時允許瞬時計算尖峰，但平均 CPU 明顯低於每幀重算。

## 風險
- cache 與實際畫面不同步可能造成短暫重疊。
- dirty 判準過寬會退化成接近全域重算。
- dirty 判準過窄可能延遲最佳位置更新。

## 可調參數
- `drift_threshold`（endpoint 位移門檻）
- `score_delta_threshold`（預設 10）
- `MAX_CANDIDATES_PER_VARIANT`

## 實作切分建議
1. 先加 cache 與 trigger gate（不改佈局結果）。
2. 加 cheap check + dirty 集合。
3. 加局部重排與全域升級。
4. 補效能與回歸測試。
