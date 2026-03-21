const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

function readText(relativePath) {
  return fs.readFileSync(path.join(process.cwd(), relativePath), 'utf8');
}

test('plot Rust model stays aligned with the TypeScript snapshot contract', () => {
  const rustModel = readText('rust/plot-viewer/src/model.rs');
  const tsSnapshot = readText('src/lib/plot/plot-snapshot.ts');

  assert.match(tsSnapshot, /export interface PlotSnapshot \{/);
  assert.match(tsSnapshot, /schemaVersion: 1;/);
  assert.match(tsSnapshot, /generatedAt: number;/);
  assert.match(tsSnapshot, /currentProfileId: string \| null;/);
  assert.match(tsSnapshot, /profiles: PlotProfile\[];/);
  assert.match(tsSnapshot, /sevenDayWindow: PlotWindowBounds;/);
  assert.match(tsSnapshot, /sevenDayPoints: PlotWindowPoint\[];/);
  assert.match(tsSnapshot, /fiveHourWindow: PlotWindowBounds;/);
  assert.match(tsSnapshot, /fiveHourBand: PlotFiveHourBand;/);
  assert.match(tsSnapshot, /summaryLabels: PlotSummaryLabels;/);

  assert.match(rustModel, /#[\[]serde\(rename = "schemaVersion"\)\]/);
  assert.match(rustModel, /#[\[]serde\(rename = "generatedAt"\)\]/);
  assert.match(rustModel, /#[\[]serde\(rename = "currentProfileId"\)\]/);
  assert.match(rustModel, /pub profiles: Vec<PlotProfile>,/);
  assert.match(rustModel, /#[\[]serde\(rename = "sevenDayWindow"\)\]/);
  assert.match(rustModel, /#[\[]serde\(rename = "sevenDayPoints"\)\]/);
  assert.match(rustModel, /#[\[]serde\(rename = "fiveHourWindow"\)\]/);
  assert.match(rustModel, /#[\[]serde\(rename = "fiveHourBand"\)\]/);
  assert.match(rustModel, /#[\[]serde\(rename = "summaryLabels", default\)\]/);

  assert.match(rustModel, /pub fn active_profile\(&self\) -> Option<&PlotProfile> \{/);
  assert.match(rustModel, /pub fn current_profile\(&self\) -> Option<&PlotProfile> \{/);
  assert.match(rustModel, /pub fn current_profile_index\(&self\) -> Option<usize> \{/);
  assert.match(rustModel, /fn refresh_derived_state\(&mut self\) \{/);
});
