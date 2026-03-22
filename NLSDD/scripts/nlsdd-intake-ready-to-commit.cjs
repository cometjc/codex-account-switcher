#!/usr/bin/env node

const {
  loadLanePlan,
  loadLaneState,
  loadPreferredScoreboardTable,
  resolveProjectRoot,
} = require('./nlsdd-lib.cjs');
const {prepareExecutionState} = require('./nlsdd-envelope.cjs');

function parseArgs(argv) {
  const args = {};
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value === '--execution') {
      args.execution = argv[index + 1];
      index += 1;
    } else if (value === '--lane') {
      args.lane = `Lane ${argv[index + 1]}`.replace(/^Lane\s+Lane\s+/, 'Lane ');
      index += 1;
    } else if (value === '--json') {
      args.json = true;
    }
  }
  return args;
}

function phaseForIntake(row, laneState) {
  return (laneState?.phase || row['Effective phase'] || row.Phase || '').trim();
}

function buildCommitIntake(projectRoot, execution, row) {
  const laneState = loadLaneState(projectRoot, execution, row.Lane);
  const phase = phaseForIntake(row, laneState);
  if (!['coordinator-commit-pending', 'READY_TO_COMMIT', 'ready-to-commit'].includes(phase)) {
    return null;
  }

  const lanePlan = loadLanePlan(projectRoot, execution, row.Lane);
  return {
    execution,
    lane: row.Lane,
    phase,
    item: row['Current item'],
    commit: laneState?.latestCommit || null,
    proposedCommitTitle: laneState?.proposedCommitTitle || null,
    proposedCommitBody: laneState?.proposedCommitBody || null,
    scope: lanePlan?.ownershipEntries || [],
    verification: laneState?.lastVerification || lanePlan?.verificationCommands || [],
    note: laneState?.note || row['Latest event'] || row.Notes || 'n/a',
    nextExpectedPhase: laneState?.expectedNextPhase || null,
    worktreePath: lanePlan?.worktreePath || null,
  };
}

function intakeReadyToCommit(projectRoot, execution, lane = null) {
  prepareExecutionState(projectRoot, execution);
  const table = loadPreferredScoreboardTable(projectRoot);
  const rows = table.objects.filter(
    (row) => row.Execution === execution && (!lane || row.Lane === lane),
  );
  return rows.map((row) => buildCommitIntake(projectRoot, execution, row)).filter(Boolean);
}

function intakeReadyToCommitWithContext(projectRoot, execution, lane = null) {
  try {
    const entries = intakeReadyToCommit(projectRoot, execution, lane);
    const table = loadPreferredScoreboardTable(projectRoot);
    return {
      entries,
      degradedSurfaces: table.scoreboardLoad?.degraded
        ? [
            {
              surface: 'scoreboard',
              source: table.scoreboardLoad.source,
              path: table.scoreboardLoad.path,
              errors: table.scoreboardLoad.errors || [],
            },
          ]
        : [],
    };
  } catch (error) {
    return {
      entries: [],
      degradedSurfaces: [
        {
          surface: 'commit-intake',
          source: 'unreadable-scoreboard',
          path: null,
          errors: [{path: null, message: error.message}],
        },
      ],
    };
  }
}

function renderIntake(entries) {
  if (entries.length === 0) {
    return 'Commit intake: none';
  }
  return [
    'Commit intake:',
    ...entries.flatMap((entry) => [
      `- ${entry.lane} · ${entry.item}`,
      `  Proposed title: ${entry.proposedCommitTitle || 'n/a'}`,
      `  Proposed body: ${entry.proposedCommitBody || 'n/a'}`,
      `  Verification: ${entry.verification.length > 0 ? entry.verification.join(' ; ') : 'n/a'}`,
      `  Scope: ${entry.scope.length > 0 ? entry.scope.join(' ; ') : 'n/a'}`,
      `  Worktree: ${entry.worktreePath || 'n/a'}`,
      `  Note: ${entry.note || 'n/a'}`,
    ]),
  ].join('\n');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.execution) {
    throw new Error(
      'Usage: node NLSDD/scripts/nlsdd-intake-ready-to-commit.cjs --execution <id> [--lane <n>] [--json]',
    );
  }

  const result = intakeReadyToCommit(resolveProjectRoot(), args.execution, args.lane);
  if (args.json) {
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    return;
  }
  process.stdout.write(`${renderIntake(result)}\n`);
}

if (require.main === module) {
  main();
}

module.exports = {
  parseArgs,
  phaseForIntake,
  buildCommitIntake,
  intakeReadyToCommit,
  intakeReadyToCommitWithContext,
  renderIntake,
};
