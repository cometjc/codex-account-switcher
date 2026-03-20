const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

async function loadLayoutModule() {
  return import(path.join(process.cwd(), 'dist/lib/root-table-layout.js'));
}

test('table headers own Time to reset, Usage Left, and Drift labels', async () => {
  const {renderRootHeaderBlock, renderWindowDetailLine} = await loadLayoutModule();
  const widths = {
    profile: 12,
    lastUpdate: 6,
    status: 13,
    bar: 30,
    timeToReset: 13,
    usageLeft: 10,
    drift: 12,
  };

  const header = renderRootHeaderBlock(widths);
  const weeklyLine = renderWindowDetailLine(
    {
      windowLabel: 'W:',
      bar: '[████░░░░░░░░░░░░░░░░░░░░░░░░]',
      timeToReset: '6.8d',
      usageLeft: '97% left',
      drift: '-1.6% Under',
      bottleneck: true,
    },
    widths,
  );

  assert.match(header, /Time to reset/);
  assert.match(header, /Usage Left/);
  assert.match(header, /Drift/);
  assert.doesNotMatch(weeklyLine, /Time to reset/);
  assert.doesNotMatch(weeklyLine, /Usage Left/);
  assert.doesNotMatch(weeklyLine, /Drift/);
});
