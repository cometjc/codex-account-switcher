const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

const RootCommand = require(path.join(process.cwd(), 'dist/commands/root.js')).default;
const packageJson = require(path.join(process.cwd(), 'package.json'));

function createRootCommand() {
  const command = Object.create(RootCommand.prototype);
  command.ansiEnabled = false;
  return command;
}

test('plot mode stays visible in the root shell help text', () => {
  const command = createRootCommand();

  assert.equal(command.nextBarStyle('quota'), 'delta');
  assert.equal(command.nextBarStyle('delta'), 'plot');
  assert.equal(command.nextBarStyle('plot'), 'quota');
  assert.equal(command.formatBarStyle('plot'), 'Plot');

  const helpText = command.buildActionsHelpText('plot', 'auto');

  assert.match(helpText, /\[B\]ar Style: Plot/);
  assert.match(helpText, /\[W\]orkload: Auto/);
  assert.match(helpText, /\[Q\]uit/);
  assert.doesNotMatch(helpText, /Bar Style: Delta/);
});

test('plot viewer package scripts remain wired in package.json', () => {
  assert.equal(packageJson.scripts['plot:viewer:build'], 'cargo build --manifest-path rust/plot-viewer/Cargo.toml --bin codex-auth');
  assert.equal(packageJson.scripts['plot:viewer:run'], 'cargo run --manifest-path rust/plot-viewer/Cargo.toml --bin codex-auth --');
  assert.equal(packageJson.scripts['rust:auth:build'], 'cargo build --manifest-path rust/plot-viewer/Cargo.toml --bin codex-auth');
  assert.equal(packageJson.scripts['rust:auth:run'], 'cargo run --manifest-path rust/plot-viewer/Cargo.toml --bin codex-auth --');
});
