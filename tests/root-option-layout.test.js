const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');
const RootCommand = require(path.join(process.cwd(), 'dist/commands/root.js')).default;

async function loadOptionModule() {
  return import(path.join(process.cwd(), 'dist/lib/root-option-layout.js'));
}

test('option label only contains indicator, profile name, and delta', async () => {
  const {renderSelectionOptionLabel} = await loadOptionModule();

  const label = renderSelectionOptionLabel({
    indicator: '▶',
    profile: 'main-account-profile',
    delta: '+3.1%',
  });

  assert.match(label, /^▶ /);
  assert.match(label, /main-account-profile/);
  assert.match(label, /\+3\.1%/);
  assert.doesNotMatch(label, /weekly/i);
  assert.doesNotMatch(label, /Usage Left/);
  assert.doesNotMatch(label, /Time to reset/);
});

test('root command exposes minimal option labels and full prompt panel text separately', () => {
  const command = Object.create(RootCommand.prototype);
  command.ansiEnabled = false;

  const item = {
    isCurrent: true,
    profileName: 'main-account-profile',
  };
  const row = {
    profile: '▶ main-account-profile',
    lastUpdate: '2m',
    status: 'Good',
    statusValue: null,
    scoreLabel: 'Good',
    weeklyBar: '[████░░░░░░░░░░░░░░░░░░░░░░░░]',
    weeklyTimeToReset: '6.8d',
    weeklyUsageLeft: '91% left',
    weeklyDrift: '-1.6% Under',
    weeklyBottleneck: false,
    fiveHourBar: '[██████████░░░░░░░░░░░░░░░░░░]',
    fiveHourTimeToReset: '2.1h',
    fiveHourUsageLeft: '68% left',
    fiveHourDrift: '+3.1% Overuse',
    fiveHourBottleneck: true,
  };

  const optionLabel = command.renderSelectionOption(item, row);
  const panelText = command.renderPromptPanelText([row]);

  assert.match(optionLabel, /^▶ /);
  assert.match(optionLabel, /main-account-profile/);
  assert.match(optionLabel, /\+3\.1%/);
  assert.doesNotMatch(optionLabel, /weekly/i);
  assert.match(panelText, /main-account-profile/);
  assert.match(panelText, /W:/);
  assert.match(panelText, /5H:/);
});
