const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {execFileSync} = require('node:child_process');

function repoRoot(...segments) {
  return path.join(process.cwd(), ...segments);
}

function freshRequire(relativePath) {
  const fullPath = repoRoot(relativePath);
  delete require.cache[require.resolve(fullPath)];
  return require(fullPath);
}

function run(command, args, cwd) {
  return execFileSync(command, args, {
    cwd,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trimEnd();
}

function writeLaneState(root, execution, laneNumber, state) {
  const stateDir = path.join(root, 'NLSDD', 'state', execution);
  fs.mkdirSync(stateDir, {recursive: true});
  fs.writeFileSync(
    path.join(stateDir, `lane-${laneNumber}.json`),
    `${JSON.stringify(state, null, 2)}\n`,
    'utf8',
  );
}

function setupTempGitRepo(dir) {
  fs.mkdirSync(dir, {recursive: true});
  run('git', ['init', '-q'], dir);
  run('git', ['config', 'user.name', 'Codex Test'], dir);
  run('git', ['config', 'user.email', 'codex@example.com'], dir);
  fs.writeFileSync(path.join(dir, 'tracked.txt'), 'initial\n', 'utf8');
  fs.mkdirSync(path.join(dir, 'rust', 'plot-viewer', 'target'), {recursive: true});
  fs.writeFileSync(
    path.join(dir, 'rust', 'plot-viewer', 'target', 'noise.bin'),
    'initial-artifact\n',
    'utf8',
  );
  run('git', ['add', 'tracked.txt'], dir);
  run('git', ['add', 'rust/plot-viewer/target/noise.bin'], dir);
  run('git', ['commit', '-m', 'init'], dir);
}

function setupNlsddFixture() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-'));
  const laneWorktree = path.join(root, '.worktrees', 'lane-1-node');
  setupTempGitRepo(laneWorktree);

  fs.writeFileSync(path.join(laneWorktree, 'tracked.txt'), 'changed\n', 'utf8');
  fs.writeFileSync(
    path.join(laneWorktree, 'rust', 'plot-viewer', 'target', 'noise.bin'),
    'changed-artifact\n',
    'utf8',
  );

  fs.mkdirSync(path.join(root, 'NLSDD', 'executions', 'plot-mode'), {recursive: true});
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'executions', 'plot-mode', 'lane-1.md'),
    `# Lane 1 Plan - Node Contract and Handoff

> Ownership family:
> \`src/commands/root.ts\`
>
> NLSDD worktree: \`.worktrees/lane-1-node\`
>
> Lane-local verification:
> \`git status --short\`
> \`git rev-parse --short HEAD\`

## M - Model / Contract

- [x] Done model item
- [ ] Tighten snapshot builder semantics when real 7d history evolves

## Current Lane Status

- [x] Review previous commit

## Refill Order

- [ ] Then consume remaining Model items
`,
    'utf8',
  );

  fs.mkdirSync(path.join(root, 'NLSDD'), {recursive: true});
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'scoreboard.md'),
    `# NLSDD Scoreboard

| Execution | Lane | Ownership | Current item | Phase | Item commit | Last verification | Blocked by | Next refill target | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| plot-mode | Lane 1 | Node contract + handoff | Viewer launch confidence / shell messaging | spec-review-pending | \`abc1234\` | \`git status --short\` | none | Tighten snapshot builder semantics when real 7d history evolves | test row |
`,
    'utf8',
  );

  const codexDir = path.join(root, '.codex');
  const sessionsRoot = path.join(codexDir, 'sessions', '2026', '03', '21');
  fs.mkdirSync(sessionsRoot, {recursive: true});
  const dbPath = path.join(codexDir, 'state_5.sqlite');
  run(
    'sqlite3',
    [
      dbPath,
      `
create table threads (
  id text,
  cwd text,
  agent_nickname text,
  agent_role text,
  title text,
  updated_at integer
);
insert into threads values ('thread-lane-1-pass', '${root.replace(/'/g, "''")}', 'Meitner', 'worker', 'Lane 1 spec review', 1774031868);
insert into threads values ('thread-lane-1-fail', '${root.replace(/'/g, "''")}', 'Erdos', 'worker', 'Lane 1 quality review', 1774031800);
      `,
    ],
    root,
  );

  fs.writeFileSync(
    path.join(sessionsRoot, 'rollout-2026-03-21T03-00-00-thread-lane-1-pass.jsonl'),
    `${JSON.stringify({
      timestamp: '2026-03-21T03:00:00.000Z',
      type: 'event_msg',
      payload: {
        type: 'agent_message',
        message: 'status: PASS\\n\\nlane name: Lane 1\\n\\nspec review passed cleanly',
      },
    })}\n`,
    'utf8',
  );
  fs.writeFileSync(
    path.join(sessionsRoot, 'rollout-2026-03-21T02-59-00-thread-lane-1-fail.jsonl'),
    `${JSON.stringify({
      timestamp: '2026-03-21T02:59:00.000Z',
      type: 'event_msg',
      payload: {
        type: 'agent_message',
        message: 'status: FAIL\\n\\nlane name: Lane 1\\n\\nquality review found scope issue',
      },
    })}\n`,
    'utf8',
  );

  return {root, dbPath, sessionsRoot, laneWorktree};
}

function setupSelfHostingScheduleFixture() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-schedule-'));
  const executionDir = path.join(root, 'NLSDD', 'executions', 'nlsdd-self-hosting');
  fs.mkdirSync(executionDir, {recursive: true});

  const lanePlans = [
    {
      lane: 1,
      worktree: '.worktrees/nlsdd-lane-1-scheduler',
      title: 'Scheduler Core',
      verification: 'node --test tests/nlsdd-automation.test.js',
      item: 'Normalize scheduling phases for multi-lane dispatch',
    },
    {
      lane: 2,
      worktree: '.worktrees/nlsdd-lane-2-scoreboard',
      title: 'Scoreboard Integration',
      verification: 'npm run nlsdd:scoreboard:refresh',
      item: 'Keep scoreboard rows aligned with the active-cap model',
    },
    {
      lane: 3,
      worktree: '.worktrees/nlsdd-lane-3-rules',
      title: 'Rules and Communication',
      verification: 'rg -n "active lane count" spec/NLSDD',
      item: 'Rewrite remaining fixed-lane wording',
    },
    {
      lane: 4,
      worktree: '.worktrees/nlsdd-lane-4-tests',
      title: 'Regression and CLI Surface',
      verification: 'node --test tests/nlsdd-automation.test.js',
      item: 'Add schedule regression coverage',
    },
    {
      lane: 5,
      worktree: '.worktrees/nlsdd-lane-5-docs',
      title: 'Plot-Mode Migration',
      verification: 'rg -n "lane pool" NLSDD/executions/plot-mode',
      item: 'Adjust plot-mode docs to lane-pool language',
    },
    {
      lane: 6,
      worktree: '.worktrees/nlsdd-lane-6-followup',
      title: 'Coordinator Follow-up',
      verification: 'sed -n \'1,220p\' tasks/todo.md',
      item: 'Capture coordinator ergonomics follow-up',
    },
  ];

  for (const lanePlan of lanePlans) {
    const laneDir = path.join(root, lanePlan.worktree);
    fs.mkdirSync(laneDir, {recursive: true});
    fs.mkdirSync(path.join(laneDir, 'rust', 'plot-viewer', 'target'), {recursive: true});
    fs.writeFileSync(path.join(laneDir, 'tracked.txt'), `lane-${lanePlan.lane}\n`, 'utf8');
    fs.writeFileSync(
      path.join(laneDir, 'rust', 'plot-viewer', 'target', 'noise.bin'),
      `artifact-${lanePlan.lane}\n`,
      'utf8',
    );

    fs.writeFileSync(
      path.join(executionDir, `lane-${lanePlan.lane}.md`),
      `# Lane ${lanePlan.lane} Plan - ${lanePlan.title}

> Ownership family:
> \`tests/nlsdd-automation.test.js\`
>
> NLSDD worktree: \`${lanePlan.worktree}\`
>
> Lane-local verification:
> \`${lanePlan.verification}\`

## M - Model / Work Item

- [ ] ${lanePlan.item}

## Current Lane Status

- [x] parked
`,
      'utf8',
    );
  }

  fs.mkdirSync(path.join(root, 'NLSDD'), {recursive: true});
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'scoreboard.md'),
    `# NLSDD Scoreboard

| Execution | Lane | Ownership | Current item | Phase | Item commit | Last verification | Blocked by | Next refill target | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| nlsdd-self-hosting | Lane 1 | Scheduler core | Normalize scheduling phases for multi-lane dispatch | implementing | \`1a2b3c4\` | \`node --test tests/nlsdd-automation.test.js\` | none | Scheduler edge cases | active lane |
| nlsdd-self-hosting | Lane 2 | Scoreboard integration | Keep scoreboard rows aligned with the active-cap model | spec-review-pending | \`2a2b3c4\` | \`npm run nlsdd:scoreboard:refresh\` | none | Scoreboard wording polish | active lane |
| nlsdd-self-hosting | Lane 3 | Rules and communication | Rewrite remaining fixed-lane wording | refill-ready | \`3a2b3c4\` | \`rg -n "active lane count" spec/NLSDD; rg -n "4 active lanes" NLSDD\` | none | Execution wording cleanup | ready to refill |
| nlsdd-self-hosting | Lane 4 | Regression and CLI surface | Add schedule regression coverage | refill-ready | \`4a2b3c4\` | \`node --test tests/nlsdd-automation.test.js\` | none | Scoreboard/schedule cross-check coverage | ready to refill |
| nlsdd-self-hosting | Lane 5 | Plot-mode migration | Adjust plot-mode docs to lane-pool language | queued | \`n/a\` | \`rg -n "lane pool" NLSDD/executions/plot-mode\` | wait-slot | Plot-mode overview wording | queued lane |
| nlsdd-self-hosting | Lane 6 | Coordinator follow-up | Capture coordinator ergonomics follow-up | queued | \`n/a\` | \`sed -n '1,220p' tasks/todo.md\` | wait-slot | Coordinator follow-up | queued lane |
`,
    'utf8',
  );

  return {root};
}

test('scoreboard refresh v2 backfills effective phase and lane event metadata', () => {
  const fixture = setupNlsddFixture();
  process.env.NLSDD_PROJECT_ROOT = fixture.root;
  process.env.NLSDD_SCOREBOARD_PATH = path.join(fixture.root, 'NLSDD', 'scoreboard.md');
  process.env.NLSDD_RUNTIME_SCOREBOARD_PATH = path.join(
    fixture.root,
    'NLSDD',
    'state',
    'scoreboard.runtime.md',
  );
  process.env.CODEX_STATE_DB_PATH = fixture.dbPath;
  process.env.CODEX_SESSIONS_ROOT = path.join(fixture.root, '.codex', 'sessions');

  const trackedBefore = fs.readFileSync(process.env.NLSDD_SCOREBOARD_PATH, 'utf8');
  const {updateScoreboard} = freshRequire('NLSDD/scripts/nlsdd-refresh-scoreboard.cjs');
  updateScoreboard();

  const trackedAfter = fs.readFileSync(process.env.NLSDD_SCOREBOARD_PATH, 'utf8');
  const runtimeText = fs.readFileSync(process.env.NLSDD_RUNTIME_SCOREBOARD_PATH, 'utf8');
  assert.equal(trackedAfter, trackedBefore);
  assert.match(
    runtimeText,
    /This runtime scoreboard expands the tracked manual scoreboard with derived lane state, probe results, and recent Codex thread activity\./,
  );
  assert.doesNotMatch(runtimeText, /This tracked scoreboard keeps only coordinator-owned manual fields\./);
  assert.match(runtimeText, /\| plot-mode \| Lane 1 \|/);
  assert.match(runtimeText, /\| quality-review-pending \|/);
  assert.match(runtimeText, /PASS · Meitner · 2026-03-21 03:00:00Z/);
  assert.match(runtimeText, /\| 1 \| 2026-03-20 18:37:48Z \|/);
  assert.match(runtimeText, /mixed/);
  assert.match(runtimeText, /## Recent Codex Threads/);
});

test('resolveProjectRoot prefers the current linked worktree when it has its own NLSDD surface', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-root-'));
  fs.mkdirSync(path.join(root, '.worktrees'), {recursive: true});
  run('git', ['init', '-q'], root);
  run('git', ['config', 'user.name', 'Codex Test'], root);
  run('git', ['config', 'user.email', 'codex@example.com'], root);
  fs.writeFileSync(path.join(root, 'tracked.txt'), 'root\n', 'utf8');
  fs.mkdirSync(path.join(root, 'NLSDD'), {recursive: true});
  fs.writeFileSync(path.join(root, 'NLSDD', 'scoreboard.md'), '# root scoreboard\n', 'utf8');
  run('git', ['add', 'tracked.txt'], root);
  run('git', ['add', 'NLSDD/scoreboard.md'], root);
  run('git', ['commit', '-m', 'init'], root);
  run('git', ['worktree', 'add', path.join(root, '.worktrees', 'lane-1-node'), '-b', 'lane-1-node'], root);

  const originalCwd = process.cwd();
  const modulePath = path.join(originalCwd, 'NLSDD', 'scripts', 'nlsdd-lib.cjs');
  delete process.env.NLSDD_PROJECT_ROOT;
  try {
    const worktreeRoot = path.join(root, '.worktrees', 'lane-1-node');
    fs.writeFileSync(path.join(worktreeRoot, 'NLSDD', 'scoreboard.md'), '# worktree scoreboard\n', 'utf8');
    process.chdir(worktreeRoot);
    delete require.cache[require.resolve(modulePath)];
    const {resolveProjectRoot} = require(modulePath);
    assert.equal(resolveProjectRoot(), worktreeRoot);
  } finally {
    process.chdir(originalCwd);
  }
});

test('schedule helper prefers the current linked worktree NLSDD surface over the canonical repo root', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-worktree-'));
  fs.mkdirSync(path.join(root, '.worktrees'), {recursive: true});
  run('git', ['init', '-q'], root);
  run('git', ['config', 'user.name', 'Codex Test'], root);
  run('git', ['config', 'user.email', 'codex@example.com'], root);
  fs.mkdirSync(path.join(root, 'NLSDD', 'executions', 'plot-mode'), {recursive: true});
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'scoreboard.md'),
    `# NLSDD Scoreboard

| Execution | Lane | Ownership | Current item | Phase | Item commit | Last verification | Blocked by | Next refill target | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| plot-mode | Lane 1 | root ownership | Root row | parked | \`root000\` | \`root verify\` | none | none | root |
`,
    'utf8',
  );
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'executions', 'plot-mode', 'lane-1.md'),
    `# Lane 1

> NLSDD worktree: \`.worktrees/lane-1-node\`
>
> Lane-local verification:
> \`echo root\`

## M - Model

- [ ] Root item
`,
    'utf8',
  );
  run('git', ['add', '.'], root);
  run('git', ['commit', '-m', 'init'], root);
  run('git', ['worktree', 'add', path.join(root, '.worktrees', 'lane-1-node'), '-b', 'lane-1-node'], root);

  const worktreeRoot = path.join(root, '.worktrees', 'lane-1-node');
  fs.writeFileSync(
    path.join(worktreeRoot, 'NLSDD', 'scoreboard.md'),
    `# NLSDD Scoreboard

| Execution | Lane | Ownership | Current item | Phase | Item commit | Last verification | Blocked by | Next refill target | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| plot-mode | Lane 1 | worktree ownership | Worktree row | refill-ready | \`tree111\` | \`tree verify\` | none | Worktree item | worktree |
`,
    'utf8',
  );
  fs.writeFileSync(
    path.join(worktreeRoot, 'NLSDD', 'executions', 'plot-mode', 'lane-1.md'),
    `# Lane 1

> NLSDD worktree: \`.worktrees/lane-1-node\`
>
> Lane-local verification:
> \`echo worktree\`

## M - Model

- [ ] Worktree item
`,
    'utf8',
  );

  const originalCwd = process.cwd();
  const modulePath = path.join(originalCwd, 'NLSDD', 'scripts', 'nlsdd-lib.cjs');
  try {
    delete process.env.NLSDD_PROJECT_ROOT;
    delete process.env.NLSDD_SCOREBOARD_PATH;
    delete process.env.NLSDD_RUNTIME_SCOREBOARD_PATH;
    process.chdir(worktreeRoot);
    delete require.cache[require.resolve(modulePath)];
    const {computeExecutionSchedule, resolveProjectRoot} = require(modulePath);
    const schedule = computeExecutionSchedule(resolveProjectRoot(), 'plot-mode', 4);
    assert.equal(schedule.refillReadyRows.some((row) => row['Current item'] === 'Worktree row'), true);
    assert.equal(schedule.refillReadyRows.some((row) => row['Current item'] === 'Root row'), false);
    assert.equal(schedule.dispatchSuggestions[0].nextItem, 'Worktree item');
  } finally {
    process.chdir(originalCwd);
  }
});

test('lane plans in a linked worktree still resolve shared .worktrees paths from the canonical worktree pool root', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-pool-root-'));
  fs.mkdirSync(path.join(root, '.worktrees'), {recursive: true});
  run('git', ['init', '-q'], root);
  run('git', ['config', 'user.name', 'Codex Test'], root);
  run('git', ['config', 'user.email', 'codex@example.com'], root);
  fs.mkdirSync(path.join(root, 'NLSDD', 'executions', 'plot-mode'), {recursive: true});
  fs.writeFileSync(path.join(root, 'NLSDD', 'scoreboard.md'), '# root scoreboard\n', 'utf8');
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'executions', 'plot-mode', 'lane-1.md'),
    `# Lane 1

> NLSDD worktree: \`.worktrees/lane-1-node\`
>
> Lane-local verification:
> \`echo root\`

## M - Model

- [ ] Root item
`,
    'utf8',
  );
  run('git', ['add', '.'], root);
  run('git', ['commit', '-m', 'init'], root);
  run('git', ['worktree', 'add', path.join(root, '.worktrees', 'lane-1-node'), '-b', 'lane-1-node'], root);

  const recoveryRoot = path.join(root, '.worktrees', 'recovery');
  run('git', ['worktree', 'add', recoveryRoot, '-b', 'recovery'], root);
  fs.mkdirSync(path.join(recoveryRoot, 'NLSDD', 'executions', 'plot-mode'), {recursive: true});
  fs.writeFileSync(path.join(recoveryRoot, 'NLSDD', 'scoreboard.md'), '# recovery scoreboard\n', 'utf8');
  fs.writeFileSync(
    path.join(recoveryRoot, 'NLSDD', 'executions', 'plot-mode', 'lane-1.md'),
    `# Lane 1

> NLSDD worktree: \`.worktrees/lane-1-node\`
>
> Lane-local verification:
> \`echo recovery\`

## M - Model

- [ ] Recovery item
`,
    'utf8',
  );

  const originalCwd = process.cwd();
  const modulePath = path.join(originalCwd, 'NLSDD', 'scripts', 'nlsdd-lib.cjs');
  try {
    delete process.env.NLSDD_PROJECT_ROOT;
    delete process.env.NLSDD_WORKTREE_POOL_ROOT;
    process.chdir(recoveryRoot);
    delete require.cache[require.resolve(modulePath)];
    const {loadLanePlan, resolveProjectRoot, resolveWorktreePoolRoot} = require(modulePath);
    const projectRoot = resolveProjectRoot();
    const lanePlan = loadLanePlan(projectRoot, 'plot-mode', 'Lane 1');

    assert.equal(projectRoot, recoveryRoot);
    assert.equal(resolveWorktreePoolRoot(projectRoot), root);
    assert.equal(lanePlan.worktreePath, path.join(root, '.worktrees', 'lane-1-node'));
    assert.equal(lanePlan.actionableItems[0].text, 'Recovery item');
  } finally {
    process.chdir(originalCwd);
  }
});

test('probe helper reports source changes, artifact noise, and verification commands', () => {
  const fixture = setupNlsddFixture();
  writeLaneState(fixture.root, 'plot-mode', 1, {
    execution: 'plot-mode',
    lane: 'Lane 1',
    phase: 'quality-review-pending',
    expectedNextPhase: 'refill-ready',
    latestCommit: 'feedbee',
    lastReviewerResult: 'PASS',
    correctionCount: 2,
    updatedAt: '2026-03-21T03:30:00.000Z',
  });
  const {probeLane} = freshRequire('NLSDD/scripts/nlsdd-probe-lane.cjs');

  const result = probeLane(fixture.root, 'plot-mode', 'Lane 1');

  assert.equal(result.execution, 'plot-mode');
  assert.equal(result.lane, 'Lane 1');
  assert.equal(result.sourcePaths.includes('tracked.txt'), true);
  assert.equal(
    result.artifactPaths.includes('rust/plot-viewer/target/noise.bin'),
    true,
  );
  assert.equal(result.noise, 'mixed');
  assert.equal(result.laneState.phase, 'quality-review-pending');
  assert.equal(result.laneState.expectedNextPhase, 'refill-ready');
  assert.deepEqual(
    result.verificationResults.map((entry) => entry.command),
    ['git status --short', 'git rev-parse --short HEAD'],
  );
});

test('lane automation prefers execution-aware lane state journal over shared lane-number heuristics', () => {
  const fixture = setupNlsddFixture();
  process.env.NLSDD_PROJECT_ROOT = fixture.root;
  process.env.CODEX_STATE_DB_PATH = fixture.dbPath;
  process.env.CODEX_SESSIONS_ROOT = path.join(fixture.root, '.codex', 'sessions');

  writeLaneState(fixture.root, 'plot-mode', 1, {
    execution: 'plot-mode',
    lane: 'Lane 1',
    phase: 'quality-review-pending',
    expectedNextPhase: 'refill-ready',
    latestCommit: 'feedbee',
    lastVerification: ['git status --short'],
    lastReviewerResult: 'PASS',
    correctionCount: 7,
    updatedAt: '2026-03-21T03:30:00.000Z',
  });

  const {computeLaneAutomation} = freshRequire('NLSDD/scripts/nlsdd-lib.cjs');
  const automation = computeLaneAutomation(fixture.root, 'plot-mode', 'Lane 1', 'spec-review-pending');

  assert.equal(automation.effectivePhase, 'quality-review-pending');
  assert.equal(automation.latestEventText, 'PASS · journal · 2026-03-21 03:30:00Z');
  assert.equal(automation.correctionCount, 7);
  assert.equal(automation.lastActivityText, '2026-03-21 03:30:00Z');
});

test('refill assistant suggests the next unchecked lane-local item for refill-ready lanes', () => {
  const fixture = setupNlsddFixture();
  const scoreboardPath = path.join(fixture.root, 'NLSDD', 'scoreboard.md');
  const scoreboardText = fs
    .readFileSync(scoreboardPath, 'utf8')
    .replace('manual-review-needed', 'refill-ready')
    .replace('spec-review-pending', 'refill-ready');
  fs.writeFileSync(scoreboardPath, scoreboardText, 'utf8');

  process.env.NLSDD_PROJECT_ROOT = fixture.root;
  process.env.NLSDD_SCOREBOARD_PATH = scoreboardPath;

  const {suggestRefill} = freshRequire('NLSDD/scripts/nlsdd-suggest-refill.cjs');
  const suggestion = suggestRefill(fixture.root, 'plot-mode', 'Lane 1');

  assert.equal(suggestion.outcome, 'refill-target');
  assert.equal(
    suggestion.nextItem,
    'Tighten snapshot builder semantics when real 7d history evolves',
  );
  assert.equal(suggestion.nextItemSection, 'M - Model / Contract');
});

test('self-hosting schedule keeps the active thread cap at four and dispatches refill-ready lanes first', () => {
  const fixture = setupSelfHostingScheduleFixture();
  const {computeExecutionSchedule} = freshRequire('NLSDD/scripts/nlsdd-lib.cjs');
  const {renderSchedule} = freshRequire('NLSDD/scripts/nlsdd-suggest-schedule.cjs');

  const schedule = computeExecutionSchedule(fixture.root, 'nlsdd-self-hosting', 4);
  const output = renderSchedule(schedule);

  assert.equal(schedule.maxActiveThreads, 4);
  assert.equal(schedule.activeRows.length, 2);
  assert.equal(schedule.availableSlots, 2);
  assert.deepEqual(
    schedule.dispatchSuggestions.map((suggestion) => suggestion.lane),
    ['Lane 3', 'Lane 4'],
  );
  assert.deepEqual(
    schedule.dispatchSuggestions.map((suggestion) => suggestion.nextItem),
    ['Rewrite remaining fixed-lane wording', 'Add schedule regression coverage'],
  );
  assert.deepEqual(
    schedule.queuedRows.map((row) => row.Lane),
    ['Lane 5', 'Lane 6'],
  );
  assert.match(output, /Available slots: 2/);
  assert.ok(output.indexOf('Lane 3 (refill-ready)') < output.indexOf('Lane 4 (refill-ready)'));
  assert.ok(output.indexOf('Refill-ready lanes:') < output.indexOf('Queued lanes:'));
});

test('schedule suggestion prefers lane journal phase over stale scoreboard phase', () => {
  const fixture = setupSelfHostingScheduleFixture();
  writeLaneState(fixture.root, 'nlsdd-self-hosting', 5, {
    execution: 'nlsdd-self-hosting',
    lane: 'Lane 5',
    phase: 'refill-ready',
    expectedNextPhase: 'implementing',
    latestCommit: 'ab12cd3',
    lastReviewerResult: 'PASS',
    correctionCount: 0,
    updatedAt: '2026-03-21T04:00:00.000Z',
  });

  const {computeExecutionSchedule} = freshRequire('NLSDD/scripts/nlsdd-lib.cjs');
  const schedule = computeExecutionSchedule(fixture.root, 'nlsdd-self-hosting', 4);

  assert.equal(schedule.refillReadyRows.some((row) => row.Lane === 'Lane 5'), true);
  assert.equal(schedule.queuedRows.some((row) => row.Lane === 'Lane 5'), false);
});

test('schedule and refill helpers prefer runtime scoreboard when present', () => {
  const fixture = setupNlsddFixture();
  const runtimeScoreboardPath = path.join(
    fixture.root,
    'NLSDD',
    'state',
    'scoreboard.runtime.md',
  );
  fs.mkdirSync(path.dirname(runtimeScoreboardPath), {recursive: true});
  fs.writeFileSync(
    runtimeScoreboardPath,
    `# NLSDD Scoreboard

| Execution | Lane | Ownership | Current item | Phase | Effective phase | Item commit | Branch HEAD | Last verification | Last probe | Latest event | Correction count | Last activity | Blocked by | Next refill target | Noise | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| plot-mode | Lane 1 | Node contract + handoff | Viewer launch confidence / shell messaging | queued | refill-ready | \`abc1234\` | \`abc1234\` | \`git status --short\` | n/a | PASS · runtime · 2026-03-21 03:40:00Z | 0 | 2026-03-21 03:40:00Z | none | Tighten snapshot builder semantics when real 7d history evolves | none | runtime row |
`,
    'utf8',
  );

  process.env.NLSDD_PROJECT_ROOT = fixture.root;
  process.env.NLSDD_RUNTIME_SCOREBOARD_PATH = runtimeScoreboardPath;

  const {computeExecutionSchedule} = freshRequire('NLSDD/scripts/nlsdd-lib.cjs');
  const {suggestRefill} = freshRequire('NLSDD/scripts/nlsdd-suggest-refill.cjs');

  const schedule = computeExecutionSchedule(fixture.root, 'plot-mode', 4);
  const suggestion = suggestRefill(fixture.root, 'plot-mode', 'Lane 1');

  assert.equal(schedule.refillReadyRows.some((row) => row.Lane === 'Lane 1'), true);
  assert.equal(schedule.queuedRows.some((row) => row.Lane === 'Lane 1'), false);
  assert.equal(suggestion.outcome, 'refill-target');
  assert.equal(
    suggestion.nextItem,
    'Tighten snapshot builder semantics when real 7d history evolves',
  );
});

test('message helper renders correction-loop text without opening direct reviewer channels', () => {
  const {composeMessage} = freshRequire('NLSDD/scripts/nlsdd-compose-message.cjs');
  const message = composeMessage({
    phase: 'correction-loop',
    execution: 'plot-mode',
    lane: '1',
    item: 'Viewer launch confidence / shell messaging',
    commit: '1d29843',
    'fail-reason': 'FAIL [src/commands/root.ts:10] missing retry hint',
    scope: 'src/commands/root.ts and tests/plot-mode-shell.test.js',
    verification: 'npm run build && node --test tests/plot-mode-shell.test.js',
    files: 'src/commands/root.ts, tests/plot-mode-shell.test.js',
  });

  assert.match(message, /Execution: plot-mode/);
  assert.match(message, /Lane: Lane 1/);
  assert.match(message, /Failing commit: 1d29843/);
  assert.match(message, /Reviewer finding: FAIL \[src\/commands\/root\.ts:10\] missing retry hint/);
  assert.match(
    message,
    /Return a new commit sha and verification results, or READY_TO_COMMIT with a commit-ready summary if coordinator commit is required/,
  );
  assert.doesNotMatch(message, /talk directly to reviewer/i);
});

test('message helper tells implementers to hand commit-ready mvc back to coordinator when commit may be gated', () => {
  const {composeMessage} = freshRequire('NLSDD/scripts/nlsdd-compose-message.cjs');
  const message = composeMessage({
    phase: 'implementer-assignment',
    execution: 'plot-mode',
    lane: '4',
    item: 'Recommendation-rich compare panel',
    scope: 'rust/plot-viewer/src/render/panels.rs and related tests',
    verification: 'cargo test --manifest-path rust/plot-viewer/Cargo.toml render::panels',
  });

  assert.match(message, /Execution: plot-mode/);
  assert.match(message, /Lane: Lane 4/);
  assert.match(message, /commit sha or READY_TO_COMMIT package/);
  assert.match(message, /Do not run git commit yourself unless this lane explicitly says self-commit is allowed/);
  assert.match(
    message,
    /Default NLSDD flow in this repo: hand back READY_TO_COMMIT with intended commit title\/body summary so coordinator can commit for you/,
  );
});

test('lane state recorder writes execution-aware journal files for coordinator tooling', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-record-'));
  process.env.NLSDD_PROJECT_ROOT = root;

  const {recordLaneState} = freshRequire('NLSDD/scripts/nlsdd-record-lane-state.cjs');
  const filePath = recordLaneState(root, {
    execution: 'plot-mode',
    lane: 'Lane 2',
    phase: 'quality-review-pending',
    'expected-next-phase': 'refill-ready',
    commit: 'abc1234',
    'commit-title': 'feat(plot): tighten runtime seam',
    'commit-body': '補上 compare payload handoff',
    reviewer: 'PASS',
    'correction-count': 3,
    verification: ['cargo test', 'cargo check'],
    'blocked-by': 'none',
    note: 'ready for quality review',
    'updated-at': '2026-03-21T04:20:00.000Z',
  });

  const state = JSON.parse(fs.readFileSync(filePath, 'utf8'));
  assert.equal(state.execution, 'plot-mode');
  assert.equal(state.lane, 'Lane 2');
  assert.equal(state.phase, 'quality-review-pending');
  assert.equal(state.expectedNextPhase, 'refill-ready');
  assert.equal(state.latestCommit, 'abc1234');
  assert.equal(state.proposedCommitTitle, 'feat(plot): tighten runtime seam');
  assert.equal(state.proposedCommitBody, '補上 compare payload handoff');
  assert.equal(state.lastReviewerResult, 'PASS');
  assert.deepEqual(state.lastVerification, ['cargo test', 'cargo check']);
  assert.equal(state.blockedBy, 'none');
  assert.equal(state.note, 'ready for quality review');
  assert.equal(state.correctionCount, 3);
  assert.equal(state.updatedAt, '2026-03-21T04:20:00.000Z');
});

test('execution insight recorder appends subagent and coordinator learnings into runtime insights journal', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-insight-'));
  process.env.NLSDD_PROJECT_ROOT = root;

  const {recordInsight} = freshRequire('NLSDD/scripts/nlsdd-record-insight.cjs');

  const first = recordInsight(root, {
    execution: 'plot-mode',
    lane: '4',
    source: 'subagent',
    kind: 'suggestion',
    status: 'open',
    summary: 'Need render-boundary compare payload in Lane 2',
    detail: 'Lane 4 cannot consume the richer compare seam until render/mod.rs exposes it.',
    'related-lane': '2',
    'related-agent': 'Rawls',
    timestamp: '2026-03-21T07:30:00.000Z',
  });

  const second = recordInsight(root, {
    execution: 'plot-mode',
    lane: 'global',
    source: 'coordinator',
    kind: 'observed-issue',
    status: 'adopted',
    summary: 'Commit responsibility must be split by role',
    detail: 'Main agent direct work can auto-commit; NLSDD subagents should default to READY_TO_COMMIT.',
    'related-commit': 'cd5070c',
    'recorded-by': 'main-agent',
    timestamp: '2026-03-21T07:31:00.000Z',
  });

  assert.equal(first.filePath, second.filePath);
  const lines = fs.readFileSync(first.filePath, 'utf8').trim().split('\n');
  assert.equal(lines.length, 2);

  const firstEntry = JSON.parse(lines[0]);
  assert.equal(firstEntry.execution, 'plot-mode');
  assert.equal(firstEntry.lane, 'Lane 4');
  assert.equal(firstEntry.source, 'subagent');
  assert.equal(firstEntry.kind, 'suggestion');
  assert.equal(firstEntry.relatedLane, 'Lane 2');
  assert.equal(firstEntry.relatedAgent, 'Rawls');

  const secondEntry = JSON.parse(lines[1]);
  assert.equal(secondEntry.lane, 'global');
  assert.equal(secondEntry.source, 'coordinator');
  assert.equal(secondEntry.kind, 'observed-issue');
  assert.equal(secondEntry.status, 'adopted');
  assert.equal(secondEntry.relatedCommit, 'cd5070c');
  assert.equal(secondEntry.recordedBy, 'main-agent');
});

test('active-set replan helper updates tracked phases and lane journals atomically', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-replan-'));
  process.env.NLSDD_PROJECT_ROOT = root;

  fs.mkdirSync(path.join(root, 'NLSDD', 'executions', 'plot-mode'), {recursive: true});
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'scoreboard.md'),
    `# NLSDD Scoreboard

| Execution | Lane | Ownership | Current item | Phase | Item commit | Last verification | Blocked by | Next refill target | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| plot-mode | Lane 1 | Node contract + handoff | Future shell polish only if Rust viewer launch UX changes again | refill-ready | \`baa7b8e\` | \`npm run build\`; \`node --test tests/plot-snapshot.test.js\` | none | Re-activate only if shell UX changes | accepted |
| plot-mode | Lane 2 | Rust runtime + boundary | Live profile/focus navigation coherence on the recovery baseline | parked | \`3b62c5b\` | \`cargo test\`; \`cargo check\` | none | Revisit stronger nested usage decode only if needed | should re-activate |
| plot-mode | Lane 3 | Rust chart surface | Chart compatibility with richer focus and profile cycling | refill-ready | \`35c8351\` | \`cargo test render::chart\`; \`cargo check\` | none | Widen only if chart UX still needs it | accepted |
`,
    'utf8',
  );

  writeLaneState(root, 'plot-mode', 1, {
    execution: 'plot-mode',
    lane: 'Lane 1',
    phase: 'refill-ready',
    expectedNextPhase: 'implementing',
    latestCommit: 'baa7b8e',
    lastReviewerResult: 'PASS',
    lastVerification: ['npm run build'],
    blockedBy: null,
    note: 'accepted lane 1 state',
    correctionCount: 0,
    updatedAt: '2026-03-21T07:00:00.000Z',
  });
  writeLaneState(root, 'plot-mode', 2, {
    execution: 'plot-mode',
    lane: 'Lane 2',
    phase: 'implementing',
    expectedNextPhase: 'spec-review-pending',
    latestCommit: '19ebd40',
    lastReviewerResult: null,
    lastVerification: [],
    blockedBy: null,
    note: 'stale dispatch state',
    correctionCount: 0,
    updatedAt: '2026-03-21T07:00:00.000Z',
  });
  writeLaneState(root, 'plot-mode', 3, {
    execution: 'plot-mode',
    lane: 'Lane 3',
    phase: 'implementing',
    expectedNextPhase: 'spec-review-pending',
    latestCommit: '19ebd40',
    lastReviewerResult: null,
    lastVerification: [],
    blockedBy: null,
    note: 'stale dispatch state',
    correctionCount: 0,
    updatedAt: '2026-03-21T07:00:00.000Z',
  });

  const {replanActiveSet} = freshRequire('NLSDD/scripts/nlsdd-replan-active-set.cjs');
  replanActiveSet(root, {
    execution: 'plot-mode',
    active: ['Lane 2', 'Lane 3'],
    parked: ['Lane 1'],
    note: 'manual 4a replan',
    'updated-at': '2026-03-21T08:00:00.000Z',
  });

  const scoreboardText = fs.readFileSync(path.join(root, 'NLSDD', 'scoreboard.md'), 'utf8');
  assert.match(scoreboardText, /\| plot-mode \| Lane 1 \| Node contract \+ handoff \| Future shell polish only if Rust viewer launch UX changes again \| parked \|/);
  assert.match(scoreboardText, /\| plot-mode \| Lane 2 \| Rust runtime \+ boundary \| Live profile\/focus navigation coherence on the recovery baseline \| queued \|/);
  assert.match(scoreboardText, /\| plot-mode \| Lane 3 \| Rust chart surface \| Chart compatibility with richer focus and profile cycling \| queued \|/);

  const lane1State = JSON.parse(
    fs.readFileSync(path.join(root, 'NLSDD', 'state', 'plot-mode', 'lane-1.json'), 'utf8'),
  );
  const lane2State = JSON.parse(
    fs.readFileSync(path.join(root, 'NLSDD', 'state', 'plot-mode', 'lane-2.json'), 'utf8'),
  );
  const lane3State = JSON.parse(
    fs.readFileSync(path.join(root, 'NLSDD', 'state', 'plot-mode', 'lane-3.json'), 'utf8'),
  );

  assert.equal(lane1State.phase, 'parked');
  assert.equal(lane1State.expectedNextPhase, null);
  assert.equal(lane1State.latestCommit, 'baa7b8e');
  assert.equal(lane1State.note, 'manual 4a replan');

  assert.equal(lane2State.phase, 'queued');
  assert.equal(lane2State.expectedNextPhase, 'implementing');
  assert.equal(lane2State.latestCommit, '3b62c5b');
  assert.deepEqual(lane2State.lastVerification, ['cargo test', 'cargo check']);

  assert.equal(lane3State.phase, 'queued');
  assert.equal(lane3State.expectedNextPhase, 'implementing');
  assert.equal(lane3State.latestCommit, '35c8351');

  const {computeExecutionSchedule} = freshRequire('NLSDD/scripts/nlsdd-lib.cjs');
  const schedule = computeExecutionSchedule(root, 'plot-mode', 4);
  assert.equal(schedule.activeRows.length, 0);
  assert.deepEqual(
    schedule.queuedRows.map((row) => row.Lane),
    ['Lane 2', 'Lane 3'],
  );
  assert.deepEqual(
    schedule.refillReadyRows.map((row) => row.Lane),
    [],
  );
});

test('schedule marks clean implementing lane at same HEAD as stale-implementing', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-stale-'));
  const laneWorktree = path.join(root, '.worktrees', 'lane-1-node');
  setupTempGitRepo(laneWorktree);
  process.env.NLSDD_PROJECT_ROOT = root;

  const head = run('git', ['rev-parse', '--short', 'HEAD'], laneWorktree);

  fs.mkdirSync(path.join(root, 'NLSDD', 'executions', 'plot-mode'), {recursive: true});
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'executions', 'plot-mode', 'lane-1.md'),
    `# Lane 1 Plan - Node Contract and Handoff

> Ownership family:
> \`src/commands/root.ts\`
>
> NLSDD worktree: \`.worktrees/lane-1-node\`
>
> Lane-local verification:
> \`git status --short\`

## C - Controller / Handoff

- [ ] Audit shell/handoff alignment
`,
    'utf8',
  );

  fs.writeFileSync(
    path.join(root, 'NLSDD', 'scoreboard.md'),
    `# NLSDD Scoreboard

| Execution | Lane | Ownership | Current item | Phase | Item commit | Last verification | Blocked by | Next refill target | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| plot-mode | Lane 1 | Node contract + handoff | Audit shell/handoff alignment | implementing | \`${head}\` | \`git status --short\` | none | none | stale implementing candidate |
`,
    'utf8',
  );

  writeLaneState(root, 'plot-mode', 1, {
    execution: 'plot-mode',
    lane: 'Lane 1',
    phase: 'implementing',
    expectedNextPhase: 'spec-review-pending',
    latestCommit: head,
    lastReviewerResult: null,
    lastVerification: ['git status --short'],
    blockedBy: null,
    note: 'stale implementing candidate',
    correctionCount: 0,
    updatedAt: '2020-01-01T00:00:00.000Z',
  });

  const {computeExecutionSchedule} = freshRequire('NLSDD/scripts/nlsdd-lib.cjs');
  const schedule = computeExecutionSchedule(root, 'plot-mode', 4);

  assert.equal(schedule.activeRows.length, 0);
  assert.deepEqual(schedule.staleRows.map((row) => row.Lane), ['Lane 1']);
  assert.equal(schedule.staleRows[0].staleImplementing.kind, 'stale-implementing');
  assert.equal(schedule.availableSlots, 4);
});

test('dispatch cycle reconciles stale lanes and promotes next queued work', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-cycle-'));
  process.env.NLSDD_PROJECT_ROOT = root;

  const lane1Worktree = path.join(root, '.worktrees', 'lane-1-node');
  const lane2Worktree = path.join(root, '.worktrees', 'lane-2-runtime');
  setupTempGitRepo(lane1Worktree);
  setupTempGitRepo(lane2Worktree);

  const lane1Head = run('git', ['rev-parse', '--short', 'HEAD'], lane1Worktree);
  const lane2Head = run('git', ['rev-parse', '--short', 'HEAD'], lane2Worktree);

  fs.mkdirSync(path.join(root, 'NLSDD', 'executions', 'plot-mode'), {recursive: true});
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'executions', 'plot-mode', 'lane-1.md'),
    `# Lane 1

> Ownership family:
> \`src/commands/root.ts\`
>
> NLSDD worktree: \`.worktrees/lane-1-node\`
>
> Lane-local verification:
> \`git status --short\`

## C - Controller

- [ ] Future shell audit
`,
    'utf8',
  );
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'executions', 'plot-mode', 'lane-2.md'),
    `# Lane 2

> Ownership family:
> \`rust/plot-viewer/src/render/mod.rs\`
>
> NLSDD worktree: \`.worktrees/lane-2-runtime\`
>
> Lane-local verification:
> \`git status --short\`

## V - View

- [ ] Runtime compare seam follow-up
`,
    'utf8',
  );

  fs.writeFileSync(
    path.join(root, 'NLSDD', 'scoreboard.md'),
    `# NLSDD Scoreboard

| Execution | Lane | Ownership | Current item | Phase | Item commit | Last verification | Blocked by | Next refill target | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| plot-mode | Lane 1 | Node contract + handoff | Future shell audit | parked | \`${lane1Head}\` | \`git status --short\` | none | none | parked truth |
| plot-mode | Lane 2 | Rust runtime + boundary | Runtime compare seam follow-up | queued | \`${lane2Head}\` | \`git status --short\` | none | next runtime step | queued truth |
`,
    'utf8',
  );

  writeLaneState(root, 'plot-mode', 1, {
    execution: 'plot-mode',
    lane: 'Lane 1',
    phase: 'implementing',
    expectedNextPhase: 'spec-review-pending',
    latestCommit: lane1Head,
    lastReviewerResult: null,
    lastVerification: ['git status --short'],
    blockedBy: null,
    note: 'stale implementing lane',
    correctionCount: 0,
    updatedAt: '2020-01-01T00:00:00.000Z',
  });
  writeLaneState(root, 'plot-mode', 2, {
    execution: 'plot-mode',
    lane: 'Lane 2',
    phase: 'queued',
    expectedNextPhase: 'implementing',
    latestCommit: lane2Head,
    lastReviewerResult: null,
    lastVerification: ['git status --short'],
    blockedBy: null,
    note: 'next lane ready',
    correctionCount: 0,
    updatedAt: '2026-03-21T10:00:00.000Z',
  });

  const {runCycle} = freshRequire('NLSDD/scripts/nlsdd-run-cycle.cjs');
  const result = runCycle(root, 'plot-mode', 2, false);

  assert.deepEqual(result.reconciled.map((entry) => entry.lane), ['Lane 1']);
  assert.deepEqual(result.promoted.map((entry) => entry.lane), ['Lane 2']);
  assert.equal(result.idleSlots, 1);

  const lane1State = JSON.parse(
    fs.readFileSync(path.join(root, 'NLSDD', 'state', 'plot-mode', 'lane-1.json'), 'utf8'),
  );
  const lane2State = JSON.parse(
    fs.readFileSync(path.join(root, 'NLSDD', 'state', 'plot-mode', 'lane-2.json'), 'utf8'),
  );
  assert.equal(lane1State.phase, 'parked');
  assert.equal(lane2State.phase, 'implementing');

  const finalSchedule = result.finalSchedule;
  assert.deepEqual(finalSchedule.activeRows.map((row) => row.Lane), ['Lane 2']);
  assert.deepEqual(finalSchedule.staleRows.map((row) => row.Lane), []);
});

test('launch helper returns assignment bundles for newly promoted lanes', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-launch-'));
  process.env.NLSDD_PROJECT_ROOT = root;

  const lane1Worktree = path.join(root, '.worktrees', 'lane-1-node');
  const lane2Worktree = path.join(root, '.worktrees', 'lane-2-runtime');
  setupTempGitRepo(lane1Worktree);
  setupTempGitRepo(lane2Worktree);

  const lane1Head = run('git', ['rev-parse', '--short', 'HEAD'], lane1Worktree);
  const lane2Head = run('git', ['rev-parse', '--short', 'HEAD'], lane2Worktree);

  fs.mkdirSync(path.join(root, 'NLSDD', 'executions', 'plot-mode'), {recursive: true});
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'executions', 'plot-mode', 'lane-1.md'),
    `# Lane 1

> Ownership family:
> \`src/commands/root.ts\`
>
> NLSDD worktree: \`.worktrees/lane-1-node\`
>
> Lane-local verification:
> \`git status --short\`

## C - Controller

- [ ] Future shell audit
`,
    'utf8',
  );
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'executions', 'plot-mode', 'lane-2.md'),
    `# Lane 2

> Ownership family:
> \`rust/plot-viewer/src/render/mod.rs\`
>
> NLSDD worktree: \`.worktrees/lane-2-runtime\`
>
> Lane-local verification:
> \`git status --short\`
> \`cargo check --manifest-path rust/plot-viewer/Cargo.toml\`

## V - View

- [ ] Runtime compare seam follow-up
`,
    'utf8',
  );

  fs.writeFileSync(
    path.join(root, 'NLSDD', 'scoreboard.md'),
    `# NLSDD Scoreboard

| Execution | Lane | Ownership | Current item | Phase | Item commit | Last verification | Blocked by | Next refill target | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| plot-mode | Lane 1 | Node contract + handoff | Future shell audit | parked | \`${lane1Head}\` | \`git status --short\` | none | none | parked truth |
| plot-mode | Lane 2 | Rust runtime + boundary | Runtime compare seam follow-up | queued | \`${lane2Head}\` | \`git status --short\` | none | next runtime step | queued truth |
`,
    'utf8',
  );

  writeLaneState(root, 'plot-mode', 1, {
    execution: 'plot-mode',
    lane: 'Lane 1',
    phase: 'implementing',
    expectedNextPhase: 'spec-review-pending',
    latestCommit: lane1Head,
    lastReviewerResult: null,
    lastVerification: ['git status --short'],
    blockedBy: null,
    note: 'stale implementing lane',
    correctionCount: 0,
    updatedAt: '2020-01-01T00:00:00.000Z',
  });
  writeLaneState(root, 'plot-mode', 2, {
    execution: 'plot-mode',
    lane: 'Lane 2',
    phase: 'queued',
    expectedNextPhase: 'implementing',
    latestCommit: lane2Head,
    lastReviewerResult: null,
    lastVerification: ['git status --short'],
    blockedBy: null,
    note: 'next lane ready',
    correctionCount: 0,
    updatedAt: '2026-03-21T10:00:00.000Z',
  });

  const {launchActiveSet} = freshRequire('NLSDD/scripts/nlsdd-launch-active-set.cjs');
  const result = launchActiveSet(root, 'plot-mode', 2, false);

  assert.deepEqual(result.completedLanes, ['Lane 1']);
  assert.deepEqual(result.promoted.map((entry) => entry.lane), ['Lane 2']);
  assert.equal(result.assignments.length, 1);
  assert.equal(result.assignments[0].lane, 'Lane 2');
  assert.equal(result.assignments[0].nextItem, 'Runtime compare seam follow-up');
  assert.deepEqual(result.assignments[0].verification, [
    'git status --short',
    'cargo check --manifest-path rust/plot-viewer/Cargo.toml',
  ]);
  assert.match(result.assignments[0].scope, /rust\/plot-viewer\/src\/render\/mod\.rs/);
  assert.match(result.assignments[0].message, /Execution: plot-mode/);
  assert.match(result.assignments[0].message, /Lane: Lane 2/);
  assert.match(result.assignments[0].message, /Lane item intent: Runtime compare seam follow-up/);
  assert.match(result.assignments[0].message, /Write scope: rust\/plot-viewer\/src\/render\/mod\.rs/);
});

test('review loop driver returns coordinator-ready bundles for review and correction phases', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-review-'));
  process.env.NLSDD_PROJECT_ROOT = root;

  fs.mkdirSync(path.join(root, 'NLSDD', 'executions', 'plot-mode'), {recursive: true});
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'executions', 'plot-mode', 'lane-2.md'),
    `# Lane 2

> Ownership family:
> \`rust/plot-viewer/src/render/mod.rs\`
>
> NLSDD worktree: \`.worktrees/lane-2-runtime\`
>
> Lane-local verification:
> \`cargo test --manifest-path rust/plot-viewer/Cargo.toml\`
> \`cargo check --manifest-path rust/plot-viewer/Cargo.toml\`

## V - View

- [ ] Runtime compare seam follow-up
`,
    'utf8',
  );
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'executions', 'plot-mode', 'lane-4.md'),
    `# Lane 4

> Ownership family:
> \`rust/plot-viewer/src/render/panels.rs\`
>
> NLSDD worktree: \`.worktrees/lane-4-panels\`
>
> Lane-local verification:
> \`cargo test render_panels_builds_visible_summary_and_compare_blocks --manifest-path rust/plot-viewer/Cargo.toml\`

## C - Controller

- [ ] Recommendation-rich Compare panel on the recovery baseline
`,
    'utf8',
  );
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'scoreboard.md'),
    `# NLSDD Scoreboard

| Execution | Lane | Ownership | Current item | Phase | Item commit | Last verification | Blocked by | Next refill target | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| plot-mode | Lane 2 | Rust runtime + boundary | Runtime compare seam follow-up | spec-review-pending | \`abc1234\` | \`cargo test --manifest-path rust/plot-viewer/Cargo.toml\` | none | none | spec row |
| plot-mode | Lane 4 | Rust panels + docs | Recommendation-rich Compare panel on the recovery baseline | correction | \`def5678\` | \`cargo test render_panels_builds_visible_summary_and_compare_blocks --manifest-path rust/plot-viewer/Cargo.toml\` | none | none | correction row |
`,
    'utf8',
  );

  writeLaneState(root, 'plot-mode', 2, {
    execution: 'plot-mode',
    lane: 'Lane 2',
    phase: 'spec-review-pending',
    expectedNextPhase: 'quality-review-pending',
    latestCommit: 'abc1234',
    lastReviewerResult: null,
    lastVerification: [
      'cargo test --manifest-path rust/plot-viewer/Cargo.toml',
      'cargo check --manifest-path rust/plot-viewer/Cargo.toml',
    ],
    blockedBy: null,
    note: 'spec review next',
    correctionCount: 0,
    updatedAt: '2026-03-21T10:00:00.000Z',
  });
  writeLaneState(root, 'plot-mode', 4, {
    execution: 'plot-mode',
    lane: 'Lane 4',
    phase: 'correction',
    expectedNextPhase: 'spec-review-pending',
    latestCommit: 'def5678',
    lastReviewerResult: 'FAIL',
    lastVerification: [
      'cargo test render_panels_builds_visible_summary_and_compare_blocks --manifest-path rust/plot-viewer/Cargo.toml',
    ],
    blockedBy: null,
    note: 'Compare panel still re-derives label heuristics instead of consuming runtime-owned payload.',
    correctionCount: 1,
    updatedAt: '2026-03-21T10:05:00.000Z',
  });

  const {driveReviewLoop} = freshRequire('NLSDD/scripts/nlsdd-drive-review-loop.cjs');
  const result = driveReviewLoop(root, 'plot-mode');

  assert.equal(result.length, 2);
  assert.equal(result[0].lane, 'Lane 2');
  assert.equal(result[0].action, 'spec-review');
  assert.match(result[0].message, /Review target commit: abc1234/);

  assert.equal(result[1].lane, 'Lane 4');
  assert.equal(result[1].action, 'correction-loop');
  assert.match(result[1].message, /Reviewer finding: Compare panel still re-derives label heuristics/);
  assert.match(result[1].message, /Accepted write scope: rust\/plot-viewer\/src\/render\/panels\.rs/);
});

test('ready-to-commit intake helper returns structured coordinator commit bundle', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-intake-'));
  process.env.NLSDD_PROJECT_ROOT = root;

  fs.mkdirSync(path.join(root, 'NLSDD', 'executions', 'plot-mode'), {recursive: true});
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'executions', 'plot-mode', 'lane-5.md'),
    `# Lane 5

> Ownership family:
> \`README.md\`
> \`tests/plot-readme.test.js\`
>
> NLSDD worktree: \`.worktrees/lane-5-docs\`
>
> Lane-local verification:
> \`npm run build\`
> \`node --test tests/plot-readme.test.js\`

## C - Controller

- [ ] Recovery-baseline README and local run instructions
`,
    'utf8',
  );
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'scoreboard.md'),
    `# NLSDD Scoreboard

| Execution | Lane | Ownership | Current item | Phase | Item commit | Last verification | Blocked by | Next refill target | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| plot-mode | Lane 5 | Plot viewer docs + operator flow | Recovery-baseline README and local run instructions | coordinator-commit-pending | \`n/a\` | \`npm run build\`; \`node --test tests/plot-readme.test.js\` | none | none | ready to commit |
`,
    'utf8',
  );

  writeLaneState(root, 'plot-mode', 5, {
    execution: 'plot-mode',
    lane: 'Lane 5',
    phase: 'coordinator-commit-pending',
    'expected-next-phase': 'spec-review-pending',
    commit: null,
    'commit-title': 'docs(plot): 補上 recovery baseline 操作說明',
    'commit-body': '同步 shell/readme 驗證與本地啟動步驟',
    reviewer: null,
    verification: ['npm run build', 'node --test tests/plot-readme.test.js'],
    'blocked-by': null,
    note: 'READY_TO_COMMIT from docs lane',
    'updated-at': '2026-03-21T11:00:00.000Z',
  });

  const {intakeReadyToCommit} = freshRequire('NLSDD/scripts/nlsdd-intake-ready-to-commit.cjs');
  const result = intakeReadyToCommit(root, 'plot-mode');

  assert.equal(result.length, 1);
  assert.equal(result[0].lane, 'Lane 5');
  assert.equal(result[0].phase, 'coordinator-commit-pending');
  assert.equal(result[0].proposedCommitTitle, 'docs(plot): 補上 recovery baseline 操作說明');
  assert.equal(result[0].proposedCommitBody, '同步 shell/readme 驗證與本地啟動步驟');
  assert.deepEqual(result[0].verification, ['npm run build', 'node --test tests/plot-readme.test.js']);
  assert.deepEqual(result[0].scope, ['README.md', 'tests/plot-readme.test.js']);
  assert.match(result[0].note, /READY_TO_COMMIT/);
});
