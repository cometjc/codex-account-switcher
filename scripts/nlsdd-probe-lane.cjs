#!/usr/bin/env node

const {execFileSync} = require('node:child_process');
const {
  loadLanePlan,
  resolveProjectRoot,
  tryRun,
  splitStatusEntries,
  classifyNoise,
} = require('./nlsdd-lib.cjs');

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

function probeLane(projectRoot, execution, lane) {
  const lanePlan = loadLanePlan(projectRoot, execution, lane);
  if (!lanePlan || !lanePlan.worktreePath) {
    throw new Error(`Could not resolve lane plan or worktree for ${execution} ${lane}`);
  }

  const head = tryRun('git', ['rev-parse', '--short', 'HEAD'], lanePlan.worktreePath) || 'n/a';
  const branch = tryRun('git', ['rev-parse', '--abbrev-ref', 'HEAD'], lanePlan.worktreePath) || 'n/a';
  const statusOutput = tryRun('git', ['status', '--short'], lanePlan.worktreePath);
  const diffStat = tryRun('git', ['diff', '--stat'], lanePlan.worktreePath);
  const latestCommit = tryRun('git', ['log', '--oneline', '-n', '1'], lanePlan.worktreePath) || 'n/a';
  const {sourcePaths, artifactPaths} = splitStatusEntries(statusOutput);

  const verificationResults = lanePlan.verificationCommands.map((command) => {
    try {
      const output = execFileSync('/bin/bash', ['-lc', command], {
        cwd: lanePlan.worktreePath,
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'pipe'],
      }).trimEnd();
      return {command, ok: true, output: output || '(no output)'};
    } catch (error) {
      const output = String(error.stderr || error.stdout || '').trim() || '(no output)';
      return {command, ok: false, output};
    }
  });

  return {
    execution,
    lane,
    worktreePath: lanePlan.worktreePath,
    branch,
    head,
    latestCommit,
    noise: classifyNoise(statusOutput),
    sourcePaths,
    artifactPaths,
    diffStat: diffStat || '(clean)',
    verificationResults,
  };
}

function renderProbe(result) {
  return [
    `Execution: ${result.execution}`,
    `Lane: ${result.lane}`,
    `Worktree: ${result.worktreePath}`,
    `Branch: ${result.branch}`,
    `HEAD: ${result.head}`,
    `Latest commit: ${result.latestCommit}`,
    `Noise: ${result.noise}`,
    `Source paths: ${result.sourcePaths.length === 0 ? 'none' : result.sourcePaths.join(', ')}`,
    `Artifact paths: ${result.artifactPaths.length === 0 ? 'none' : result.artifactPaths.join(', ')}`,
    `Diff stat: ${result.diffStat}`,
    'Verification:',
    ...result.verificationResults.map(
      (entry) => `- ${entry.command} => ${entry.ok ? 'ok' : 'no-output-or-failed'}`,
    ),
  ].join('\n');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.execution || !args.lane) {
    throw new Error('Usage: node scripts/nlsdd-probe-lane.cjs --execution <id> --lane <n> [--json]');
  }
  const result = probeLane(resolveProjectRoot(), args.execution, args.lane);
  if (args.json) {
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    return;
  }
  process.stdout.write(`${renderProbe(result)}\n`);
}

if (require.main === module) {
  main();
}

module.exports = {
  parseArgs,
  probeLane,
  renderProbe,
};
