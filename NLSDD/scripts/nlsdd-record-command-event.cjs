#!/usr/bin/env node

const {recordEnvelope} = require('./nlsdd-envelope.cjs');
const {resolveProjectRoot} = require('./nlsdd-lib.cjs');

const COMMAND_EVENT_TYPES = [
  'command-started',
  'command-finished',
  'command-failed',
  'command-blocked',
  'command-probe',
];

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
    } else if (value === '--event-type') {
      args.eventType = argv[index + 1];
      index += 1;
    } else if (value === '--command') {
      args.command = argv[index + 1];
      index += 1;
    } else if (value === '--cwd') {
      args.cwd = argv[index + 1];
      index += 1;
    } else if (value === '--status') {
      args.status = argv[index + 1];
      index += 1;
    } else if (value === '--exit-code') {
      args.exitCode = argv[index + 1];
      index += 1;
    } else if (value === '--duration-ms') {
      args.durationMs = argv[index + 1];
      index += 1;
    } else if (value === '--block-kind') {
      args.blockKind = argv[index + 1];
      index += 1;
    } else if (value === '--probe-summary') {
      args.probeSummary = argv[index + 1];
      index += 1;
    } else if (value === '--pid') {
      args.pid = argv[index + 1];
      index += 1;
    } else if (value === '--timestamp') {
      args.timestamp = argv[index + 1];
      index += 1;
    } else if (value === '--summary') {
      args.summary = argv[index + 1];
      index += 1;
    }
  }
  return args;
}

function normalizeNumber(value) {
  if (value === undefined || value === null || value === '') {
    return null;
  }
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function defaultStatus(eventType, explicitStatus) {
  if (explicitStatus) {
    return explicitStatus;
  }
  switch (eventType) {
    case 'command-started':
      return 'started';
    case 'command-finished':
      return 'finished';
    case 'command-failed':
      return 'failed';
    case 'command-blocked':
      return 'blocked';
    case 'command-probe':
      return 'probe';
    default:
      return null;
  }
}

function defaultSummary(args) {
  if (args.summary) {
    return args.summary;
  }
  if (args.eventType === 'command-blocked') {
    return args.blockKind
      ? `Command blocked: ${args.command} (${args.blockKind})`
      : `Command blocked: ${args.command}`;
  }
  if (args.eventType === 'command-failed') {
    return `Command failed: ${args.command}${args.exitCode != null ? ` (exit ${args.exitCode})` : ''}`;
  }
  if (args.eventType === 'command-finished') {
    return `Command finished: ${args.command}`;
  }
  if (args.eventType === 'command-probe') {
    return args.probeSummary
      ? `Command probe: ${args.command} · ${args.probeSummary}`
      : `Command probe: ${args.command}`;
  }
  return `Command started: ${args.command}`;
}

function recordCommandEvent(projectRoot, args) {
  if (!args.execution || !args.lane || !args.eventType || !args.command) {
    throw new Error(
      'execution, lane, eventType, and command are required to record NLSDD command telemetry',
    );
  }
  if (!COMMAND_EVENT_TYPES.includes(args.eventType)) {
    throw new Error(
      `Unknown NLSDD command eventType "${args.eventType}". Expected one of: ${COMMAND_EVENT_TYPES.join(', ')}`,
    );
  }

  const result = recordEnvelope(projectRoot, {
    execution: args.execution,
    lane: args.lane,
    role: 'worker',
    eventType: args.eventType,
    command: args.command,
    cwd: args.cwd || process.cwd(),
    status: defaultStatus(args.eventType, args.status),
    exitCode: normalizeNumber(args.exitCode),
    durationMs: normalizeNumber(args.durationMs),
    blockKind: args.blockKind || null,
    probeSummary: args.probeSummary || null,
    pid: normalizeNumber(args.pid) ?? process.pid,
    summary: defaultSummary(args),
    timestamp: args.timestamp || new Date().toISOString(),
  });
  return result.filePath;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.execution || !args.lane || !args.eventType || !args.command) {
    throw new Error(
      'Usage: node NLSDD/scripts/nlsdd-record-command-event.cjs --execution <id> --lane <n> --event-type <command-started|command-finished|command-failed|command-blocked|command-probe> --command <text> [--cwd <path>] [--status <status>] [--exit-code <n>] [--duration-ms <n>] [--block-kind <kind>] [--probe-summary <text>] [--pid <n>] [--timestamp <iso>]',
    );
  }
  const filePath = recordCommandEvent(resolveProjectRoot(), args);
  process.stdout.write(`${filePath}\n`);
}

if (require.main === module) {
  main();
}

module.exports = {
  parseArgs,
  recordCommandEvent,
  COMMAND_EVENT_TYPES,
};
