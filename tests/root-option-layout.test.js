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
    weeklyTimeLeftPercent: '95%',
    weeklyUsageLeft: '91% left',
    weeklyDrift: '-1.6% Under',
    weeklyBottleneck: false,
    fiveHourBar: '[██████████░░░░░░░░░░░░░░░░░░]',
    fiveHourTimeToReset: '2.1h',
    fiveHourTimeLeftPercent: '42%',
    fiveHourUsageLeft: '68% left',
    fiveHourDrift: '+3.1% Overuse',
    fiveHourBottleneck: true,
  };

  const optionLabel = command.renderSelectionOption(item, row);
  const panelText = command.renderPromptPanelText([row], 'delta');

  assert.match(optionLabel, /^▶ /);
  assert.match(optionLabel, /main-account-profile/);
  assert.match(optionLabel, /\+3\.1%/);
  assert.doesNotMatch(optionLabel, /weekly/i);
  assert.match(panelText, /main-account-profile/);
  assert.match(panelText, /last update: 2m ago/);
  assert.match(panelText, /🔄 in\s+6\.8d\s+\(95%\)/);
  assert.match(panelText, /🔄 in\s+2\.1h\s+\(42%\)/);
});

test('delta panel only colors pacing on the adopted bottleneck row', () => {
  const command = Object.create(RootCommand.prototype);
  command.ansiEnabled = true;

  const row = {
    profile: '▶ main-account-profile',
    lastUpdate: '2m',
    status: 'Good',
    statusValue: null,
    scoreLabel: 'Good',
    weeklyBar: '[████░░░░░░░░░░░░░░░░░░░░░░░░]',
    weeklyTimeToReset: '6.8d',
    weeklyTimeLeftPercent: '95%',
    weeklyUsageLeft: '91% left',
    weeklyDrift: '-1.6% Under',
    weeklyBottleneck: false,
    fiveHourBar: '[██████████░░░░░░░░░░░░░░░░░░]',
    fiveHourTimeToReset: '2.1h',
    fiveHourTimeLeftPercent: '42%',
    fiveHourUsageLeft: '68% left',
    fiveHourDrift: '+3.1% Overuse',
    fiveHourBottleneck: true,
  };

  const panelText = command.renderPromptPanelText([row], 'delta');
  const lines = panelText.split('\n');

  assert.match(lines[0], /\u001b\[90mlast update: 2m ago\u001b\[0m/);
  assert.doesNotMatch(lines[1], /\u001b\[[0-9;]*mPacing/);
  assert.match(lines[1], /Pacing\s+-1\.6%\s+Under/);
  assert.match(lines[2], /Pacing /);
  assert.match(lines[2], /Pacing\s+\u001b\[[0-9;]*m\+3\.1%\u001b\[0m\s+\u001b\[[0-9;]*mOveruse\u001b\[0m/);
});

test('quota mode keeps only quota fields under the shared profile header', () => {
  const command = Object.create(RootCommand.prototype);
  command.ansiEnabled = false;

  const row = {
    profile: '▶ main-account-profile',
    lastUpdate: '2m',
    status: 'Good',
    statusValue: null,
    scoreLabel: 'Good',
    weeklyBar: '[████░░░░░░░░░░░░░░░░░░░░░░░░]',
    weeklyTimeToReset: '6.8d',
    weeklyTimeLeftPercent: '95%',
    weeklyUsageLeft: '91% left',
    weeklyDrift: '-1.6% Under',
    weeklyBottleneck: false,
    fiveHourBar: '[██████████░░░░░░░░░░░░░░░░░░]',
    fiveHourTimeToReset: '2.1h',
    fiveHourTimeLeftPercent: '42%',
    fiveHourUsageLeft: '68% left',
    fiveHourDrift: '+3.1% Overuse',
    fiveHourBottleneck: true,
  };

  const panelText = command.renderPromptPanelText([row], 'quota');
  const lines = panelText.split('\n');

  assert.match(lines[0], /^▶ main-account-profile last update: 2m ago$/);
  assert.match(lines[1], /^    W:/);
  assert.match(lines[2], /^    5H:/);
  assert.match(panelText, /\[████/);
  assert.doesNotMatch(panelText, /📊/);
  assert.doesNotMatch(panelText, /Overuse|Under|Bottleneck/);
});

test('overuse pacing uses darker theme-neutral background tones', () => {
  const command = Object.create(RootCommand.prototype);

  assert.equal(command.pickPaceStyle(6), '\u001b[97;48;5;88m');
  assert.equal(command.pickPaceStyle(25), '\u001b[97;48;5;52m');
});
