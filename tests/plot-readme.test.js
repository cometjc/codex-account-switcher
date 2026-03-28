const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const README_PATH = path.join(process.cwd(), 'README.md');

function readReadme() {
  return fs.readFileSync(README_PATH, 'utf8');
}

test('README reflects the Rust-first agent-switch runtime and thin npm shim', () => {
  const readme = readReadme();

  const plotSectionIndex = readme.indexOf('## Rust TUI');
  const requirementsIndex = readme.indexOf('## Requirements');
  const buildIndex = readme.indexOf('## Build');
  const installIndex = readme.indexOf('## Install (npm)');

  assert.ok(plotSectionIndex >= 0, 'README should include a Rust TUI section');
  assert.ok(requirementsIndex >= 0, 'README should include a Requirements section');
  assert.ok(buildIndex >= 0, 'README should include a Build section');
  assert.ok(installIndex >= 0, 'README should include an Install section');
  assert.ok(requirementsIndex < plotSectionIndex, 'Rust TUI should stay after Requirements');
  assert.ok(plotSectionIndex < buildIndex, 'Rust TUI should stay before Build');
  assert.ok(buildIndex < installIndex, 'Build should stay before Install');

  const plotSection = readme.slice(plotSectionIndex, installIndex);

  assert.match(
    plotSection,
    /`agent-switch` now has a Rust-first runtime for auth\/profile management and plot rendering\./,
  );
  assert.match(
    plotSection,
    /Auth snapshot storage, saved profile switching, usage cache reads, and the plot view all live in the same Rust app\./,
  );
  assert.match(
    plotSection,
    /Plot is no longer treated as a separate external viewer truth; it is a built-in view of the main TUI\./,
  );
  assert.match(
    plotSection,
    /thin shim.*single Rust `agent-switch` binary entrypoint/i,
  );
  assert.match(readme, /Node\.js 18 or newer for repo contract tests/);
  assert.doesNotMatch(plotSection, /@oclif\/core|dist\/index\.js|legacy development helpers|plot:viewer:\*/);
});
