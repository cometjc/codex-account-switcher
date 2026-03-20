const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

async function loadPanelModule() {
  return import(path.join(process.cwd(), 'dist/lib/root-panel-layout.js'));
}

test('panel renders profile header, whitespace-aligned detail rows, and hides missing limit rows', async () => {
  const {computePanelWidths, renderRootDetailPanel} = await loadPanelModule();

  const rows = [
    {
      profile: '▶ comet.jc-gmail.com-plus',
      lastUpdate: 'last update: 2.0h ago',
      weekly: {
        label: 'W:',
        usageLeft: '📊  95% left',
        resetLabel: '🔄 in',
        resetTime: '6.7d',
        resetPercent: '(95%)',
        pacingLabel: 'Pacing',
        pacingValue: '+0.7%',
        pacingDescription: 'Overuse',
      },
      fiveHour: {
        label: '5H:',
        usageLeft: '📊  98% left',
        resetLabel: '🔄 in',
        resetTime: '2.8h',
        resetPercent: '(56%)',
        pacingLabel: 'Pacing',
        pacingValue: '-41.9%',
        pacingDescription: 'Under',
      },
    },
    {
      profile: '  jethro-teamt5.org-free',
      lastUpdate: 'last update: 1.5h ago',
      weekly: {
        label: 'W:',
        usageLeft: '📊   2% left',
        resetLabel: '🔄 in',
        resetTime: '5.5d',
        resetPercent: '(79%)',
        pacingLabel: 'Pacing',
        pacingValue: '+77.3%',
        pacingDescription: 'Overuse',
      },
      fiveHour: null,
    },
  ];

  const widths = computePanelWidths(rows);
  const panel = renderRootDetailPanel(rows, widths);
  const lines = panel.split('\n');

  assert.equal(
    lines[0],
    '▶ comet.jc-gmail.com-plus last update: 2.0h ago',
  );
  assert.match(lines[1], /^    W:/);
  assert.match(lines[2], /^    5H:/);
  assert.match(panel, /📊  95% left/);
  assert.doesNotMatch(panel, / \| /);
  assert.match(panel, /🔄 in\s+6\.7d\s+\(95%\)/);
  assert.match(panel, /Pacing\s+\+0\.7%\s+Overuse/);
  assert.match(panel, /Pacing\s+-41\.9%\s+Under/);
  assert.doesNotMatch(panel, /N\/A|n\/a/);

  const secondProfileHeaderIndex = lines.findIndex((line) =>
    line.includes('jethro-teamt5.org-free'),
  );
  assert.equal(lines[secondProfileHeaderIndex + 1].indexOf('📊   2% left'), lines[1].indexOf('📊  95% left'));
  assert.equal(lines[secondProfileHeaderIndex + 1].indexOf('🔄 in'), lines[1].indexOf('🔄 in'));
  assert.equal(lines[secondProfileHeaderIndex + 1].indexOf('5.5d'), lines[1].indexOf('6.7d'));
  assert.equal(lines[secondProfileHeaderIndex + 1].indexOf('(79%)'), lines[1].indexOf('(95%)'));
  assert.equal(
    lines[secondProfileHeaderIndex + 1].indexOf('+77.3%') + '+77.3%'.length,
    lines[1].indexOf('+0.7%') + '+0.7%'.length,
  );
  assert.equal(lines[secondProfileHeaderIndex + 1].indexOf('Overuse'), lines[1].indexOf('Overuse'));
  assert.equal(panel.includes('    5H: 📊'), true);
  assert.equal(
    lines.slice(secondProfileHeaderIndex + 1).some((line) => /5H:/.test(line)),
    false,
  );
});
