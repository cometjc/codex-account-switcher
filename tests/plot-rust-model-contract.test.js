const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

function readText(relativePath) {
  return fs.readFileSync(path.join(process.cwd(), relativePath), 'utf8');
}

test('Rust plot snapshot model keeps the embedded loader contract stable', () => {
  const rustModel = readText('src/model.rs');

  assert.match(rustModel, /#[\[]serde\(rename = "schemaVersion"\)\]/);
  assert.match(rustModel, /#[\[]serde\(rename = "generatedAt"\)\]/);
  assert.match(rustModel, /#[\[]serde\(rename = "currentProfileId"\)\]/);
  assert.match(rustModel, /pub profiles: Vec<PlotProfile>,/);
  assert.match(rustModel, /#[\[]serde\(rename = "sevenDayWindow"\)\]/);
  assert.match(rustModel, /#[\[]serde\(rename = "sevenDayPoints"\)\]/);
  assert.match(rustModel, /#[\[]serde\(rename = "fiveHourWindow"\)\]/);
  assert.match(rustModel, /#[\[]serde\(rename = "fiveHourBand"\)\]/);
  assert.match(rustModel, /#[\[]serde\(rename = "summaryLabels", default\)\]/);

  assert.match(rustModel, /pub fn load_from_path\(path: impl AsRef<Path>\) -> Result<Self> \{/);
  assert.match(rustModel, /pub fn active_profile\(&self\) -> Option<&PlotProfile> \{/);
  assert.match(rustModel, /pub fn current_profile\(&self\) -> Option<&PlotProfile> \{/);
  assert.match(rustModel, /pub fn current_profile_index\(&self\) -> Option<usize> \{/);
  assert.match(rustModel, /fn refresh_derived_state\(&mut self\) \{/);
});
