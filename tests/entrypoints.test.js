const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

function repoPath(...segments) {
  return path.join(process.cwd(), ...segments);
}

function readText(relativePath) {
  return fs.readFileSync(repoPath(relativePath), 'utf8');
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

test('package bin points at the Rust thin shim', () => {
  const packageJson = require(repoPath('package.json'));
  const shimPath = repoPath('bin/codex-auth.cjs');

  assert.equal(packageJson.bin['codex-auth'], 'bin/codex-auth.cjs');
  assert.equal(
    packageJson.scripts.build,
    'cargo build --manifest-path rust/plot-viewer/Cargo.toml --bin codex-auth',
  );
  assert.ok(fs.existsSync(shimPath), 'expected Rust thin shim to exist');

  const shimSource = fs.readFileSync(shimPath, 'utf8');
  assert.match(shimSource, /run\(cargo, \['run'/);
  assert.match(shimSource, /Cargo\.toml/);
  assert.match(shimSource, /CODEX_AUTH_BIN/);
});

test('legacy Node product entrypoints are gone', () => {
  const packageJson = require(repoPath('package.json'));

  assert.equal(packageJson.dependencies?.['@oclif/core'], undefined);
  assert.equal(packageJson.oclif, undefined);
  assert.equal(fs.existsSync(repoPath('src/index.ts')), false);
  assert.equal(fs.existsSync(repoPath('src/commands')), false);
  assert.equal(fs.existsSync(repoPath('bin/codex-auth-dev.cjs')), false);
});
