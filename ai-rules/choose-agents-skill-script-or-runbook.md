# 依改善內容型態決定落到 AGENTS、skill、script 或 runbook

## 核心要求

- 每次收斂出可重複沿用的改善時，先判斷它屬於治理規則、情境 workflow、機械步驟，還是低頻知識/證據。
- 高頻、可泛化、跨案例的治理要求，放進本地 `AGENTS.md`。
- 有 trigger、決策節點、續作順序與 exit criteria 的 workflow，放進 skill。
- 輸入輸出明確、值得自動化的機械步驟，放進 script。
- 低頻但關鍵的排障知識、證據與查表資訊，放進 runbook 或 context。

## 為什麼需要這條規則

agent 協作型專案通常同時擁有治理規則、workflow 技能、腳本工具與各式排障筆記。
若沒有明確邊界，常見問題包括：

- `AGENTS.md` 持續膨脹成操作手冊
- skill 混入一次性 issue 證據與歷史紀錄
- 明明可腳本化的重複工作仍靠手編
- 低頻但關鍵的取證知識無處安放，只能散落在對話或 commit message

將改善內容先按型態分類，再決定落點，可以降低規則膨脹與工具重複。

## 本地同步時要討論什麼

- 本地 `AGENTS.md` 是否已明確區分治理規則與 workflow 細節。
- 本地 skill 是否承接了真正需要的流程判斷，而不是純機械編輯。
- 哪些重複手動步驟已到 script 化門檻。
- runbook 或 context 是否已有固定落點，能承接低頻但關鍵的知識。

## 驗證方式

- 檢查專案規則是否明確區分 AGENTS、skill、script、runbook/context 的責任邊界。
- 檢查新增 workflow 細節時，是否優先改 skill，而非直接堆進 AGENTS。
- 檢查重複手動操作是否在第二到第三次後被評估並收斂為 script。
- 檢查低頻排障知識是否被放進 runbook/context，而不是散落在臨時對話中。
- 檢查本地較成熟的專案是否已把 script 化門檻與 runbook/context 落點寫成可執行規則，而非只有抽象分類。
