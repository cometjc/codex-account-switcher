#!/usr/bin/env node

const fs = require('node:fs');
const path = require('node:path');
const {execFileSync} = require('node:child_process');

const repoRoot = path.resolve(__dirname, '..');
const sourcePath = path.join(repoRoot, 'bin', 'agent-switch.cjs');
const targetDir = process.env.AGENT_SWITCH_DEV_BIN_DIR || process.env.CODEX_AUTH_DEV_BIN_DIR || getGlobalBinDir();
const targetPath = path.join(targetDir, 'agent-switch');

fs.mkdirSync(targetDir, {recursive: true});

try {
  const existing = fs.lstatSync(targetPath);
  if (existing.isSymbolicLink()) {
    const linkedPath = fs.readlinkSync(targetPath);
    const resolvedLinkedPath = path.resolve(targetDir, linkedPath);
    if (resolvedLinkedPath === sourcePath) {
      console.log(`agent-switch is already linked at ${targetPath}`);
      process.exit(0);
    }
  }

  fs.rmSync(targetPath, {force: true});
} catch (error) {
  if (error && error.code !== 'ENOENT') throw error;
}

fs.symlinkSync(sourcePath, targetPath);

console.log(`Linked agent-switch -> ${sourcePath}`);
console.log(`Target: ${targetPath}`);

function getGlobalBinDir() {
  const prefix = execFileSync('npm', ['prefix', '-g'], {
    cwd: repoRoot,
    encoding: 'utf8',
  }).trim();

  return process.platform === 'win32' ? prefix : path.join(prefix, 'bin');
}
