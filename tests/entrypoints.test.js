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

test('Rust main entrypoint stays wired to the unified agent-switch app', () => {
  const mainRs = readText('rust/plot-viewer/src/main.rs');
  const libRs = readText('rust/plot-viewer/src/lib.rs');

  assert.match(mainRs, /^use agent_switch::app::App;$/m);
  assert.match(mainRs, /^use agent_switch::paths::AppPaths;$/m);
  assert.match(mainRs, /^use agent_switch::store::\{AccountStore, StorePlatform\};$/m);
  assert.match(mainRs, /^use agent_switch::usage::UsageService;$/m);
  assert.match(mainRs, /^use agent_switch::claude::\{ClaudePaths, ClaudeStore, ClaudeCredentials\};$/m);
  assert.match(mainRs, /let mut app = App::load\(store, usage, cron_status, claude_store, claude_usage_service\)\?;/);
  assert.match(mainRs, /app\.run\(\)/);

  assert.match(libRs, /^pub mod app;$/m);
  assert.match(libRs, /^pub mod paths;$/m);
  assert.match(libRs, /^pub mod store;$/m);
  assert.match(libRs, /^pub mod usage;$/m);
});

test('package bin points at the Rust thin shim', () => {
  const packageJson = require(repoPath('package.json'));
  const shimPath = repoPath('bin/agent-switch.cjs');

  assert.equal(packageJson.bin['agent-switch'], 'bin/agent-switch.cjs');
  assert.equal(
    packageJson.scripts.build,
    'cargo build --manifest-path rust/plot-viewer/Cargo.toml --bin agent-switch',
  );
  assert.ok(fs.existsSync(shimPath), 'expected Rust thin shim to exist');

  const shimSource = fs.readFileSync(shimPath, 'utf8');
  assert.match(shimSource, /run\(cargo, \['run'/);
  assert.match(shimSource, /Cargo\.toml/);
  assert.match(shimSource, /AGENT_SWITCH_BIN/);
});

test('dev link scripts point at the Rust thin shim workflow', () => {
  const packageJson = require(repoPath('package.json'));
  const linkScript = readText('scripts/link-dev-bin.cjs');
  const unlinkScript = readText('scripts/unlink-dev-bin.cjs');

  assert.equal(packageJson.scripts['link:dev'], 'node scripts/link-dev-bin.cjs');
  assert.equal(packageJson.scripts['unlink:dev'], 'node scripts/unlink-dev-bin.cjs');
  assert.match(linkScript, /agent-switch\.cjs/);
  assert.match(linkScript, /const targetPath = path\.join\(targetDir, 'agent-switch'\);/);
  assert.match(unlinkScript, /agent-switch\.cjs/);
  assert.match(unlinkScript, /Refusing to remove symlink not owned by this repo/);
});

test('legacy Node product entrypoints are gone', () => {
  const packageJson = require(repoPath('package.json'));

  assert.equal(packageJson.dependencies?.['@oclif/core'], undefined);
  assert.equal(packageJson.oclif, undefined);
  assert.equal(fs.existsSync(repoPath('src/index.ts')), false);
  assert.equal(fs.existsSync(repoPath('src/commands')), false);
  assert.equal(fs.existsSync(repoPath('bin/agent-switch-dev.cjs')), false);
});
