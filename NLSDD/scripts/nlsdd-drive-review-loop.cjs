#!/usr/bin/env node

const fs = require('node:fs');
const {
  buildReviewLoopFromExecutor,
  hasExecutorDb,
} = require('./nlsdd-executor-lib.cjs');
const {
  loadLanePlan,
  loadScoreboardTable,
  loadLaneState,
  resolvePreferredScoreboardPath,
  resolveProjectRoot,
  summarizeExecutionInsights,
} = require('./nlsdd-lib.cjs');
const {prepareExecutionState} = require('./nlsdd-envelope.cjs');
const {composeMessage} = require('./nlsdd-compose-message.cjs');

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

function phaseForAction(row, laneState) {
  return (laneState?.phase || row['Effective phase'] || row.Phase || '').trim();
}

function buildContext(projectRoot, execution, row) {
  const lanePlan = loadLanePlan(projectRoot, execution, row.Lane);
  const laneState = loadLaneState(projectRoot, execution, row.Lane);
  return {
    row,
    lanePlan,
    laneState,
    phase: phaseForAction(row, laneState),
    item: row['Current item'],
    commit:
      laneState?.latestCommit ||
      String(row['Item commit'] || '')
        .replaceAll('`', '')
        .trim() ||
      'n/a',
    scope:
      lanePlan?.ownershipEntries?.join('; ') || 'Use the lane ownership family only.',
    verification:
      lanePlan?.verificationCommands?.join('; ') || 'Run the lane-local verification commands.',
  };
}

function correctionReason(context) {
  return (
    context.laneState?.note ||
    context.row['Latest event'] ||
    context.row.Notes ||
    'n/a'
  );
}

function buildAction(context) {
  switch (context.phase) {
    case 'spec-review-pending':
      return {
        action: 'spec-review',
        message: composeMessage({
          phase: 'spec-review',
          execution: context.row.Execution,
          lane: context.row.Lane,
          item: context.item,
          commit: context.commit,
        }),
      };
    case 'quality-review-pending':
      return {
        action: 'quality-review',
        message: composeMessage({
          phase: 'quality-review',
          execution: context.row.Execution,
          lane: context.row.Lane,
          item: context.item,
          commit: context.commit,
        }),
      };
    case 'correction':
      return {
        action: 'correction-loop',
        message: composeMessage({
          phase: 'correction-loop',
          execution: context.row.Execution,
          lane: context.row.Lane,
          item: context.item,
          commit: context.commit,
          scope: context.scope,
          verification: context.verification,
          files: context.scope,
          'fail-reason': correctionReason(context),
        }),
      };
    case 'coordinator-commit-pending':
    case 'READY_TO_COMMIT':
    case 'ready-to-commit':
      return {
        action: 'coordinator-commit-needed',
        message: [
          `Execution: ${context.row.Execution}`,
          `Lane: ${context.row.Lane}`,
          `Lane item: ${context.item}`,
          `Commit-ready handoff: ${context.commit}`,
          `Proposed commit title: ${context.laneState?.proposedCommitTitle || 'n/a'}`,
          `Proposed commit body: ${context.laneState?.proposedCommitBody || 'n/a'}`,
          `Scope: ${context.scope}`,
          `Verification: ${context.verification}`,
          `Latest note: ${context.laneState?.note || 'n/a'}`,
        ].join('\n'),
      };
    default:
      return null;
  }
}

function driveReviewLoop(projectRoot, execution, lane = null) {
  if (hasExecutorDb(projectRoot)) {
    return buildReviewLoopFromExecutor(projectRoot, execution, lane);
  }
  prepareExecutionState(projectRoot, execution);
  const scoreboardPath = resolvePreferredScoreboardPath(projectRoot);
  const scoreboardText = fs.readFileSync(scoreboardPath, 'utf8');
  const table = loadScoreboardTable(scoreboardText, scoreboardPath);
  const rows = table.objects.filter(
    (row) => row.Execution === execution && (!lane || row.Lane === lane),
  );

  const actions = rows
    .map((row) => {
      const context = buildContext(projectRoot, execution, row);
      const action = buildAction(context);
      if (!action) {
        return null;
      }
      return {
        execution,
        lane: row.Lane,
        phase: context.phase,
        item: context.item,
        commit: context.commit,
        action: action.action,
        message: action.message,
      };
    })
    .filter(Boolean);

  return {
    execution,
    lane: lane || null,
    actions,
    insightSummary: summarizeExecutionInsights(projectRoot, execution),
  };
}

function renderActions(result) {
  const actions = Array.isArray(result) ? result : result.actions;
  const insightSummary = Array.isArray(result) ? null : result.insightSummary;
  if (actions.length === 0 && (!insightSummary || insightSummary.actionableCount === 0)) {
    return 'Review actions: none';
  }

  const lines = [];
  if (actions.length === 0) {
    lines.push('Review actions: none');
  } else {
    lines.push(
      'Review actions:',
      ...actions.flatMap((entry) => [
        `- ${entry.lane} · ${entry.action} · ${entry.item}`,
        ...entry.message.split('\n').map((line) => `  ${line}`),
      ]),
    );
  }

  if (insightSummary && insightSummary.actionable.length > 0) {
    lines.push(`Actionable insights: ${insightSummary.actionableCount}`);
    for (const entry of insightSummary.actionable) {
      lines.push(`- [${entry.status}] ${entry.lane} · ${entry.kind} · ${entry.summary}`);
    }
  }

  if (insightSummary && insightSummary.durableLearnings.length > 0) {
    lines.push(`Durable global learnings: ${insightSummary.durableLearningCount}`);
    for (const entry of insightSummary.durableLearnings) {
      lines.push(`- [${entry.status}] ${entry.lane} · ${entry.kind} · ${entry.summary}`);
    }
  }

  return lines.join('\n');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.execution) {
    throw new Error(
      'Usage: node NLSDD/scripts/nlsdd-drive-review-loop.cjs --execution <id> [--lane <n>] [--json]',
    );
  }

  const result = driveReviewLoop(resolveProjectRoot(), args.execution, args.lane);
  if (args.json) {
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    return;
  }
  process.stdout.write(`${renderActions(result)}\n`);
}

if (require.main === module) {
  main();
}

module.exports = {
  parseArgs,
  phaseForAction,
  buildContext,
  correctionReason,
  buildAction,
  driveReviewLoop,
  renderActions,
};
