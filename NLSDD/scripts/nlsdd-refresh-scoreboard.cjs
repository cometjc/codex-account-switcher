#!/usr/bin/env node

const fs = require('node:fs');
const {
  classifyNoise,
  computeLaneAutomation,
  formatIsoTimestamp,
  joinRow,
  loadLanePlan,
  loadScoreboardTable,
  readRecentThreads,
  refreshProbe,
  resolveProjectRoot,
  resolveScoreboardPath,
  tryRun,
} = require('./nlsdd-lib.cjs');

const projectRoot = resolveProjectRoot();
const scoreboardPath = resolveScoreboardPath(projectRoot);

function escapeTableCell(value) {
  return String(value).replace(/\|/g, '\\|');
}

function buildRecentThreadsSection(threads) {
  const lines = [
    '## Recent Codex Threads',
    '',
    '> Auto-refreshed from `~/.codex/state_5.sqlite` for this repo cwd.',
    '',
    '| Nickname | Role | Thread ID | Updated |',
    '| --- | --- | --- | --- |',
  ];

  if (threads.length === 0) {
    lines.push('| n/a | n/a | n/a | n/a |');
    return lines.join('\n');
  }

  for (const thread of threads) {
    lines.push(`| ${thread.nickname} | ${thread.role} | \`${thread.id}\` | ${thread.updated} |`);
  }
  return lines.join('\n');
}

function updateScoreboard() {
  const scoreboardText = fs.readFileSync(scoreboardPath, 'utf8');
  const table = loadScoreboardTable(scoreboardText, scoreboardPath);

  for (const row of table.objects) {
    const lanePlan = loadLanePlan(projectRoot, row.Execution, row.Lane);
    const worktree = lanePlan?.worktreePath;
    const automation = computeLaneAutomation(projectRoot, row.Execution, row.Lane, row.Phase);
    row['Effective phase'] = automation.effectivePhase;
    row['Latest event'] = automation.latestEventText;
    row['Correction count'] = String(automation.correctionCount);
    row['Last activity'] = automation.lastActivityText;

    if (!worktree || !fs.existsSync(worktree)) {
      row['Branch HEAD'] = '`n/a`';
      row['Noise'] = lanePlan ? 'missing-worktree' : 'missing-lane-plan';
      row['Last probe'] = lanePlan ? 'missing-worktree' : 'missing-lane-plan';
      continue;
    }

    const head = tryRun('git', ['rev-parse', '--short', 'HEAD'], worktree) || 'n/a';
    const statusOutput = tryRun('git', ['status', '--short'], worktree);
    row['Branch HEAD'] = `\`${head}\``;
    row['Noise'] = classifyNoise(statusOutput);
    row['Last probe'] = refreshProbe(head, statusOutput);
  }

  const renderedRows = table.objects.map((row) =>
    joinRow(table.columns.map((column) => escapeTableCell(row[column] || ''))),
  );
  const recentThreadsSection = buildRecentThreadsSection(readRecentThreads(projectRoot, 8));
  const nextLines = [
    ...table.lines.slice(0, table.headerIndex),
    table.header,
    table.separator,
    ...renderedRows,
    '',
    recentThreadsSection,
    '',
  ];

  fs.writeFileSync(scoreboardPath, nextLines.join('\n'), 'utf8');
}

if (require.main === module) {
  updateScoreboard();
}

module.exports = {
  updateScoreboard,
  buildRecentThreadsSection,
};
