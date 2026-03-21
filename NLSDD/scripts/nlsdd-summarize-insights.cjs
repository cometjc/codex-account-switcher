#!/usr/bin/env node

const {resolveProjectRoot, summarizeExecutionInsights} = require('./nlsdd-lib.cjs');

function parseArgs(argv) {
  const args = {limit: 5};
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value === '--execution') {
      args.execution = argv[index + 1];
      index += 1;
    } else if (value === '--limit') {
      args.limit = Number(argv[index + 1]);
      index += 1;
    } else if (value === '--json') {
      args.json = true;
    }
  }
  return args;
}

function renderInsightSummary(summary) {
  const lines = [
    `Execution: ${summary.execution}`,
    `Insights total: ${summary.total}`,
    `Actionable insights: ${summary.actionableCount}`,
    `Status counts: open=${summary.countsByStatus.open || 0}, adopted=${summary.countsByStatus.adopted || 0}, resolved=${summary.countsByStatus.resolved || 0}, rejected=${summary.countsByStatus.rejected || 0}`,
  ];

  if (summary.actionable.length === 0) {
    lines.push('Actionable entries: none');
    return lines.join('\n');
  }

  lines.push('Actionable entries:');
  for (const entry of summary.actionable) {
    lines.push(
      `- [${entry.status}] ${entry.lane} · ${entry.kind} · ${entry.summary}`,
    );
  }
  return lines.join('\n');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.execution) {
    throw new Error(
      'Usage: node NLSDD/scripts/nlsdd-summarize-insights.cjs --execution <id> [--limit <n>] [--json]',
    );
  }

  const summary = summarizeExecutionInsights(resolveProjectRoot(), args.execution, args.limit);
  if (args.json) {
    process.stdout.write(`${JSON.stringify(summary, null, 2)}\n`);
    return;
  }
  process.stdout.write(`${renderInsightSummary(summary)}\n`);
}

if (require.main === module) {
  main();
}

module.exports = {
  parseArgs,
  renderInsightSummary,
};
