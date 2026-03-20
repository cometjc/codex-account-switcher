# 2026-03-20 codex-auth-dev 入口對齊

- [x] 重現 `codex-auth-dev` 與 `codex-auth` 無參數執行時的入口差異並鎖定根因
- [x] 先補回歸測試，驗證 dev bin 無參數時要與正式 bin 一樣進入互動模式
- [x] 修改 dev bin 入口，改走與正式版一致的 argv 路由
- [x] 執行建置與測試驗證，並在 review 區塊記錄結果

## Review

- 根因：[`bin/codex-auth-dev.cjs`](/home/jethro/repo/side-projects/codex-account-switcher/bin/codex-auth-dev.cjs) 直接呼叫 oclif `execute(...)`，沒有像正式入口一樣把空 argv 轉成 `root`，所以無參數執行時只會顯示 help。
- 修正：抽出共用的 [`src/lib/route-cli-argv.ts`](/home/jethro/repo/side-projects/codex-account-switcher/src/lib/route-cli-argv.ts)，讓 [`src/index.ts`](/home/jethro/repo/side-projects/codex-account-switcher/src/index.ts) 與 [`bin/codex-auth-dev.cjs`](/home/jethro/repo/side-projects/codex-account-switcher/bin/codex-auth-dev.cjs) 使用同一套路由規則。
- 回歸測試：新增 [`tests/entrypoints.test.js`](/home/jethro/repo/side-projects/codex-account-switcher/tests/entrypoints.test.js) 驗證 shared router 與 dev entrypoint 都會把空 argv 導到 `root`。
- 驗證：
  `npm run build`
  `node --test tests/entrypoints.test.js`
  `HOME="$(mktemp -d)" ... node bin/codex-auth-dev.cjs` 與 `node dist/index.js` 都顯示 `Select profile`
