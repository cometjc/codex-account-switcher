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

function assertFilePresent(relativePath) {
  const fullPath = repoPath(relativePath);
  assert.equal(fs.existsSync(fullPath), true, `expected ${relativePath} to exist`);
  assert.ok(fs.statSync(fullPath).isFile(), `expected ${relativePath} to be a file`);
}

test('plot-viewer scaffold files are present', () => {
  [
    'rust/plot-viewer/Cargo.toml',
    'rust/plot-viewer/Cargo.lock',
    'rust/plot-viewer/src/lib.rs',
    'rust/plot-viewer/src/main.rs',
    'rust/plot-viewer/src/app.rs',
    'rust/plot-viewer/src/input.rs',
    'rust/plot-viewer/src/model.rs',
    'rust/plot-viewer/src/paths.rs',
    'rust/plot-viewer/src/render/mod.rs',
    'rust/plot-viewer/src/render/chart.rs',
    'rust/plot-viewer/src/store.rs',
    'rust/plot-viewer/src/usage.rs',
    'bin/agent-switch.cjs',
  ].forEach(assertFilePresent);
});

test('Rust Cargo metadata locks the unified agent-switch shape', () => {
  const cargoToml = readText('rust/plot-viewer/Cargo.toml');

  assert.match(cargoToml, /^\[package\]$/m);
  assert.match(cargoToml, /^name = "agent-switch"$/m);
  assert.match(cargoToml, /^version = "0\.1\.0"$/m);
  assert.match(cargoToml, /^edition = "2021"$/m);
  assert.match(cargoToml, /^publish = false$/m);
  assert.match(cargoToml, /^description = "Rust TUI for agent profile management and usage plots"$/m);

  for (const dependency of ['anyhow', 'crossterm', 'ratatui', 'reqwest', 'serde', 'serde_json']) {
    assert.match(cargoToml, new RegExp(`^${dependency}\\s*=`, 'm'));
  }
});

test('Rust source modules keep the agent-switch entrypoints stable', () => {
  const mainRs = readText('rust/plot-viewer/src/main.rs');
  const libRs = readText('rust/plot-viewer/src/lib.rs');
  const renderMod = readText('rust/plot-viewer/src/render/mod.rs');
  const chartRs = readText('rust/plot-viewer/src/render/chart.rs');
  const appRs = readText('rust/plot-viewer/src/app.rs');

  assert.match(mainRs, /^use agent_switch::app::App;$/m);
  assert.match(mainRs, /let mut app = App::load\(store, usage, cron_status, claude_store, claude_usage_service\)\?;/);
  assert.match(mainRs, /app\.run\(\)/);

  assert.match(libRs, /^pub mod app;$/m);
  assert.match(libRs, /^pub mod paths;$/m);
  assert.match(libRs, /^pub mod store;$/m);
  assert.match(libRs, /^pub mod usage;$/m);

  assert.match(appRs, /^use crate::render;$/m);
  assert.match(appRs, /^pub struct App \{$/m);
  assert.match(appRs, /^enum PaneFocus \{$/m);
  assert.match(appRs, /let render_state = AppRenderState \{/);
  assert.match(appRs, /render::render\(frame, right_area, &render_state\);/);

  assert.match(renderMod, /^pub mod chart;$/m);
  assert.match(renderMod, /^pub fn render<State: RenderState>\(frame: &mut Frame, area: Rect, state: &State\) \{$/m);
  assert.match(renderMod, /Rust agent-switch plot view/);
  assert.match(chartRs, /title\("usage chart \(align to 7d window\)"\)/);
});

test('legacy Node product runtime files are removed while Rust shim files remain', () => {
  for (const relativePath of [
    'src/index.ts',
    'src/commands',
    'src/lib/accounts',
    'src/lib/limits',
    'src/lib/plot',
    'src/lib/route-cli-argv.ts',
    'src/lib/base-command.ts',
    'bin/agent-switch-dev.cjs',
  ]) {
    assert.equal(fs.existsSync(repoPath(relativePath)), false, `expected ${relativePath} to be removed`);
  }

  assert.equal(fs.existsSync(repoPath('scripts/link-dev-bin.cjs')), true);
  assert.equal(fs.existsSync(repoPath('scripts/unlink-dev-bin.cjs')), true);
});
