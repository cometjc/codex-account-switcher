#!/usr/bin/env node

const {existsSync} = require('node:fs');
const path = require('node:path');
const {spawnSync} = require('node:child_process');

const repoRoot = path.resolve(__dirname, '..');
const manifestPath = path.join(repoRoot, 'rust', 'plot-viewer', 'Cargo.toml');
const binaryCandidates = [
  path.join(repoRoot, 'rust', 'plot-viewer', 'target', 'release', 'codex-auth'),
  path.join(repoRoot, 'rust', 'plot-viewer', 'target', 'debug', 'codex-auth'),
];
const argv = process.argv.slice(2);

function run(binaryPath, binaryArgs) {
  const result = spawnSync(binaryPath, binaryArgs, {
    cwd: repoRoot,
    env: process.env,
    stdio: 'inherit',
  });

  if (result.error) {
    throw result.error;
  }

  process.exit(result.status ?? 1);
}

if (process.env.CODEX_AUTH_BIN) {
  run(process.env.CODEX_AUTH_BIN, argv);
}

for (const candidate of binaryCandidates) {
  if (existsSync(candidate)) {
    run(candidate, argv);
  }
}

const cargo = process.env.CARGO || 'cargo';
run(cargo, ['run', '--manifest-path', manifestPath, '--bin', 'codex-auth', '--', ...argv]);
