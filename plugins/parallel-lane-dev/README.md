# parallel-lane-dev（agent-switch 開發用）

本目錄提供 **PLD executor 與協調腳本** 的專案內入口；**執行真相**在 repo 根的 **`.pld/executor.sqlite`**（已 gitignore）。

## 安裝 `scripts/` 與 `skills/` 連結

預設假設與本 repo 同層的 monorepo 佈局：

`repo/side-projects/agent-switch` 與 `repo/agent-plugins` 並列，`scripts` 與 `skills` 為指向

`../../../../agent-plugins/plugins/parallel-lane-dev/{scripts,skills}` 的 symlink（自 `plugins/parallel-lane-dev/` 起算，因本 repo 在 `side-projects/` 下）。

若路徑不同，請執行：

```sh
chmod +x scripts/install-pld-plugin.sh
./scripts/install-pld-plugin.sh
# 或自訂上游套件根目錄（其下需有 scripts/ 與 skills/）：
PLD_PLUGIN_ROOT=/path/to/plugins/parallel-lane-dev ./scripts/install-pld-plugin.sh
```

## 常用指令（亦見根目錄 `package.json` 的 `pld:*`）

```sh
npm run pld:executor:audit -- --json
npm run pld:executor:go -- --json
```

固定口令 **pld-go** 的機械對應為：`pld:executor:go`（見上游技能 `skills/parallel-lane-dev/SKILL.md`，在 `agent-plugins` 套件內）。

## 本專案專屬 surface

- `scoreboard.md`、`executions/agent-switch/**`：只描述 **agent-switch** 的 execution，勿與 `agent-plugins` 內建範例混淆。
- 規格與長篇流程說明請讀上游 `agent-plugins/plugins/parallel-lane-dev/spec/PLD/`。
