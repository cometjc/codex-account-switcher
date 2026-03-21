#!/usr/bin/env node

const fs = require('node:fs');
const path = require('node:path');
const {laneStatePath, resolveProjectRoot} = require('./nlsdd-lib.cjs');

function parseArgs(argv) {
  const args = {verification: []};
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value === '--execution') {
      args.execution = argv[index + 1];
      index += 1;
    } else if (value === '--lane') {
      args.lane = `Lane ${argv[index + 1]}`.replace(/^Lane\s+Lane\s+/, 'Lane ');
      index += 1;
    } else if (value === '--phase') {
      args.phase = argv[index + 1];
      index += 1;
    } else if (value === '--expected-next-phase') {
      args['expected-next-phase'] = argv[index + 1];
      index += 1;
    } else if (value === '--commit') {
      args.commit = argv[index + 1];
      index += 1;
    } else if (value === '--commit-title') {
      args['commit-title'] = argv[index + 1];
      index += 1;
    } else if (value === '--commit-body') {
      args['commit-body'] = argv[index + 1];
      index += 1;
    } else if (value === '--reviewer') {
      args.reviewer = argv[index + 1];
      index += 1;
    } else if (value === '--correction-count') {
      args['correction-count'] = argv[index + 1];
      index += 1;
    } else if (value === '--verification') {
      args.verification.push(argv[index + 1]);
      index += 1;
    } else if (value === '--blocked-by') {
      args['blocked-by'] = argv[index + 1];
      index += 1;
    } else if (value === '--note') {
      args.note = argv[index + 1];
      index += 1;
    } else if (value === '--updated-at') {
      args['updated-at'] = argv[index + 1];
      index += 1;
    }
  }
  return args;
}

function recordLaneState(projectRoot, args) {
  if (!args.execution || !args.lane || !args.phase) {
    throw new Error(
      'execution, lane, and phase are required to record NLSDD lane state',
    );
  }

  const filePath = laneStatePath(projectRoot, args.execution, args.lane);
  if (!filePath) {
    throw new Error(`Could not resolve lane state path for ${args.execution} ${args.lane}`);
  }

  fs.mkdirSync(path.dirname(filePath), {recursive: true});
  const state = {
    execution: args.execution,
    lane: args.lane,
    phase: args.phase,
    expectedNextPhase: args['expected-next-phase'] || null,
    latestCommit: args.commit || null,
    proposedCommitTitle: args['commit-title'] || null,
    proposedCommitBody: args['commit-body'] || null,
    lastReviewerResult: args.reviewer || null,
    lastVerification: args.verification || [],
    blockedBy: args['blocked-by'] || null,
    note: args.note || null,
    correctionCount: Number(args['correction-count'] || 0),
    updatedAt: args['updated-at'] || new Date().toISOString(),
  };
  fs.writeFileSync(filePath, `${JSON.stringify(state, null, 2)}\n`, 'utf8');
  return filePath;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.execution || !args.lane || !args.phase) {
    throw new Error(
      'Usage: node NLSDD/scripts/nlsdd-record-lane-state.cjs --execution <id> --lane <n> --phase <phase> [--expected-next-phase <phase>] [--commit <sha>] [--commit-title <title>] [--commit-body <body>] [--reviewer <result>] [--correction-count <n>] [--verification <cmd>] [--blocked-by <reason>] [--note <text>] [--updated-at <iso>]',
    );
  }
  const filePath = recordLaneState(resolveProjectRoot(), args);
  process.stdout.write(`${filePath}\n`);
}

if (require.main === module) {
  main();
}

module.exports = {
  parseArgs,
  recordLaneState,
};
