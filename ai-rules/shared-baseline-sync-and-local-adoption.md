# 以 shared baseline 同步共同規則，並在本地逐條討論採納

## 核心要求

- 組織維護之共同 baseline 倉庫為上游規則來源；**本 repo** 的 `AGENTS.md` 為本地治理權威入口。
- 與 baseline 對齊之正式章節置於本 repo 的 `ai-rules/*.md`，並由本 `AGENTS.md` 索引。
- 各專案在 **baseline 倉庫** 內以 `adopted/<project>` branch 表示最近一次完成同步的 baseline commit。
- 後續跟進時，應根據 baseline diff 與其間 commit message 整理規則差異條目，再逐條討論本地採納。
- 本地落地完成後，才更新 baseline 倉庫的 `adopted/<project>` branch。

## 為什麼需要這條規則

如果 shared rule repo 沒有固定的權威入口與同步模型，常見結果是：

- 入口分散，無法快速判斷哪份文件才是正式版本
- 章節彼此版本不同步，導致專案同步時不知道應以哪一份為準
- 採納討論直接綁在檔案 metadata，而不是從實際變更與 commit 脈絡整理差異

把本 repo 的 `AGENTS.md` 固定為本地權威入口，並把與 baseline 對齊的內容收斂到少數正式章節，可以讓同步流程更清楚。

## 本地同步時要討論什麼

- 本地是否已有對應章節或流程，可直接映射 shared baseline 的要求。
- 這次 baseline diff 代表的是新規則、責任邊界調整，還是既有要求被加嚴或放寬。
- 哪些差異應採更本地化、更通用、更積極、保守或不採納的策略。
- commit message 是否已足夠支撐差異條目的整理；若不足，後續是否需要強化 shared repo 的提交習慣。

## 驗證方式

- 檢查本 repo 的 `AGENTS.md` 是否能獨立說明專案定位、核心章節與 baseline 索引。
- 檢查 `ai-rules/*.md` 是否都是 `AGENTS.md` 索引的正式章節。
- 檢查同步流程是否明確要求同時查看 baseline diff 與 commit message。
- 檢查更新 baseline 倉庫內 `adopted/<project>` branch 的時機是否落在本地採納完成之後。
