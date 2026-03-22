#!/usr/bin/env node

const {
  computeExecutionSchedule,
  loadLaneState,
  resolveProjectRoot,
} = require('./nlsdd-lib.cjs');
const {
  buildCycleFromExecutor: buildExecutorCycle,
  hasExecutorDb: hasExecutorDbState,
} = require('./nlsdd-executor-lib.cjs');
const {prepareExecutionState} = require('./nlsdd-envelope.cjs');
const {recordLaneState} = require('./nlsdd-record-lane-state.cjs');
const {updateScoreboard} = require('./nlsdd-refresh-scoreboard.cjs');

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

function normalizeCellPhase(value) {
  const phase = String(value || '').trim();
  return phase || null;
}

function nextExpectedPhaseFor(phase) {
  if (['queued', 'lane-ready', 'refill-ready'].includes(phase)) {
    return 'implementing';
  }
  if (phase === 'implementing') {
    return 'spec-review-pending';
  }
  return null;
}

function reconcileStaleRows(projectRoot, execution, schedule, dryRun = false) {
  const reconciled = [];

  for (const row of schedule.staleRows || []) {
    const trackedPhase = normalizeCellPhase(row.Phase);
    if (!trackedPhase || trackedPhase === 'implementing') {
      continue;
    }

    const existingState = loadLaneState(projectRoot, execution, row.Lane);
    const nextPhase = nextExpectedPhaseFor(trackedPhase);
    const note = `Cycle reconciled stale implementing lane back to tracked phase ${trackedPhase}.`;

    if (!dryRun) {
      recordLaneState(projectRoot, {
        execution,
        lane: row.Lane,
        phase: trackedPhase,
        'expected-next-phase': nextPhase,
        commit: existingState?.latestCommit || null,
        reviewer: existingState?.lastReviewerResult || null,
        'correction-count': existingState?.correctionCount || 0,
        verification: existingState?.lastVerification || [],
        'blocked-by': existingState?.blockedBy || null,
        note,
      });
    }

    reconciled.push({
      lane: row.Lane,
      from: row.schedulingPhase,
      to: trackedPhase,
      reason: 'tracked-phase-reconcile',
    });
  }

  return reconciled;
}

function promoteSuggestedRows(projectRoot, execution, schedule, dryRun = false) {
  const promoted = [];

  for (const suggestion of schedule.dispatchSuggestions || []) {
    const existingState = loadLaneState(projectRoot, execution, suggestion.lane);
    const note =
      `Cycle promoted ${suggestion.lane} from ${suggestion.phase} to implementing for ` +
      `${suggestion.nextItem}.`;

    if (!dryRun) {
      recordLaneState(projectRoot, {
        execution,
        lane: suggestion.lane,
        phase: 'implementing',
        'expected-next-phase': 'spec-review-pending',
        commit: existingState?.latestCommit || null,
        reviewer: existingState?.lastReviewerResult || null,
        'correction-count': existingState?.correctionCount || 0,
        verification: existingState?.lastVerification || [],
        'blocked-by': existingState?.blockedBy || null,
        note,
      });
    }

    promoted.push({
      slot: suggestion.slot,
      lane: suggestion.lane,
      from: suggestion.phase,
      to: 'implementing',
      nextItem: suggestion.nextItem,
      nextItemSection: suggestion.nextItemSection,
    });
  }

  return promoted;
}

function runCycle(projectRoot, execution, maxActive = 4, dryRun = false) {
  if (hasExecutorDbState(projectRoot)) {
    return buildExecutorCycle(projectRoot, execution, maxActive, dryRun);
  }
  prepareExecutionState(projectRoot, execution);
  const before = computeExecutionSchedule(projectRoot, execution, maxActive);
  const observedDegradedScoreboardLoad = before.scoreboardLoad?.degraded
    ? before.scoreboardLoad
    : null;
  const reconciled = reconcileStaleRows(projectRoot, execution, before, dryRun);

  const afterReconcile = reconciled.length > 0 && !dryRun
    ? computeExecutionSchedule(projectRoot, execution, maxActive)
    : before;

  const promoted = promoteSuggestedRows(projectRoot, execution, afterReconcile, dryRun);

  if (!dryRun) {
    updateScoreboard(projectRoot);
  }

  const finalSchedule = !dryRun
    ? computeExecutionSchedule(projectRoot, execution, maxActive)
    : afterReconcile;

  return {
    execution,
    maxActiveThreads: maxActive,
    dryRun,
    reconciled,
    promoted,
    observedDegradedScoreboardLoad,
    idleSlots: finalSchedule.availableSlots,
    completedLanes: reconciled
      .filter((entry) => ['refill-ready', 'parked', 'queued'].includes(entry.to))
      .map((entry) => entry.lane),
    noDispatchReason:
      promoted.length === 0 && finalSchedule.availableSlots > 0
        ? 'no-dispatchable-lane'
        : null,
    finalSchedule,
  };
}

function renderCycle(result) {
  const lines = [
    `Execution: ${result.execution}`,
    `Max active threads: ${result.maxActiveThreads}`,
    `Dry run: ${result.dryRun ? 'yes' : 'no'}`,
    `Reconciled lanes: ${result.reconciled.length}`,
  ];

  if (result.reconciled.length > 0) {
    for (const entry of result.reconciled) {
      lines.push(`- ${entry.lane}: ${entry.from} -> ${entry.to} (${entry.reason})`);
    }
  }

  lines.push(`Promoted lanes: ${result.promoted.length}`);
  if (result.promoted.length > 0) {
    for (const entry of result.promoted) {
      lines.push(`- Slot ${entry.slot}: ${entry.lane} -> ${entry.nextItem}`);
    }
  }

  lines.push(`Idle slots: ${result.idleSlots}`);
  if (result.noDispatchReason) {
    lines.push(`No dispatch reason: ${result.noDispatchReason}`);
  }

  return lines.join('\n');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.execution) {
    throw new Error(
      'Usage: node NLSDD/scripts/nlsdd-run-cycle.cjs --execution <id> [--max-active <n>] [--dry-run] [--json]',
    );
  }

  const result = runCycle(resolveProjectRoot(), args.execution, args.maxActive, args['dry-run']);
  if (args.json) {
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    return;
  }
  process.stdout.write(`${renderCycle(result)}\n`);
}

if (require.main === module) {
  main();
}

module.exports = {
  parseArgs,
  normalizeCellPhase,
  nextExpectedPhaseFor,
  reconcileStaleRows,
  promoteSuggestedRows,
  runCycle,
  renderCycle,
};
