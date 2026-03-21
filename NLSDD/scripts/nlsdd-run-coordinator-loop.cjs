#!/usr/bin/env node

const {resolveProjectRoot, summarizeExecutionInsights} = require('./nlsdd-lib.cjs');
const {launchActiveSet} = require('./nlsdd-launch-active-set.cjs');
const {driveReviewLoop} = require('./nlsdd-drive-review-loop.cjs');
const {intakeReadyToCommit} = require('./nlsdd-intake-ready-to-commit.cjs');

function parseArgs(argv) {
  const args = {maxActive: 4};
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value === '--execution') {
      args.execution = argv[index + 1];
      index += 1;
    } else if (value === '--max-active') {
      args.maxActive = Number(argv[index + 1]);
      index += 1;
    } else if (value === '--json') {
      args.json = true;
    } else if (value === '--dry-run') {
      args['dry-run'] = true;
    }
  }
  return args;
}

function runCoordinatorLoop(projectRoot, execution, maxActive = 4, dryRun = false) {
  const launch = launchActiveSet(projectRoot, execution, maxActive, dryRun);
  const reviewResult = driveReviewLoop(projectRoot, execution);
  const reviewActions = reviewResult.actions;
  const commitIntake = intakeReadyToCommit(projectRoot, execution);
  const insightSummary = reviewResult.insightSummary || summarizeExecutionInsights(projectRoot, execution);
  return {
    execution,
    maxActiveThreads: maxActive,
    dryRun,
    launch,
    reviewActions,
    commitIntake,
    insightSummary,
    idleSlots: launch.idleSlots,
    completedLanes: launch.completedLanes,
    promotedLanes: launch.promoted.map((entry) => entry.lane),
    reviewLaneCount: reviewActions.length,
    commitLaneCount: commitIntake.length,
    noDispatchReason: launch.noDispatchReason,
  };
}

function renderCoordinatorLoop(result) {
  const lines = [
    `Execution: ${result.execution}`,
    `Max active threads: ${result.maxActiveThreads}`,
    `Dry run: ${result.dryRun ? 'yes' : 'no'}`,
    `Completed lanes: ${result.completedLanes.length > 0 ? result.completedLanes.join(', ') : 'none'}`,
    `Promoted lanes: ${result.promotedLanes.length > 0 ? result.promotedLanes.join(', ') : 'none'}`,
    `Idle slots: ${result.idleSlots}`,
    `Review actions: ${result.reviewLaneCount}`,
    `Commit intakes: ${result.commitLaneCount}`,
    `Actionable insights: ${result.insightSummary.actionableCount}`,
    `Durable global learnings: ${result.insightSummary.durableLearningCount}`,
  ];

  if (result.noDispatchReason) {
    lines.push(`No dispatch reason: ${result.noDispatchReason}`);
  }

  if (result.launch.assignments.length > 0) {
    lines.push('Assignments:');
    for (const assignment of result.launch.assignments) {
      lines.push(`- ${assignment.lane}: ${assignment.nextItem}`);
    }
  }

  if (result.reviewActions.length > 0) {
    lines.push('Review lanes:');
    for (const action of result.reviewActions) {
      lines.push(`- ${action.lane}: ${action.action}`);
    }
  }

  if (result.commitIntake.length > 0) {
    lines.push('Commit-ready lanes:');
    for (const entry of result.commitIntake) {
      lines.push(`- ${entry.lane}: ${entry.proposedCommitTitle || 'n/a'}`);
    }
  }

  if (result.insightSummary.actionable.length > 0) {
    lines.push('Execution insights:');
    for (const entry of result.insightSummary.actionable) {
      lines.push(`- [${entry.status}] ${entry.lane}: ${entry.summary}`);
    }
  }

  if (result.insightSummary.durableLearnings.length > 0) {
    lines.push('Durable global learnings:');
    for (const entry of result.insightSummary.durableLearnings) {
      lines.push(`- [${entry.status}] ${entry.lane}: ${entry.summary}`);
    }
  }

  return lines.join('\n');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.execution) {
    throw new Error(
      'Usage: node NLSDD/scripts/nlsdd-run-coordinator-loop.cjs --execution <id> [--max-active <n>] [--dry-run] [--json]',
    );
  }
  const result = runCoordinatorLoop(
    resolveProjectRoot(),
    args.execution,
    args.maxActive,
    args['dry-run'],
  );
  if (args.json) {
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    return;
  }
  process.stdout.write(`${renderCoordinatorLoop(result)}\n`);
}

if (require.main === module) {
  main();
}

module.exports = {
  parseArgs,
  runCoordinatorLoop,
  renderCoordinatorLoop,
};
