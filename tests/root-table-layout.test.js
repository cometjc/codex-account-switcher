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

test('detail headers align with detail values on the same columns', async () => {
  const {renderWindowDetailLine} = await loadLayoutModule();
  const widths = {
    profile: 12,
    lastUpdate: 6,
    status: 13,
    bar: 30,
    timeToReset: 13,
    usageLeft: 10,
    drift: 12,
  };

  const detailHeader = renderWindowDetailLine(
    {
      windowLabel: '',
      bar: '',
      timeToReset: 'Time to reset',
      usageLeft: 'Usage Left',
      drift: 'Drift',
      bottleneck: false,
    },
    widths,
  );
  const weeklyLine = renderWindowDetailLine(
    {
      windowLabel: 'W:',
      bar: '[████░░░░░░░░░░░░░░░░░░░░░░░░]',
      timeToReset: '6.8d',
      usageLeft: '97% left',
      drift: '-1.6% Under',
      bottleneck: false,
    },
    widths,
  );

  const headerTimeEnd = detailHeader.indexOf('Time to reset') + 'Time to reset'.length;
  const rowTimeEnd = weeklyLine.indexOf('6.8d') + '6.8d'.length;
  const headerUsageEnd = detailHeader.indexOf('Usage Left') + 'Usage Left'.length;
  const rowUsageEnd = weeklyLine.indexOf('97% left') + '97% left'.length;
  const headerDriftStart = detailHeader.indexOf('Drift');
  const rowDriftStart = weeklyLine.indexOf('-1.6% Under');

  assert.equal(headerTimeEnd, rowTimeEnd);
  assert.equal(headerUsageEnd, rowUsageEnd);
  assert.equal(headerDriftStart, rowDriftStart);
});
