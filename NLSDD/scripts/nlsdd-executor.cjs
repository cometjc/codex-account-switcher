#!/usr/bin/env node

const {
  auditExecutor,
  claimAssignment,
  goExecutor,
  importLegacyExecutionState,
  importPlanFiles,
  reportResult,
} = require('./nlsdd-executor-lib.cjs');
const {resolveProjectRoot} = require('./nlsdd-lib.cjs');

function parseArgs(argv) {
  const args = {
    cleanup: false,
    json: false,
  };
  const positionals = [];

  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value === '--project-root') {
      args.projectRoot = argv[index + 1];
      index += 1;
    } else if (value === '--cleanup') {
      args.cleanup = true;
    } else if (value === '--json') {
      args.json = true;
    } else if (value === '--execution') {
      args.execution = argv[index + 1];
      index += 1;
    } else if (value === '--lane') {
      args.lane = argv[index + 1];
      index += 1;
    } else if (value === '--status') {
      args.status = argv[index + 1];
      index += 1;
    } else if (value === '--result-branch') {
      args.resultBranch = argv[index + 1];
      index += 1;
    } else if (value === '--verification-summary') {
      args.verificationSummary = argv[index + 1];
      index += 1;
    } else {
      positionals.push(value);
    }
  }

  return {
    ...args,
    command: positionals[0] || null,
  };
}

function printResult(result, asJson) {
  if (asJson) {
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    return;
  }
  process.stdout.write(`${JSON.stringify(result)}\n`);
}

function usage() {
  return [
    'Usage: node NLSDD/scripts/nlsdd-executor.cjs <command> [options]',
    '',
    'Commands:',
    '  import-plans [--cleanup] [--json]',
    '  audit [--json]',
    '  go [--json]',
    '  claim-assignment --execution <id> --lane <Lane N> [--json]',
    '  report-result --execution <id> --lane <Lane N> --status <STATUS> --result-branch <branch> [--verification-summary <text>] [--json]',
  ].join('\n');
}

function main(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const projectRoot = args.projectRoot || resolveProjectRoot();

  switch (args.command) {
    case 'import-plans': {
      const planImport = importPlanFiles(projectRoot, {cleanup: args.cleanup});
      const legacyImport = importLegacyExecutionState(projectRoot);
      printResult({...planImport, ...legacyImport}, args.json);
      return;
    }
    case 'audit':
      printResult(auditExecutor(projectRoot), args.json);
      return;
    case 'go':
      printResult(goExecutor(projectRoot), args.json);
      return;
    case 'claim-assignment':
      if (!args.execution || !args.lane) {
        throw new Error('claim-assignment requires --execution and --lane');
      }
      printResult(claimAssignment(projectRoot, args.execution, args.lane), args.json);
      return;
    case 'report-result':
      if (!args.execution || !args.lane || !args.status || !args.resultBranch) {
        throw new Error(
          'report-result requires --execution, --lane, --status, and --result-branch',
        );
      }
      printResult(
        reportResult(projectRoot, args.execution, args.lane, args.status, args.resultBranch, {
          verificationSummary: args.verificationSummary || null,
        }),
        args.json,
      );
      return;
    default:
      throw new Error(usage());
  }
}

if (require.main === module) {
  try {
    main();
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}

module.exports = {
  main,
  parseArgs,
};
