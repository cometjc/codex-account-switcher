#!/usr/bin/env node

const fs = require('node:fs');
const {
  resolveProjectRoot,
  telemetryReviewPath,
  telemetrySummaryPath,
} = require('./nlsdd-lib.cjs');

function parseArgs(argv) {
  const args = {};
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value === '--execution') {
      args.execution = argv[index + 1];
      index += 1;
    }
  }
  return args;
}

function renderTelemetryReview(summary) {
  const lines = [
    `# ${summary.execution} Telemetry Review`,
    '',
    `- Wall clock duration: ${summary.wallClockDurationMs} ms`,
    `- First activity: ${summary.firstActivityAt || 'n/a'}`,
    `- Last activity: ${summary.lastActivityAt || 'n/a'}`,
    `- Minute buckets: ${summary.minuteBuckets.length}`,
    `- Drop segments: ${summary.dropSegments.length}`,
    '',
    '## Per-Minute Worker Counts',
    '',
    '| Minute | Active workers | Productive workers |',
    '| --- | --- | --- |',
  ];

  for (const bucket of summary.minuteBuckets) {
    lines.push(
      `| ${bucket.minuteStartAt || bucket.minute} | ${bucket.activeWorkers} | ${bucket.productiveWorkers} |`,
    );
  }

  lines.push('', '## Drop Segments', '');
  if (summary.dropSegments.length === 0) {
    lines.push('- none');
  } else {
    for (const segment of summary.dropSegments) {
      lines.push(
        `- ${segment.reason} [${segment.metric}] ${segment.fromMinute} -> ${segment.toMinute} (confidence: ${segment.confidence})`,
      );
      if (segment.missingSignals && segment.missingSignals.length > 0) {
        lines.push(`  Missing signals: ${segment.missingSignals.join(', ')}`);
      }
    }
  }

  return `${lines.join('\n')}\n`;
}

function renderTelemetryReviewFile(projectRoot, execution) {
  const summaryPath = telemetrySummaryPath(projectRoot, execution);
  if (!summaryPath || !fs.existsSync(summaryPath)) {
    throw new Error(`Telemetry summary not found for execution "${execution}"`);
  }
  const summary = JSON.parse(fs.readFileSync(summaryPath, 'utf8'));
  const outputPath = telemetryReviewPath(projectRoot, execution);
  fs.mkdirSync(require('node:path').dirname(outputPath), {recursive: true});
  const content = renderTelemetryReview(summary);
  fs.writeFileSync(outputPath, content, 'utf8');
  return {outputPath, content, summary};
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.execution) {
    throw new Error(
      'Usage: node NLSDD/scripts/nlsdd-render-telemetry-review.cjs --execution <id>',
    );
  }
  const result = renderTelemetryReviewFile(resolveProjectRoot(), args.execution);
  process.stdout.write(`${result.outputPath}\n`);
}

if (require.main === module) {
  main();
}

module.exports = {
  parseArgs,
  renderTelemetryReview,
  renderTelemetryReviewFile,
};
