const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

function readText(relativePath) {
  return fs.readFileSync(path.join(process.cwd(), relativePath), 'utf8');
}

test('Rust main entrypoint stays wired to the unified codex-auth app', () => {
  const mainRs = readText('rust/plot-viewer/src/main.rs');
  const libRs = readText('rust/plot-viewer/src/lib.rs');

  assert.match(mainRs, /^use codex_auth::app::App;$/m);
  assert.match(mainRs, /^use codex_auth::paths::AppPaths;$/m);
  assert.match(mainRs, /^use codex_auth::store::\{AccountStore, StorePlatform\};$/m);
  assert.match(mainRs, /^use codex_auth::usage::UsageService;$/m);
  assert.match(mainRs, /let mut app = App::load\(store, usage\)\?;/);
  assert.match(mainRs, /app\.run\(\)/);

  assert.match(libRs, /^pub mod app;$/m);
  assert.match(libRs, /^pub mod paths;$/m);
  assert.match(libRs, /^pub mod store;$/m);
  assert.match(libRs, /^pub mod usage;$/m);
});
