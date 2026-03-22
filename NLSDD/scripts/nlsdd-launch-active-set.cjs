#!/usr/bin/env node

const {composeMessage} = require('./nlsdd-compose-message.cjs');
const {loadLanePlan, resolveProjectRoot} = require('./nlsdd-lib.cjs');
const {
  buildLaunchFromExecutor,
  hasExecutorDb,
} = require('./nlsdd-executor-lib.cjs');
const {runCycle} = require('./nlsdd-run-cycle.cjs');

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

function scopeText(lanePlan) {
  if (!lanePlan || !lanePlan.ownershipEntries || lanePlan.ownershipEntries.length === 0) {
    return 'Use the lane ownership family only.';
  }
  return lanePlan.ownershipEntries.join('; ');
}

function verificationText(lanePlan) {
  if (!lanePlan || !lanePlan.verificationCommands || lanePlan.verificationCommands.length === 0) {
    return 'Run the lane-local verification commands.';
  }
  return lanePlan.verificationCommands.join('; ');
}

function buildAssignmentBundle(projectRoot, execution, promotedEntry) {
  const lanePlan = loadLanePlan(projectRoot, execution, promotedEntry.lane);
  const message = composeMessage({
    phase: 'implementer-assignment',
    execution,
    lane: promotedEntry.lane,
    item: promotedEntry.nextItem,
    scope: scopeText(lanePlan),
    verification: verificationText(lanePlan),
  });

  return {
    slot: promotedEntry.slot,
    lane: promotedEntry.lane,
    currentPhase: promotedEntry.from,
    promotedPhase: promotedEntry.to,
    nextItem: promotedEntry.nextItem,
    nextItemSection: promotedEntry.nextItemSection,
    scope: scopeText(lanePlan),
    verification: lanePlan?.verificationCommands || [],
    worktreePath: lanePlan?.worktreePath || null,
    message,
  };
}

function launchActiveSet(projectRoot, execution, maxActive = 4, dryRun = false) {
  if (hasExecutorDb(projectRoot)) {
    return buildLaunchFromExecutor(projectRoot, execution, maxActive, dryRun);
  }
  const cycle = runCycle(projectRoot, execution, maxActive, dryRun);
  const assignments = cycle.promoted.map((entry) =>
    buildAssignmentBundle(projectRoot, execution, entry),
  );
  return {
    ...cycle,
    assignments,
  };
}

function renderLaunchResult(result) {
  const lines = [
    `Execution: ${result.execution}`,
    `Max active threads: ${result.maxActiveThreads}`,
    `Dry run: ${result.dryRun ? 'yes' : 'no'}`,
    `Completed lanes: ${result.completedLanes.length > 0 ? result.completedLanes.join(', ') : 'none'}`,
    `Promoted lanes: ${result.promoted.length}`,
    `Idle slots: ${result.idleSlots}`,
  ];

  if (result.noDispatchReason) {
    lines.push(`No dispatch reason: ${result.noDispatchReason}`);
  }

  if (result.assignments.length === 0) {
    lines.push('Assignments: none');
    return lines.join('\n');
  }

  lines.push('Assignments:');
  for (const assignment of result.assignments) {
    lines.push(`- Slot ${assignment.slot} · ${assignment.lane} · ${assignment.nextItem}`);
    lines.push(`  Worktree: ${assignment.worktreePath || 'n/a'}`);
    lines.push(`  Verification: ${assignment.verification.length > 0 ? assignment.verification.join(' ; ') : 'n/a'}`);
    lines.push('  Message:');
    lines.push(...assignment.message.split('\n').map((line) => `    ${line}`));
  }

  return lines.join('\n');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.execution) {
    throw new Error(
      'Usage: node NLSDD/scripts/nlsdd-launch-active-set.cjs --execution <id> [--max-active <n>] [--dry-run] [--json]',
    );
  }

  const result = launchActiveSet(resolveProjectRoot(), args.execution, args.maxActive, args['dry-run']);
  if (args.json) {
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    return;
  }
  process.stdout.write(`${renderLaunchResult(result)}\n`);
}

if (require.main === module) {
  main();
}

module.exports = {
  parseArgs,
  scopeText,
  verificationText,
  buildAssignmentBundle,
  launchActiveSet,
  renderLaunchResult,
};
