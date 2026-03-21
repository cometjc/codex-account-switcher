const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const README_PATH = path.join(process.cwd(), 'README.md');

function readReadme() {
  return fs.readFileSync(README_PATH, 'utf8');
}

test('README keeps plot mode in its current in-progress position', () => {
  const readme = readReadme();

  const plotSectionIndex = readme.indexOf('## Plot Mode');
  const requirementsIndex = readme.indexOf('## Requirements');
  const installIndex = readme.indexOf('## Install (npm)');

  assert.ok(plotSectionIndex >= 0, 'README should include a Plot Mode section');
  assert.ok(requirementsIndex >= 0, 'README should include a Requirements section');
  assert.ok(installIndex >= 0, 'README should include an Install section');
  assert.ok(
    requirementsIndex < plotSectionIndex,
    'Plot Mode should stay after Requirements',
  );
  assert.ok(
    plotSectionIndex < installIndex,
    'Plot Mode should stay before Install',
  );

  const plotSection = readme.slice(plotSectionIndex, installIndex);

  assert.match(
    plotSection,
    /Plot mode is an experimental, in-progress phase-1 migration toward a Rust TUI viewer\./,
  );
  assert.match(
    plotSection,
    /Node remains the source of truth for auth, cache, and API access in this phase\./,
  );
  assert.match(
    plotSection,
    /Any `plot:viewer:\*` scripts in `package\.json` should be treated as developer scaffolding, not stable end-user commands\./,
  );
  assert.match(
    plotSection,
    /Those scripts now point at cargo-backed Rust viewer commands, but the viewer itself is still being fleshed out\./,
  );
});
