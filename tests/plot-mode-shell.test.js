const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

function readText(relativePath) {
  return fs.readFileSync(path.join(process.cwd(), relativePath), 'utf8');
}

test('plot mode is rendered from Rust app state, not Node shell wiring', () => {
  const appRs = readText('rust/plot-viewer/src/app.rs');
  const renderMod = readText('rust/plot-viewer/src/render/mod.rs');
  const chartRs = readText('rust/plot-viewer/src/render/chart.rs');

  assert.match(appRs, /pub enum ViewMode \{/);
  assert.match(appRs, /ViewMode::Plot/);
  assert.match(appRs, /render::render\(frame, frame\.area\(\), &render_state\);/);

  assert.match(renderMod, /pub mod chart;/);
  assert.match(renderMod, /pub mod panels;/);
  assert.match(renderMod, /title\("codex-auth plot"\)/);
  assert.match(renderMod, /Selected: .*Current: .*Visible profiles:/);
  assert.match(renderMod, /Tab switches panel focus/);

  assert.match(chartRs, /usage plot overlays/);
  assert.match(chartRs, /Profiles: /);
  assert.match(chartRs, /Legend: /);
  assert.match(chartRs, /5h frame: /);
  assert.match(chartRs, /5h band: /);
});
