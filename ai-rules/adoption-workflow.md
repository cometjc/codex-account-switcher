# Shared Baseline Sync Workflow

## Tracking model

- 每個專案以 `adopted/<project>` branch 表示最近一次完成同步的 shared baseline。
- `git diff adopted/<project>..HEAD -- AGENTS.md ai-rules/` 用來查看 shared baseline 自上次同步後的內容差異。
- `git log --oneline adopted/<project>..HEAD` 用來整理這段期間各次規則調整的 commit 脈絡。
- 若專案尚未建立 `adopted/<project>`，第一次同步前應全量閱讀 `AGENTS.md` 與它索引的正式章節。
- 不得借用其他專案的 `adopted/*` branch 當作自己的同步基線。

## Review loop

### 1. Inspect the baseline delta

- 已有同步基線：查看 `adopted/<project>..HEAD` 的檔案差異與 commit message。
- 尚無同步基線：全量閱讀 `AGENTS.md`、`ai-rules/adoption-workflow.md` 與 `ai-rules/` 的正式章節。
- 若 branch 只是剛建立來開始追蹤、但本地尚未完成第一次同步，仍視為第一次同步，應先全量盤點。

### 2. Turn changes into adoption items

不要把 shared baseline 的文字直接當成必須原封不動落地的固定模板。
應先把 diff 與 commit message 收斂成可討論的規則差異條目，例如：

- 新增了哪個核心流程
- 哪條既有規則被改嚴或放寬
- 哪個章節的索引與責任邊界被重寫

### 3. Discuss each adoption item

逐條提問並比較本地現況：

- 這個差異對應到本地哪個流程、文件、skill 或慣例？
- 差異主要在範圍、嚴格度、觸發時機、落點，還是術語？
- 本地應採取更本地化、更通用、更積極、保守，還是不採納的策略？

每個差異條目至少整理 4 個可選方案，並附上建議。

### 4. Integrate locally

- 將採納結果映射回本地 `AGENTS.md`、skill、runbook、script 或既有文件。
- 若本地已有文件覆蓋相同主題，優先整併進既有落點，而不是平行新增近似檔案。
- shared baseline 的語言保持一般化；本地術語與例外留在本地 repo。

### 5. Update the baseline

- 只有在本地落地完成後，才更新 `adopted/<project>` branch 到目前 `HEAD`。
- 不要在討論途中提早更新 baseline，否則後續 diff 會掩蓋尚未消化的差異。
