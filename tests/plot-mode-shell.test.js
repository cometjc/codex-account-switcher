const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

function readText(relativePath) {
  return fs.readFileSync(path.join(process.cwd(), relativePath), 'utf8');
}

test('plot mode is rendered from Rust app state, not Node shell wiring', () => {
  const appRs = readText('src/app.rs');
  const renderMod = readText('src/render/mod.rs');
  const chartRs = readText('src/render/chart.rs');

  assert.match(appRs, /enum PaneFocus \{/);
  assert.match(appRs, /PaneFocus::Plot/);
  assert.match(appRs, /render::render\(frame, right_area, &render_state\);/);

  assert.match(renderMod, /pub mod chart;/);
  assert.match(renderMod, /Rust agent-switch plot view/);
  assert.match(appRs, /cursor_x: None/);
  assert.match(appRs, /x_window_days: 7/);

  assert.match(chartRs, /usage chart \(align to 7d window\)/);
  assert.match(chartRs, /Window: \{\} · \+\/-=zoom · z=reset · 1\/3\/7=window/);
  assert.match(chartRs, /Band drift: 7d/);
  assert.match(chartRs, /5h frame: /);
  assert.match(chartRs, /5h band: /);
});
