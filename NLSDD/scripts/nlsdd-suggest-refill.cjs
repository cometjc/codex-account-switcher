#!/usr/bin/env node

const {
  loadScoreboardTable,
  resolveProjectRoot,
  resolvePreferredScoreboardPath,
  findNextRefillItem,
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

function suggestRefill(projectRoot, execution, lane = null) {
  prepareExecutionState(projectRoot, execution);
  const scoreboardPath = resolvePreferredScoreboardPath(projectRoot);
  const scoreboardText = require('node:fs').readFileSync(scoreboardPath, 'utf8');
  const table = loadScoreboardTable(scoreboardText, scoreboardPath);
  const candidates = table.objects.filter((row) => row.Execution === execution);
  const rows = lane ? candidates.filter((row) => row.Lane === lane) : candidates;

  const suggestions = rows.map((row) => {
    const eligible =
      row['Effective phase'] === 'refill-ready' ||
      row.Phase === 'refill-ready';
    const nextItem = eligible ? findNextRefillItem(projectRoot, execution, row.Lane) : null;
    return {
      execution,
      lane: row.Lane,
      eligible,
      currentItem: row['Current item'],
      effectivePhase: row['Effective phase'] || row.Phase,
      nextItem: nextItem ? nextItem.text : null,
      nextItemSection: nextItem ? nextItem.section : null,
      outcome: eligible ? (nextItem ? 'refill-target' : 'lane-exhausted') : 'not-ready',
    };
  });

  return lane ? suggestions[0] || null : suggestions;
}

function renderSuggestion(suggestion) {
  if (!suggestion) {
    return 'No matching execution/lane found.';
  }
  if (Array.isArray(suggestion)) {
    return suggestion.map(renderSuggestion).join('\n\n');
  }
  return [
    `Execution: ${suggestion.execution}`,
    `Lane: ${suggestion.lane}`,
    `Current item: ${suggestion.currentItem}`,
    `Effective phase: ${suggestion.effectivePhase}`,
    `Outcome: ${suggestion.outcome}`,
    `Next refill target: ${suggestion.nextItem || 'n/a'}`,
    `Next refill section: ${suggestion.nextItemSection || 'n/a'}`,
  ].join('\n');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.execution) {
    throw new Error(
      'Usage: node NLSDD/scripts/nlsdd-suggest-refill.cjs --execution <id> [--lane <n>] [--json]',
    );
  }
  const result = suggestRefill(resolveProjectRoot(), args.execution, args.lane);
  if (args.json) {
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    return;
  }
  process.stdout.write(`${renderSuggestion(result)}\n`);
}

if (require.main === module) {
  main();
}

module.exports = {
  parseArgs,
  suggestRefill,
  renderSuggestion,
};
