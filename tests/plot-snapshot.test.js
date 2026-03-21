const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

function loadPlotModule() {
  return require(path.join(process.cwd(), 'dist/lib/plot'));
}

test('plot snapshot barrel exports the contract builders', () => {
  const plot = loadPlotModule();

  assert.deepEqual(Object.keys(plot).sort(), ['buildPlotSnapshot', 'serializePlotSnapshot']);
  assert.equal(typeof plot.buildPlotSnapshot, 'function');
  assert.equal(typeof plot.serializePlotSnapshot, 'function');
});

test('buildPlotSnapshot normalizes a representative snapshot shape', () => {
  const {buildPlotSnapshot} = loadPlotModule();

  const snapshot = buildPlotSnapshot({
    generatedAt: 1234567890,
    profiles: [
      {
        id: 'beta',
        name: 'Beta',
        isCurrent: true,
        usage: null,
        sevenDayWindow: {startAt: null, endAt: 20},
        sevenDayPoints: [
          {offsetSeconds: 120, usedPercent: 25},
          {offsetSeconds: 30, usedPercent: 10},
        ],
        fiveHourWindow: {startAt: 1, endAt: null},
        summaryLabels: {usageLeft: 'Remaining usage'},
      },
      {
        id: 'alpha',
        name: 'Alpha',
        usage: null,
        sevenDayWindow: {startAt: 5, endAt: 15},
        sevenDayPoints: [],
        fiveHourWindow: {startAt: null, endAt: null},
        fiveHourBand: {available: false, reason: 'insufficient-samples'},
      },
    ],
  });

  assert.deepEqual(snapshot, {
    schemaVersion: 1,
    generatedAt: 1234567890,
    currentProfileId: 'beta',
    profiles: [
      {
        id: 'beta',
        name: 'Beta',
        isCurrent: true,
        usage: null,
        sevenDayWindow: {startAt: null, endAt: 20},
        sevenDayPoints: [
          {offsetSeconds: 30, usedPercent: 10},
          {offsetSeconds: 120, usedPercent: 25},
        ],
        fiveHourWindow: {startAt: 1, endAt: null},
        fiveHourBand: {
          available: false,
          lowerY: null,
          upperY: null,
          bandHeight: null,
          delta7dPercent: null,
          delta5hPercent: null,
          reason: 'band-not-provided',
        },
        summaryLabels: {
          timeToReset: 'Time to reset',
          usageLeft: 'Remaining usage',
          drift: 'Drift',
          pacingStatus: 'Pacing Status',
        },
      },
      {
        id: 'alpha',
        name: 'Alpha',
        isCurrent: false,
        usage: null,
        sevenDayWindow: {startAt: 5, endAt: 15},
        sevenDayPoints: [],
        fiveHourWindow: {startAt: null, endAt: null},
        fiveHourBand: {
          available: false,
          lowerY: null,
          upperY: null,
          bandHeight: null,
          delta7dPercent: null,
          delta5hPercent: null,
          reason: 'insufficient-samples',
        },
        summaryLabels: {
          timeToReset: 'Time to reset',
          usageLeft: 'Usage Left',
          drift: 'Drift',
          pacingStatus: 'Pacing Status',
        },
      },
    ],
  });
});

test('serializePlotSnapshot emits stable pretty JSON with sorted keys', () => {
  const {buildPlotSnapshot, serializePlotSnapshot} = loadPlotModule();

  const snapshot = buildPlotSnapshot({
    generatedAt: 1234567890,
    profiles: [
      {
        id: 'beta',
        name: 'Beta',
        isCurrent: true,
        usage: null,
        sevenDayWindow: {startAt: null, endAt: 20},
        sevenDayPoints: [
          {offsetSeconds: 120, usedPercent: 25},
          {offsetSeconds: 30, usedPercent: 10},
        ],
        fiveHourWindow: {startAt: 1, endAt: null},
        summaryLabels: {usageLeft: 'Remaining usage'},
      },
      {
        id: 'alpha',
        name: 'Alpha',
        usage: null,
        sevenDayWindow: {startAt: 5, endAt: 15},
        sevenDayPoints: [],
        fiveHourWindow: {startAt: null, endAt: null},
        fiveHourBand: {available: false, reason: 'insufficient-samples'},
      },
    ],
  });

  assert.equal(
    serializePlotSnapshot(snapshot),
    [
      '{',
      '  "currentProfileId": "beta",',
      '  "generatedAt": 1234567890,',
      '  "profiles": [',
      '    {',
      '      "fiveHourBand": {',
      '        "available": false,',
      '        "bandHeight": null,',
      '        "delta5hPercent": null,',
      '        "delta7dPercent": null,',
      '        "lowerY": null,',
      '        "reason": "band-not-provided",',
      '        "upperY": null',
      '      },',
      '      "fiveHourWindow": {',
      '        "endAt": null,',
      '        "startAt": 1',
      '      },',
      '      "id": "beta",',
      '      "isCurrent": true,',
      '      "name": "Beta",',
      '      "sevenDayPoints": [',
      '        {',
      '          "offsetSeconds": 30,',
      '          "usedPercent": 10',
      '        },',
      '        {',
      '          "offsetSeconds": 120,',
      '          "usedPercent": 25',
      '        }',
      '      ],',
      '      "sevenDayWindow": {',
      '        "endAt": 20,',
      '        "startAt": null',
      '      },',
      '      "summaryLabels": {',
      '        "drift": "Drift",',
      '        "pacingStatus": "Pacing Status",',
      '        "timeToReset": "Time to reset",',
      '        "usageLeft": "Remaining usage"',
      '      },',
      '      "usage": null',
      '    },',
      '    {',
      '      "fiveHourBand": {',
      '        "available": false,',
      '        "bandHeight": null,',
      '        "delta5hPercent": null,',
      '        "delta7dPercent": null,',
      '        "lowerY": null,',
      '        "reason": "insufficient-samples",',
      '        "upperY": null',
      '      },',
      '      "fiveHourWindow": {',
      '        "endAt": null,',
      '        "startAt": null',
      '      },',
      '      "id": "alpha",',
      '      "isCurrent": false,',
      '      "name": "Alpha",',
      '      "sevenDayPoints": [],',
      '      "sevenDayWindow": {',
      '        "endAt": 15,',
      '        "startAt": 5',
      '      },',
      '      "summaryLabels": {',
      '        "drift": "Drift",',
      '        "pacingStatus": "Pacing Status",',
      '        "timeToReset": "Time to reset",',
      '        "usageLeft": "Usage Left"',
      '      },',
      '      "usage": null',
      '    }',
      '  ],',
      '  "schemaVersion": 1',
      '}',
      '',
    ].join('\n'),
  );
});
