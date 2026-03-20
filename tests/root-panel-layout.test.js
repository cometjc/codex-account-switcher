const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

async function loadPanelModule() {
  return import(path.join(process.cwd(), 'dist/lib/root-panel-layout.js'));
}

test('panel renders all profiles with profile/last first line and aligned detail rows', async () => {
  const {computePanelWidths, renderRootDetailPanel} = await loadPanelModule();

  const rows = [
    {
      profile: 'main-account-profile',
      lastUpdate: '2m',
      weeklyUsageLeft: '91% left',
      weeklyTimeToReset: '6.8d',
      weeklyDelta: '-1.6%',
      fiveHourUsageLeft: '68% left',
      fiveHourTimeToReset: '2.1h',
      fiveHourDelta: '+3.1%',
    },
    {
      profile: 'backup-account',
      lastUpdate: '11m',
      weeklyUsageLeft: '74% left',
      weeklyTimeToReset: '5.2d',
      weeklyDelta: '+0.4%',
      fiveHourUsageLeft: 'n/a',
      fiveHourTimeToReset: 'n/a',
      fiveHourDelta: 'n/a',
    },
  ];

  const widths = computePanelWidths(rows);
  const panel = renderRootDetailPanel(rows, widths);
  const lines = panel.split('\n');

  assert.match(lines[0], /main-account-profile/);
  assert.match(lines[0], /2m/);
  assert.match(panel, /backup-account/);
  assert.match(panel, /W:/);
  assert.match(panel, /5H:/);

  const firstWeekly = lines[1];
  const secondWeekly = lines[4];
  assert.equal(firstWeekly.indexOf('91% left'), secondWeekly.indexOf('74% left'));
  assert.equal(firstWeekly.indexOf('6.8d'), secondWeekly.indexOf('5.2d'));
  assert.equal(firstWeekly.indexOf('-1.6%'), secondWeekly.indexOf('+0.4%'));
});
