const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

async function importUiStateModule(homeDir) {
  const previousHome = process.env.HOME;
  process.env.HOME = homeDir;
  try {
    return await import(`${path.join(process.cwd(), 'dist/lib/config/ui-state.js')}?home=${encodeURIComponent(homeDir)}&t=${Date.now()}`);
  } finally {
    process.env.HOME = previousHome;
  }
}

test('ui state persists workload tier and falls back safely on invalid contents', async () => {
  const homeDir = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-ui-state-'));
  const codexDir = path.join(homeDir, '.codex');
  fs.mkdirSync(codexDir, {recursive: true});

  const module = await importUiStateModule(homeDir);

  assert.equal(await module.uiStateService.readWorkloadTier(), 'auto');

  await module.uiStateService.writeWorkloadTier('high');
  assert.equal(await module.uiStateService.readWorkloadTier(), 'high');

  fs.writeFileSync(path.join(codexDir, 'codex-auth-ui-state.json'), '{"workloadTier":"broken"}\n', 'utf8');
  assert.equal(await module.uiStateService.readWorkloadTier(), 'auto');
});
