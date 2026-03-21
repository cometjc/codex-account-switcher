#!/usr/bin/env node

const {computeExecutionSchedule, resolveProjectRoot} = require('./nlsdd-lib.cjs');
const {prepareExecutionState} = require('./nlsdd-envelope.cjs');

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
    }
  }
  return args;
}

function renderRows(title, rows) {
  if (rows.length === 0) {
    return `${title}: none`;
  }

  return [
    `${title}:`,
    ...rows.map((row) => `- ${row.Lane} (${row.schedulingPhase}): ${row['Current item']}`),
  ].join('\n');
}

function renderSuggestions(suggestions) {
  if (suggestions.length === 0) {
    return 'Dispatch suggestions: none';
  }
  return [
    'Dispatch suggestions:',
    ...suggestions.map(
      (suggestion) =>
        `- Slot ${suggestion.slot}: ${suggestion.lane} -> ${suggestion.nextItem} (${suggestion.nextItemSection})`,
    ),
  ].join('\n');
}

function renderSchedule(schedule) {
  return [
    `Execution: ${schedule.execution}`,
    `Max active threads: ${schedule.maxActiveThreads}`,
    `Active thread usage: ${schedule.activeRows.length}/${schedule.maxActiveThreads}`,
    `Available slots: ${schedule.availableSlots}`,
    renderRows('Active lanes', schedule.activeRows),
    renderRows('Refill-ready lanes', schedule.refillReadyRows),
    renderRows('Queued lanes', schedule.queuedRows),
    renderRows('Blocked lanes', schedule.blockedRows),
    renderRows('Stale implementing lanes', schedule.staleRows || []),
    renderSuggestions(schedule.dispatchSuggestions),
  ].join('\n\n');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.execution) {
    throw new Error(
      'Usage: node NLSDD/scripts/nlsdd-suggest-schedule.cjs --execution <id> [--max-active <n>] [--json]',
    );
  }
  const projectRoot = resolveProjectRoot();
  prepareExecutionState(projectRoot, args.execution);
  const schedule = computeExecutionSchedule(projectRoot, args.execution, args.maxActive);
  if (args.json) {
    process.stdout.write(`${JSON.stringify(schedule, null, 2)}\n`);
    return;
  }
  process.stdout.write(`${renderSchedule(schedule)}\n`);
}

if (require.main === module) {
  main();
}

module.exports = {
  parseArgs,
  renderSchedule,
};
