#!/usr/bin/env node

const {reduceExecution} = require('./nlsdd-envelope.cjs');
const {resolveProjectRoot} = require('./nlsdd-lib.cjs');

function parseArgs(argv) {
  const args = {};
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === '--execution') {
      args.execution = argv[index + 1];
      index += 1;
    } else if (argv[index] === '--json') {
      args.json = true;
    }
  }
  return args;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.execution) {
    throw new Error(
      'Usage: node NLSDD/scripts/nlsdd-reduce-execution.cjs --execution <id> [--json]',
    );
  }
  const result = reduceExecution(resolveProjectRoot(), args.execution);
  if (args.json) {
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    return;
  }
  process.stdout.write(
    `Execution: ${result.execution}\nLane states projected: ${result.laneCount}\nInsights projected: ${result.insightCount}\n`,
  );
}

if (require.main === module) {
  main();
}

module.exports = {
  parseArgs,
};
