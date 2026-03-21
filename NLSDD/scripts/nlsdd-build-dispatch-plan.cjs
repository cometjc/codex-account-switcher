#!/usr/bin/env node

const {resolveProjectRoot} = require('./nlsdd-lib.cjs');
const {runCoordinatorLoop} = require('./nlsdd-run-coordinator-loop.cjs');

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

function priorityForReviewAction(action) {
  switch (action) {
    case 'correction-loop':
      return 200;
    case 'spec-review':
      return 300;
    case 'quality-review':
      return 400;
    case 'coordinator-commit-needed':
      return 100;
    default:
      return 900;
  }
}

function buildDispatchPlan(projectRoot, execution, maxActive = 4, dryRun = false) {
  const autopilot = runCoordinatorLoop(projectRoot, execution, maxActive, dryRun);
  const queue = [];

  for (const entry of autopilot.commitIntake) {
    queue.push({
      kind: 'commit-intake',
      lane: entry.lane,
      priority: 100,
      summary: entry.proposedCommitTitle || `Commit intake for ${entry.lane}`,
      phase: entry.phase,
      worktreePath: entry.worktreePath,
      verification: entry.verification,
      message: [
        `Execution: ${entry.execution}`,
        `Lane: ${entry.lane}`,
        `Lane item: ${entry.item}`,
        `Proposed commit title: ${entry.proposedCommitTitle || 'n/a'}`,
        `Proposed commit body: ${entry.proposedCommitBody || 'n/a'}`,
        `Verification: ${entry.verification.join('; ') || 'n/a'}`,
        `Scope: ${entry.scope.join('; ') || 'n/a'}`,
        `Latest note: ${entry.note || 'n/a'}`,
      ].join('\n'),
    });
  }

  for (const entry of autopilot.reviewActions) {
    if (entry.action === 'coordinator-commit-needed') {
      continue;
    }
    queue.push({
      kind: 'review-action',
      lane: entry.lane,
      priority: priorityForReviewAction(entry.action),
      summary: `${entry.action} for ${entry.lane}`,
      phase: entry.phase,
      worktreePath: null,
      verification: [],
      message: entry.message,
    });
  }

  for (const [index, entry] of autopilot.launch.assignments.entries()) {
    queue.push({
      kind: 'launch-assignment',
      lane: entry.lane,
      priority: 501 + index,
      summary: `${entry.lane} -> ${entry.nextItem}`,
      phase: entry.promotedPhase,
      worktreePath: entry.worktreePath,
      verification: entry.verification,
      message: entry.message,
    });
  }

  queue.sort((left, right) => left.priority - right.priority || left.lane.localeCompare(right.lane));

  return {
    execution,
    maxActiveThreads: maxActive,
    dryRun,
    autopilot,
    insightSummary: autopilot.insightSummary,
    queue,
    idleSlots: autopilot.idleSlots,
  };
}

function renderDispatchPlan(result) {
  const lines = [
    `Execution: ${result.execution}`,
    `Max active threads: ${result.maxActiveThreads}`,
    `Dry run: ${result.dryRun ? 'yes' : 'no'}`,
    `Idle slots: ${result.idleSlots}`,
    `Action queue: ${result.queue.length}`,
    `Actionable insights: ${result.insightSummary.actionableCount}`,
    `Durable global learnings: ${result.insightSummary.durableLearningCount}`,
  ];

  if (result.queue.length === 0) {
    lines.push('Queue entries: none');
    return lines.join('\n');
  }

  lines.push('Queue entries:');
  for (const entry of result.queue) {
    lines.push(`- [${entry.priority}] ${entry.kind} · ${entry.lane} · ${entry.summary}`);
  }
  return lines.join('\n');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.execution) {
    throw new Error(
      'Usage: node NLSDD/scripts/nlsdd-build-dispatch-plan.cjs --execution <id> [--max-active <n>] [--dry-run] [--json]',
    );
  }

  const result = buildDispatchPlan(
    resolveProjectRoot(),
    args.execution,
    args.maxActive,
    args['dry-run'],
  );
  if (args.json) {
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    return;
  }
  process.stdout.write(`${renderDispatchPlan(result)}\n`);
}

if (require.main === module) {
  main();
}

module.exports = {
  parseArgs,
  priorityForReviewAction,
  buildDispatchPlan,
  renderDispatchPlan,
};
