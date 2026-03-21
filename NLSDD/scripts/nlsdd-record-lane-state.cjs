#!/usr/bin/env node

const {
  findNextRefillItem,
  loadLanePlan,
  loadLaneState,
  loadScoreboardTable,
  resolvePreferredScoreboardPath,
  resolveProjectRoot,
} = require('./nlsdd-lib.cjs');
const {recordEnvelope} = require('./nlsdd-envelope.cjs');

function parseArgs(argv) {
  const args = {verification: []};
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value === '--execution') {
      args.execution = argv[index + 1];
      index += 1;
    } else if (value === '--lane') {
      args.lane = `Lane ${argv[index + 1]}`.replace(/^Lane\s+Lane\s+/, 'Lane ');
      index += 1;
    } else if (value === '--phase') {
      args.phase = argv[index + 1];
      index += 1;
    } else if (value === '--expected-next-phase') {
      args['expected-next-phase'] = argv[index + 1];
      index += 1;
    } else if (value === '--commit') {
      args.commit = argv[index + 1];
      index += 1;
    } else if (value === '--commit-title') {
      args['commit-title'] = argv[index + 1];
      index += 1;
    } else if (value === '--commit-body') {
      args['commit-body'] = argv[index + 1];
      index += 1;
    } else if (value === '--reviewer') {
      args.reviewer = argv[index + 1];
      index += 1;
    } else if (value === '--correction-count') {
      args['correction-count'] = argv[index + 1];
      index += 1;
    } else if (value === '--verification') {
      args.verification.push(argv[index + 1]);
      index += 1;
    } else if (value === '--blocked-by') {
      args['blocked-by'] = argv[index + 1];
      index += 1;
    } else if (value === '--note') {
      args.note = argv[index + 1];
      index += 1;
    } else if (value === '--updated-at') {
      args['updated-at'] = argv[index + 1];
      index += 1;
    }
  }
  return args;
}

function recordLaneState(projectRoot, args) {
  if (!args.execution || !args.lane || !args.phase) {
    throw new Error(
      'execution, lane, and phase are required to record NLSDD lane state',
    );
  }
  const scoreboardPath = resolvePreferredScoreboardPath(projectRoot);
  const scoreboardText = require('node:fs').existsSync(scoreboardPath)
    ? require('node:fs').readFileSync(scoreboardPath, 'utf8')
    : '';
  const table = scoreboardText
    ? loadScoreboardTable(scoreboardText, scoreboardPath)
    : {objects: []};
  const row = table.objects.find(
    (candidate) => candidate.Execution === args.execution && candidate.Lane === args.lane,
  );
  const previous = loadLaneState(projectRoot, args.execution, args.lane);
  const lanePlan = loadLanePlan(projectRoot, args.execution, args.lane);
  const fallbackNextItem = findNextRefillItem(projectRoot, args.execution, args.lane);
  const currentItem =
    args.item ||
    args['current-item'] ||
    previous?.currentItem ||
    row?.['Current item'] ||
    fallbackNextItem?.text ||
    null;
  const nextRefillTarget =
    args['next-refill-target'] ||
    previous?.nextRefillTarget ||
    row?.['Next refill target'] ||
    null;
  const eventType = (() => {
    if (String(args.phase).toLowerCase() === 'ready-to-commit' || String(args.phase) === 'READY_TO_COMMIT') {
      return 'ready-to-commit';
    }
    if (String(args.phase).toLowerCase() === 'blocked') {
      return 'blocked';
    }
    if (String(args.phase).toLowerCase() === 'parked') {
      return 'parked';
    }
    if (String(args.reviewer || '').toLowerCase() === 'pass') {
      return 'pass';
    }
    if (String(args.reviewer || '').toLowerCase() === 'fail') {
      return 'fail';
    }
    return 'state-update';
  })();
  const normalizedExpectedNextPhase =
    args['expected-next-phase'] !== undefined
      ? args['expected-next-phase']
      : ['parked', 'blocked'].includes(String(args.phase).toLowerCase())
        ? null
        : previous?.expectedNextPhase || null;
  const result = recordEnvelope(projectRoot, {
    execution: args.execution,
    lane: args.lane,
    role: 'coordinator',
    eventType,
    phaseBefore: previous?.phase || row?.['Effective phase'] || row?.Phase || null,
    phaseAfter: args.phase,
    currentItem,
    nextRefillTarget,
    relatedCommit: args.commit || previous?.latestCommit || null,
    verification: args.verification || previous?.lastVerification || lanePlan?.verificationCommands || [],
    summary: args.note || `Lane state updated for ${args.lane}`,
    detail: args.note || null,
    nextExpectedPhase: normalizedExpectedNextPhase,
    blockedBy: args['blocked-by'] || null,
    'commit-title': args['commit-title'] || null,
    'commit-body': args['commit-body'] || null,
    'correction-count': Number(args['correction-count'] || previous?.correctionCount || 0),
    timestamp: args['updated-at'] || new Date().toISOString(),
  });
  return require('./nlsdd-lib.cjs').laneStatePath(projectRoot, args.execution, args.lane) || result.filePath;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.execution || !args.lane || !args.phase) {
    throw new Error(
      'Usage: node NLSDD/scripts/nlsdd-record-lane-state.cjs --execution <id> --lane <n> --phase <phase> [--expected-next-phase <phase>] [--commit <sha>] [--commit-title <title>] [--commit-body <body>] [--reviewer <result>] [--correction-count <n>] [--verification <cmd>] [--blocked-by <reason>] [--note <text>] [--updated-at <iso>]',
    );
  }
  const filePath = recordLaneState(resolveProjectRoot(), args);
  process.stdout.write(`${filePath}\n`);
}

if (require.main === module) {
  main();
}

module.exports = {
  parseArgs,
  recordLaneState,
};
