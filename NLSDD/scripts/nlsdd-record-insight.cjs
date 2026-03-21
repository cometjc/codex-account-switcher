#!/usr/bin/env node

const fs = require('node:fs');
const path = require('node:path');
const {
  INSIGHT_KINDS,
  INSIGHT_STATUSES,
  executionInsightsPath,
  normalizeInsightLane,
  resolveProjectRoot,
} = require('./nlsdd-lib.cjs');

function parseArgs(argv) {
  const args = {};
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value === '--execution') {
      args.execution = argv[index + 1];
      index += 1;
    } else if (value === '--lane') {
      args.lane = argv[index + 1];
      index += 1;
    } else if (value === '--source') {
      args.source = argv[index + 1];
      index += 1;
    } else if (value === '--kind') {
      args.kind = argv[index + 1];
      index += 1;
    } else if (value === '--status') {
      args.status = argv[index + 1];
      index += 1;
    } else if (value === '--summary') {
      args.summary = argv[index + 1];
      index += 1;
    } else if (value === '--detail') {
      args.detail = argv[index + 1];
      index += 1;
    } else if (value === '--related-lane') {
      args['related-lane'] = argv[index + 1];
      index += 1;
    } else if (value === '--related-commit') {
      args['related-commit'] = argv[index + 1];
      index += 1;
    } else if (value === '--related-agent') {
      args['related-agent'] = argv[index + 1];
      index += 1;
    } else if (value === '--recorded-by') {
      args['recorded-by'] = argv[index + 1];
      index += 1;
    } else if (value === '--timestamp') {
      args.timestamp = argv[index + 1];
      index += 1;
    }
  }
  return args;
}

function recordInsight(projectRoot, args) {
  if (!args.execution || !args.source || !args.kind || !args.summary) {
    throw new Error(
      'execution, source, kind, and summary are required to record an NLSDD execution insight',
    );
  }

  const filePath = executionInsightsPath(projectRoot, args.execution);
  if (!filePath) {
    throw new Error(`Could not resolve execution insights path for ${args.execution}`);
  }

  if (!INSIGHT_KINDS.includes(args.kind)) {
    throw new Error(
      `Unknown insight kind "${args.kind}". Expected one of: ${INSIGHT_KINDS.join(', ')}`,
    );
  }
  if (args.status && !INSIGHT_STATUSES.includes(args.status)) {
    throw new Error(
      `Unknown insight status "${args.status}". Expected one of: ${INSIGHT_STATUSES.join(', ')}`,
    );
  }

  fs.mkdirSync(path.dirname(filePath), {recursive: true});
  const entry = {
    timestamp: args.timestamp || new Date().toISOString(),
    execution: args.execution,
    lane: normalizeInsightLane(args.lane),
    source: args.source,
    kind: args.kind,
    status: args.status || 'open',
    summary: args.summary,
    detail: args.detail || null,
    relatedLane: normalizeInsightLane(args['related-lane'] || args.lane || 'global'),
    relatedCommit: args['related-commit'] || null,
    relatedAgent: args['related-agent'] || null,
    recordedBy: args['recorded-by'] || 'coordinator',
  };
  fs.appendFileSync(filePath, `${JSON.stringify(entry)}\n`, 'utf8');
  return {filePath, entry};
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.execution || !args.source || !args.kind || !args.summary) {
    throw new Error(
      'Usage: node NLSDD/scripts/nlsdd-record-insight.cjs --execution <id> --source <subagent|coordinator> --kind <suggestion|observed-issue|improvement-opportunity|noop-finding|blocker|resolved-blocker> --summary <text> [--lane <lane|global>] [--status <open|adopted|rejected|resolved>] [--detail <text>] [--related-lane <lane|global>] [--related-commit <sha>] [--related-agent <name>] [--recorded-by <name>] [--timestamp <iso>]',
    );
  }
  const {filePath} = recordInsight(resolveProjectRoot(), args);
  process.stdout.write(`${filePath}\n`);
}

if (require.main === module) {
  main();
}

module.exports = {
  parseArgs,
  recordInsight,
};
