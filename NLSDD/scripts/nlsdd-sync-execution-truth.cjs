#!/usr/bin/env node

const {
  computeExecutionSchedule,
  loadLaneState,
  resolveProjectRoot,
} = require('./nlsdd-lib.cjs');
const {prepareExecutionState} = require('./nlsdd-envelope.cjs');
const {recordLaneState} = require('./nlsdd-record-lane-state.cjs');
const {updateScoreboard} = require('./nlsdd-refresh-scoreboard.cjs');

function parseArgs(argv) {
  const args = {};
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value === '--execution') {
      args.execution = argv[index + 1];
      index += 1;
    } else if (value === '--dry-run') {
      args['dry-run'] = true;
    } else if (value === '--json') {
      args.json = true;
    }
  }
  return args;
}

function syncExecutionTruth(projectRoot, execution, options = {}) {
  if (!execution) {
    throw new Error('execution is required');
  }

  const dryRun = Boolean(options['dry-run'] || options.dryRun);
  prepareExecutionState(projectRoot, execution);
  const schedule = computeExecutionSchedule(projectRoot, execution, 4);
  const reconciledLanes = [];

  for (const row of schedule.staleRows || []) {
    const laneState = loadLaneState(projectRoot, execution, row.Lane);
    const note =
      `Execution-truth sync reconciled ${row.Lane} from stale implementing to parked ` +
      `because the lane worktree is clean at ${laneState?.latestCommit || 'n/a'}.`;

    if (!dryRun) {
      recordLaneState(projectRoot, {
        execution,
        lane: row.Lane,
        phase: 'parked',
        'expected-next-phase': null,
        commit: laneState?.latestCommit || null,
        reviewer: laneState?.lastReviewerResult || null,
        'correction-count': laneState?.correctionCount || 0,
        verification: laneState?.lastVerification || [],
        'blocked-by': null,
        note,
      });
    }

    reconciledLanes.push({
      lane: row.Lane,
      fromPhase: row.schedulingPhase,
      toPhase: 'parked',
      latestCommit: laneState?.latestCommit || null,
      reason: row.staleImplementing?.kind || 'stale-implementing',
    });
  }

  if (!dryRun && reconciledLanes.length > 0) {
    updateScoreboard(projectRoot);
  }

  return {
    execution,
    dryRun,
    reconciledLanes,
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.execution) {
    throw new Error(
      'Usage: node NLSDD/scripts/nlsdd-sync-execution-truth.cjs --execution <id> [--dry-run] [--json]',
    );
  }

  const result = syncExecutionTruth(resolveProjectRoot(), args.execution, args);
  if (args.json) {
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    return;
  }
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
}

if (require.main === module) {
  main();
}

module.exports = {
  parseArgs,
  syncExecutionTruth,
};
