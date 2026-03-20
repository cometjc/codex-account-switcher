#!/usr/bin/env node

const fs = require('node:fs');
const path = require('node:path');
const {execFileSync} = require('node:child_process');

const repoRoot = path.resolve(__dirname, '..');
const targetDir = process.env.CODEX_AUTH_DEV_BIN_DIR || getGlobalBinDir();
const targetPath = path.join(targetDir, 'codex-auth-dev');

try {
  const existing = fs.lstatSync(targetPath);

  if (!existing.isSymbolicLink()) {
    console.error(`Refusing to remove non-symlink target: ${targetPath}`);
    process.exit(1);
  }

  fs.rmSync(targetPath, {force: true});
  console.log(`Removed ${targetPath}`);
} catch (error) {
  if (error && error.code === 'ENOENT') {
    console.log(`No link found at ${targetPath}`);
    process.exit(0);
  }

  throw error;
}

function getGlobalBinDir() {
  const prefix = execFileSync('npm', ['prefix', '-g'], {
    cwd: repoRoot,
    encoding: 'utf8',
  }).trim();

  return process.platform === 'win32' ? prefix : path.join(prefix, 'bin');
}
