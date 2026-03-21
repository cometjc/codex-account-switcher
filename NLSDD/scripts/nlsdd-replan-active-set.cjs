#!/usr/bin/env node

const fs = require('node:fs');
const {
  joinRow,
  loadLaneState,
  loadScoreboardTable,
  resolveProjectRoot,
  resolveScoreboardPath,
} = require('./nlsdd-lib.cjs');
const {recordLaneState} = require('./nlsdd-record-lane-state.cjs');
const {updateScoreboard} = require('./nlsdd-refresh-scoreboard.cjs');

function normalizeLaneName(value) {
  if (!value) {
    return null;
  }
  return `Lane ${String(value).trim()}`
    .replace(/^Lane\s+Lane\s+/i, 'Lane ')
    .replace(/^Lane\s+/i, 'Lane ')
    .trim();
}

function parseLaneList(value) {
  if (!value) {
    return [];
  }
  if (Array.isArray(value)) {
    return value.map(normalizeLaneName).filter(Boolean);
  }
  return String(value)
    .split(',')
    .map((entry) => normalizeLaneName(entry))
    .filter(Boolean);
}

function parseArgs(argv) {
  const args = {};
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (!value.startsWith('--')) {
      continue;
    }
    const key = value.slice(2);
    const next = argv[index + 1];
    if (!next || next.startsWith('--')) {
      args[key] = true;
      continue;
    }
    args[key] = next;
    index += 1;
  }
  if (args.active) {
    args.active = parseLaneList(args.active);
  }
  if (args.parked || args.park) {
    args.parked = parseLaneList(args.parked || args.park);
  }
  return args;
}

function extractCodeValues(cell) {
  const matches = [...String(cell || '').matchAll(/`([^`]+)`/g)].map((match) => match[1].trim());
  if (matches.length > 0) {
    return matches;
  }
  const plain = String(cell || '').trim();
  if (!plain || plain === 'none' || plain === 'n/a') {
    return [];
  }
  return plain
    .split(';')
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function extractSingleCodeValue(cell) {
  return extractCodeValues(cell)[0] || null;
}

function normalizeBlockedBy(cell) {
  const value = String(cell || '').trim();
  if (!value || value === 'none' || value === 'n/a') {
    return null;
  }
  return value;
}

function updateTrackedScoreboardPhases(projectRoot, execution, lanePhaseMap) {
  const scoreboardPath = resolveScoreboardPath(projectRoot);
  const scoreboardText = fs.readFileSync(scoreboardPath, 'utf8');
  const table = loadScoreboardTable(scoreboardText, scoreboardPath);

  for (const row of table.objects) {
    if (row.Execution !== execution) {
      continue;
    }
    if (lanePhaseMap.has(row.Lane)) {
      row.Phase = lanePhaseMap.get(row.Lane);
    }
  }

  const renderedRows = table.objects.map((row) =>
    joinRow(table.columns.map((column) => row[column] || '')),
  );
  const nextLines = [
    ...table.lines.slice(0, table.headerIndex),
    table.header,
    table.separator,
    ...renderedRows,
    ...table.lines.slice(table.endIndex),
  ];
  fs.writeFileSync(scoreboardPath, nextLines.join('\n'), 'utf8');
  return table.objects.filter((row) => row.Execution === execution);
}

function replanActiveSet(projectRoot, args) {
  if (!args.execution) {
    throw new Error('execution is required');
  }

  const active = parseLaneList(args.active);
  const parked = parseLaneList(args.parked);
  const lanePhaseMap = new Map();
  for (const lane of active) {
    lanePhaseMap.set(lane, 'queued');
  }
  for (const lane of parked) {
    lanePhaseMap.set(lane, 'parked');
  }

  const executionRows = updateTrackedScoreboardPhases(
    projectRoot,
    args.execution,
    lanePhaseMap,
  );
  const rowByLane = new Map(executionRows.map((row) => [row.Lane, row]));
  const updatedAt = args['updated-at'];
  const note = args.note || 'active-set replan';

  for (const lane of [...active, ...parked]) {
    const row = rowByLane.get(lane);
    if (!row) {
      continue;
    }
    const existingState = loadLaneState(projectRoot, args.execution, lane);
    const phase = lanePhaseMap.get(lane);
    recordLaneState(projectRoot, {
      execution: args.execution,
      lane,
      phase,
      'expected-next-phase': phase === 'queued' ? 'implementing' : null,
      commit: extractSingleCodeValue(row['Item commit']) || existingState?.latestCommit || null,
      reviewer: null,
      'correction-count': existingState?.correctionCount || 0,
      'verification': extractCodeValues(row['Last verification']),
      'blocked-by': normalizeBlockedBy(row['Blocked by']),
      note,
      'updated-at': updatedAt,
    });
  }

  updateScoreboard();
  return {
    execution: args.execution,
    active,
    parked,
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.execution || (!args.active?.length && !args.parked?.length)) {
    throw new Error(
      'Usage: node NLSDD/scripts/nlsdd-replan-active-set.cjs --execution <id> [--active 2,3,4] [--parked 1] [--note <text>] [--updated-at <iso>]',
    );
  }
  const result = replanActiveSet(resolveProjectRoot(), args);
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
}

if (require.main === module) {
  main();
}

module.exports = {
  normalizeLaneName,
  parseLaneList,
  parseArgs,
  extractCodeValues,
  replanActiveSet,
};
