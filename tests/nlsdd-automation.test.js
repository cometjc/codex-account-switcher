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

function writeTelemetryEvents(root, execution, events) {
  const stateDir = path.join(root, 'NLSDD', 'state', execution);
  fs.mkdirSync(stateDir, {recursive: true});
  fs.writeFileSync(
    path.join(stateDir, 'events.ndjson'),
    `${events.map((event) => JSON.stringify(event)).join('\n')}\n`,
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

test('schedule and refill helpers prefer reduced envelope state over stale tracked rows', () => {
  const fixture = setupNlsddFixture();

  process.env.NLSDD_PROJECT_ROOT = fixture.root;

  const {recordEnvelope} = freshRequire('NLSDD/scripts/nlsdd-envelope.cjs');
  const {suggestRefill} = freshRequire('NLSDD/scripts/nlsdd-suggest-refill.cjs');
  const {computeExecutionSchedule} = freshRequire('NLSDD/scripts/nlsdd-lib.cjs');

  recordEnvelope(fixture.root, {
    execution: 'plot-mode',
    lane: 'Lane 1',
    role: 'implementer',
    eventType: 'pass',
    phaseBefore: 'implementing',
    phaseAfter: 'refill-ready',
    currentItem: 'Viewer launch confidence / shell messaging',
    nextRefillTarget: 'Tighten snapshot builder semantics when real 7d history evolves',
    relatedCommit: 'abc1234',
    verification: ['git status --short'],
    summary: 'Lane 1 current work completed and is ready for refill',
    detail: 'Shift to the next node contract refinement item.',
    nextExpectedPhase: 'implementing',
    timestamp: '2026-03-21T03:40:00.000Z',
  });

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
    /Return a new strict NLSDD lane handoff envelope JSON object, or READY_TO_COMMIT as eventType=ready-to-commit with proposed commit title\/body if coordinator commit is required/,
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
  assert.match(message, /return only one strict NLSDD lane handoff envelope JSON object/i);
  assert.match(message, /Required envelope keys:/);
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
  assert.equal(state.lastReviewerResult, 'pass');
  assert.deepEqual(state.lastVerification, ['cargo test', 'cargo check']);
  assert.equal(state.blockedBy, 'none');
  assert.equal(state.note, 'ready for quality review');
  assert.equal(state.correctionCount, 3);
  assert.equal(state.updatedAt, '2026-03-21T04:20:00.000Z');

  const eventsPath = path.join(root, 'NLSDD', 'state', 'plot-mode', 'events.ndjson');
  const eventLines = fs.readFileSync(eventsPath, 'utf8').trim().split('\n');
  assert.equal(eventLines.length, 1);
  const event = JSON.parse(eventLines[0]);
  assert.equal(event.eventType, 'pass');
  assert.equal(event.phaseAfter, 'quality-review-pending');
});

test('envelope recorder preserves command telemetry fields for command lifecycle events', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-command-envelope-'));
  process.env.NLSDD_PROJECT_ROOT = root;

  const {loadEnvelopeEvents, recordEnvelope} = freshRequire('NLSDD/scripts/nlsdd-envelope.cjs');
  const cases = [
    {
      eventType: 'command-started',
      status: 'started',
      exitCode: null,
      durationMs: null,
      blockKind: null,
      probeSummary: null,
    },
    {
      eventType: 'command-finished',
      status: 'finished',
      exitCode: 0,
      durationMs: 1200,
      blockKind: null,
      probeSummary: null,
    },
    {
      eventType: 'command-failed',
      status: 'failed',
      exitCode: 2,
      durationMs: 2400,
      blockKind: null,
      probeSummary: null,
    },
    {
      eventType: 'command-blocked',
      status: 'blocked',
      exitCode: null,
      durationMs: 9000,
      blockKind: 'dependency',
      probeSummary: null,
    },
    {
      eventType: 'command-probe',
      status: 'probe',
      exitCode: null,
      durationMs: 4500,
      blockKind: null,
      probeSummary: 'stdout silent; worker still alive',
    },
  ];

  for (const entry of cases) {
    recordEnvelope(root, {
      execution: 'plot-mode',
      lane: 'Lane 1',
      role: 'worker',
      eventType: entry.eventType,
      command: 'npm run nlsdd:probe',
      cwd: '/tmp/codex-worker',
      status: entry.status,
      exitCode: entry.exitCode,
      durationMs: entry.durationMs,
      blockKind: entry.blockKind,
      probeSummary: entry.probeSummary,
      pid: 4321,
      summary: `${entry.eventType} telemetry`,
      timestamp: '2026-03-22T01:00:00.000Z',
    });
  }

  const events = loadEnvelopeEvents(root, 'plot-mode').filter((event) =>
    event.eventType.startsWith('command-'),
  );

  assert.equal(events.length, cases.length);
  for (const entry of cases) {
    const event = events.find((candidate) => candidate.eventType === entry.eventType);
    assert.ok(event, `missing ${entry.eventType}`);
    assert.equal(event.command, 'npm run nlsdd:probe');
    assert.equal(event.cwd, '/tmp/codex-worker');
    assert.equal(event.status, entry.status);
    assert.equal(event.exitCode, entry.exitCode);
    assert.equal(event.durationMs, entry.durationMs);
    assert.equal(event.blockKind, entry.blockKind);
    assert.equal(event.probeSummary, entry.probeSummary);
    assert.equal(event.pid, 4321);
  }
});

test('envelope recorder rejects invalid command block kinds', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-command-blockkind-'));
  process.env.NLSDD_PROJECT_ROOT = root;

  const {recordEnvelope} = freshRequire('NLSDD/scripts/nlsdd-envelope.cjs');

  assert.throws(
    () =>
      recordEnvelope(root, {
        execution: 'plot-mode',
        lane: 'Lane 1',
        role: 'worker',
        eventType: 'command-blocked',
        command: 'npm run nlsdd:probe',
        cwd: '/tmp/codex-worker',
        status: 'blocked',
        blockKind: 'unmapped-kind',
        summary: 'command-blocked telemetry',
        timestamp: '2026-03-22T01:00:00.000Z',
      }),
    /blockKind/i,
  );
});

test('command telemetry survives later ordinary lane updates without changing reviewer state', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-command-state-'));
  process.env.NLSDD_PROJECT_ROOT = root;

  const {recordEnvelope} = freshRequire('NLSDD/scripts/nlsdd-envelope.cjs');

  recordEnvelope(root, {
    execution: 'plot-mode',
    lane: 'Lane 1',
    role: 'worker',
    eventType: 'pass',
    phaseAfter: 'quality-review-pending',
    currentItem: 'Record command lifecycle events',
    summary: 'lane passed review',
    timestamp: '2026-03-22T01:00:00.000Z',
  });
  recordEnvelope(root, {
    execution: 'plot-mode',
    lane: 'Lane 1',
    role: 'worker',
    eventType: 'command-blocked',
    command: 'npm run nlsdd:probe',
    cwd: '/tmp/codex-worker',
    status: 'blocked',
    blockKind: 'dependency',
    durationMs: 9000,
    pid: 4321,
    summary: 'command-blocked telemetry',
    timestamp: '2026-03-22T01:01:00.000Z',
  });
  recordEnvelope(root, {
    execution: 'plot-mode',
    lane: 'Lane 1',
    role: 'worker',
    eventType: 'state-update',
    phaseAfter: 'refill-ready',
    summary: 'ordinary lane state update',
    timestamp: '2026-03-22T01:02:00.000Z',
  });

  const laneState = JSON.parse(
    fs.readFileSync(path.join(root, 'NLSDD', 'state', 'plot-mode', 'lane-1.json'), 'utf8'),
  );
  assert.equal(laneState.lastReviewerResult, 'pass');
  assert.equal(laneState.command, 'npm run nlsdd:probe');
  assert.equal(laneState.cwd, '/tmp/codex-worker');
  assert.equal(laneState.status, 'blocked');
  assert.equal(laneState.blockKind, 'dependency');
  assert.equal(laneState.durationMs, 9000);
  assert.equal(laneState.pid, 4321);
});

test('command telemetry helper records canonical envelopes and is wired to npm scripts', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-command-cli-'));
  const originalProjectRoot = process.env.NLSDD_PROJECT_ROOT;
  process.env.NLSDD_PROJECT_ROOT = root;

  try {
    const packageJson = JSON.parse(fs.readFileSync(repoRoot('package.json'), 'utf8'));
    assert.equal(
      packageJson.scripts['nlsdd:command:record'],
      'node NLSDD/scripts/nlsdd-record-command-event.cjs',
    );

    const executionDir = path.join(root, 'NLSDD', 'executions', 'plot-mode');
    fs.mkdirSync(executionDir, {recursive: true});
    fs.writeFileSync(
      path.join(executionDir, 'lane-1.md'),
      `# Lane 1

> Ownership family:
> \`tests/nlsdd-automation.test.js\`

NLSDD worktree: \`.worktrees/lane-1-worker\`

Lane-local verification:
\`node --test tests/nlsdd-automation.test.js\`
`,
      'utf8',
    );
    fs.mkdirSync(path.join(root, 'NLSDD'), {recursive: true});
    fs.writeFileSync(
      path.join(root, 'NLSDD', 'scoreboard.md'),
      `# NLSDD Scoreboard

| Execution | Lane | Ownership | Current item | Phase | Item commit | Last verification | Blocked by | Next refill target | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| plot-mode | Lane 1 | Worker telemetry | Record command lifecycle events | queued | \`n/a\` | \`node --test tests/nlsdd-automation.test.js\` | none | n/a | telemetry lane |
`,
      'utf8',
    );

    const commandCli = repoRoot('NLSDD/scripts/nlsdd-record-command-event.cjs');
    run(
      'node',
      [
        commandCli,
        '--execution',
        'plot-mode',
        '--lane',
        '1',
        '--event-type',
        'command-probe',
        '--command',
        'npm run nlsdd:probe',
        '--cwd',
        '/tmp/codex-worker',
        '--status',
        'probe',
        '--duration-ms',
        '4500',
        '--probe-summary',
        'stdout silent; worker still alive',
        '--pid',
        '4321',
        '--timestamp',
        '2026-03-22T01:05:00.000Z',
      ],
      root,
    );
    run(
      'node',
      [
        commandCli,
        '--execution',
        'plot-mode',
        '--lane',
        '1',
        '--event-type',
        'command-blocked',
        '--command',
        'npm run nlsdd:probe',
        '--cwd',
        '/tmp/codex-worker',
        '--status',
        'blocked',
        '--block-kind',
        'dependency',
        '--duration-ms',
        '9000',
        '--pid',
        '4321',
        '--timestamp',
        '2026-03-22T01:06:00.000Z',
      ],
      root,
    );
    run(
      'node',
      [
        commandCli,
        '--execution',
        'plot-mode',
        '--lane',
        '1',
        '--event-type',
        'command-failed',
        '--command',
        'npm run nlsdd:test',
        '--cwd',
        '/tmp/codex-worker',
        '--status',
        'failed',
        '--exit-code',
        '2',
        '--duration-ms',
        '1200',
        '--pid',
        '4321',
        '--timestamp',
        '2026-03-22T01:07:00.000Z',
      ],
      root,
    );

    const eventsPath = path.join(root, 'NLSDD', 'state', 'plot-mode', 'events.ndjson');
    const events = fs
      .readFileSync(eventsPath, 'utf8')
      .trim()
      .split('\n')
      .map((line) => JSON.parse(line));
    const recordedProbe = events.find((event) => event.eventType === 'command-probe');
    const recordedBlocked = events.find((event) => event.eventType === 'command-blocked');
    const recordedFailed = events.find((event) => event.eventType === 'command-failed');
    assert.ok(recordedProbe, 'command-probe event was not recorded');
    assert.ok(recordedBlocked, 'command-blocked event was not recorded');
    assert.ok(recordedFailed, 'command-failed event was not recorded');
    assert.equal(recordedProbe.command, 'npm run nlsdd:probe');
    assert.equal(recordedProbe.cwd, '/tmp/codex-worker');
    assert.equal(recordedProbe.status, 'probe');
    assert.equal(recordedProbe.durationMs, 4500);
    assert.equal(recordedProbe.probeSummary, 'stdout silent; worker still alive');
    assert.equal(recordedProbe.pid, 4321);
    assert.equal(recordedBlocked.blockKind, 'dependency');
    assert.equal(recordedBlocked.status, 'blocked');
    assert.equal(recordedBlocked.durationMs, 9000);
    assert.equal(recordedBlocked.pid, 4321);
    assert.equal(recordedFailed.exitCode, 2);
    assert.equal(recordedFailed.status, 'failed');
    assert.equal(recordedFailed.durationMs, 1200);
    assert.equal(recordedFailed.pid, 4321);
  } finally {
    if (originalProjectRoot === undefined) {
      delete process.env.NLSDD_PROJECT_ROOT;
    } else {
      process.env.NLSDD_PROJECT_ROOT = originalProjectRoot;
    }
  }
});

test('telemetry summarizer projects minute buckets and drop diagnostics from execution events', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-telemetry-'));
  const execution = 'plot-mode';
  process.env.NLSDD_PROJECT_ROOT = root;

  writeTelemetryEvents(root, execution, [
    {
      execution,
      lane: 'Lane 1',
      role: 'coordinator',
      eventType: 'bootstrap-state',
      phaseAfter: 'implementing',
      summary: 'Lane 1 is actively implementing',
      timestamp: '2026-03-22T01:00:00.000Z',
    },
    {
      execution,
      lane: 'Lane 2',
      role: 'coordinator',
      eventType: 'bootstrap-state',
      phaseAfter: 'implementing',
      summary: 'Lane 2 is actively implementing',
      timestamp: '2026-03-22T01:00:00.000Z',
    },
    {
      execution,
      lane: 'Lane 3',
      role: 'coordinator',
      eventType: 'bootstrap-state',
      phaseAfter: 'implementing',
      summary: 'Lane 3 is actively implementing',
      timestamp: '2026-03-22T01:00:00.000Z',
    },
    {
      execution,
      lane: 'Lane 4',
      role: 'coordinator',
      eventType: 'bootstrap-state',
      phaseAfter: 'implementing',
      summary: 'Lane 4 is actively implementing',
      timestamp: '2026-03-22T01:00:00.000Z',
    },
    {
      execution,
      lane: 'Lane 5',
      role: 'coordinator',
      eventType: 'bootstrap-state',
      phaseAfter: 'implementing',
      summary: 'Lane 5 is actively implementing',
      timestamp: '2026-03-22T01:00:00.000Z',
    },
    {
      execution,
      lane: 'Lane 1',
      role: 'worker',
      eventType: 'ready-to-commit',
      phaseAfter: 'coordinator-commit-pending',
      summary: 'Lane 1 is waiting for the coordinator commit gate',
      timestamp: '2026-03-22T01:01:00.000Z',
    },
    {
      execution,
      lane: 'Lane 2',
      role: 'worker',
      eventType: 'command-started',
      command: 'curl google.com',
      cwd: '/tmp/codex-worker',
      status: 'started',
      summary: 'Lane 2 started a network probe',
      timestamp: '2026-03-22T01:02:00.000Z',
    },
    {
      execution,
      lane: 'Lane 2',
      role: 'worker',
      eventType: 'command-failed',
      command: 'curl google.com',
      cwd: '/tmp/codex-worker',
      status: 'failed',
      exitCode: 6,
      durationMs: 900,
      summary: 'curl failed fast on DNS resolution',
      timestamp: '2026-03-22T01:02:20.000Z',
    },
    {
      execution,
      lane: 'Lane 3',
      role: 'worker',
      eventType: 'command-probe',
      command: 'npm run nlsdd:probe',
      cwd: '/tmp/codex-worker',
      status: 'probe',
      durationMs: 4500,
      probeSummary: 'stdout silent; worker still alive',
      summary: 'Lane 3 is still alive but silent',
      timestamp: '2026-03-22T01:03:00.000Z',
    },
    {
      execution,
      lane: 'Lane 3',
      role: 'worker',
      eventType: 'command-blocked',
      command: 'npm run nlsdd:probe',
      cwd: '/tmp/codex-worker',
      status: 'blocked',
      blockKind: 'dependency',
      durationMs: 9000,
      summary: 'Lane 3 is blocked on a dependency with probe evidence',
      timestamp: '2026-03-22T01:03:10.000Z',
    },
    {
      execution,
      lane: 'Lane 4',
      role: 'coordinator',
      eventType: 'state-update',
      phaseAfter: 'blocked',
      blockedBy: 'dependency',
      summary: 'Lane 4 is blocked by a dependency',
      timestamp: '2026-03-22T01:04:00.000Z',
    },
    {
      execution,
      lane: 'Lane 5',
      role: 'coordinator',
      eventType: 'state-update',
      phaseAfter: 'queued',
      summary: 'Lane 5 fell silent without diagnostic evidence',
      timestamp: '2026-03-22T01:05:00.000Z',
    },
  ]);

  const {telemetrySummaryPath, summarizeTelemetry} = freshRequire(
    'NLSDD/scripts/nlsdd-summarize-telemetry.cjs',
  );
  const summary = summarizeTelemetry(root, execution);
  const summaryPath = telemetrySummaryPath(root, execution);
  const summaryOnDisk = JSON.parse(fs.readFileSync(summaryPath, 'utf8'));

  assert.equal(summaryPath, path.join(root, 'NLSDD', 'state', execution, 'telemetry-summary.json'));
  assert.deepEqual(summaryOnDisk, summary);
  assert.equal(summary.execution, execution);
  assert.equal(summary.firstActivityAt, '2026-03-22T01:00:00.000Z');
  assert.equal(summary.lastActivityAt, '2026-03-22T01:05:00.000Z');
  assert.equal(summary.wallClockDurationMs, 300000);
  assert.equal(summary.minuteBuckets.length, 6);
  assert.deepEqual(
    summary.minuteBuckets.map((bucket) => ({
      minute: bucket.minute,
      minuteStartAt: bucket.minuteStartAt,
      activeWorkers: bucket.activeWorkers,
      productiveWorkers: bucket.productiveWorkers,
    })),
    [
      {minute: 0, minuteStartAt: '2026-03-22T01:00:00.000Z', activeWorkers: 5, productiveWorkers: 5},
      {minute: 1, minuteStartAt: '2026-03-22T01:01:00.000Z', activeWorkers: 5, productiveWorkers: 4},
      {minute: 2, minuteStartAt: '2026-03-22T01:02:00.000Z', activeWorkers: 4, productiveWorkers: 3},
      {minute: 3, minuteStartAt: '2026-03-22T01:03:00.000Z', activeWorkers: 4, productiveWorkers: 2},
      {minute: 4, minuteStartAt: '2026-03-22T01:04:00.000Z', activeWorkers: 4, productiveWorkers: 1},
      {minute: 5, minuteStartAt: '2026-03-22T01:05:00.000Z', activeWorkers: 3, productiveWorkers: 0},
    ],
  );

  const segmentsByReason = new Map();
  for (const segment of summary.dropSegments) {
    if (!segmentsByReason.has(segment.reason)) {
      segmentsByReason.set(segment.reason, []);
    }
    segmentsByReason.get(segment.reason).push(segment);
  }
  assert.equal(segmentsByReason.size, 5);
  for (const reason of [
    'handoff-wait',
    'fast-fail',
    'command-blocked-with-probe-evidence',
    'dependency-blocked',
    'unknown-silence',
  ]) {
    assert.ok(segmentsByReason.has(reason), `missing drop segment reason: ${reason}`);
  }

  const handoffWait = segmentsByReason.get('handoff-wait').find(
    (segment) => segment.metric === 'productiveWorkers',
  );
  assert.equal(handoffWait.metric, 'productiveWorkers');
  assert.equal(handoffWait.fromMinute, '2026-03-22T01:00:00.000Z');
  assert.equal(handoffWait.toMinute, '2026-03-22T01:01:00.000Z');
  assert.ok(handoffWait.supportingEvents.some((event) => event.eventType === 'ready-to-commit'));
  assert.ok(Array.isArray(handoffWait.missingSignals));
  assert.equal(handoffWait.confidence, 'high');

  const fastFail = segmentsByReason.get('fast-fail').find(
    (segment) => segment.metric === 'activeWorkers',
  );
  assert.equal(fastFail.metric, 'activeWorkers');
  assert.equal(fastFail.fromMinute, '2026-03-22T01:01:00.000Z');
  assert.equal(fastFail.toMinute, '2026-03-22T01:02:00.000Z');
  assert.ok(fastFail.supportingEvents.some((event) => event.eventType === 'command-failed'));
  assert.ok(fastFail.supportingEvents.some((event) => event.command === 'curl google.com'));
  assert.equal(fastFail.confidence, 'high');

  const blockedWithProbe = segmentsByReason.get('command-blocked-with-probe-evidence').find(
    (segment) => segment.metric === 'productiveWorkers',
  );
  assert.equal(blockedWithProbe.metric, 'productiveWorkers');
  assert.equal(blockedWithProbe.fromMinute, '2026-03-22T01:02:00.000Z');
  assert.equal(blockedWithProbe.toMinute, '2026-03-22T01:03:00.000Z');
  assert.ok(blockedWithProbe.supportingEvents.some((event) => event.eventType === 'command-probe'));
  assert.ok(blockedWithProbe.supportingEvents.some((event) => event.eventType === 'command-blocked'));
  assert.equal(blockedWithProbe.confidence, 'high');

  const dependencyBlocked = segmentsByReason.get('dependency-blocked').find(
    (segment) => segment.metric === 'productiveWorkers',
  );
  assert.equal(dependencyBlocked.metric, 'productiveWorkers');
  assert.equal(dependencyBlocked.fromMinute, '2026-03-22T01:03:00.000Z');
  assert.equal(dependencyBlocked.toMinute, '2026-03-22T01:04:00.000Z');
  assert.ok(dependencyBlocked.supportingEvents.some((event) => event.blockedBy === 'dependency'));
  assert.equal(dependencyBlocked.confidence, 'high');

  const unknownSilence = segmentsByReason.get('unknown-silence').find(
    (segment) => segment.metric === 'activeWorkers',
  );
  assert.equal(unknownSilence.metric, 'activeWorkers');
  assert.equal(unknownSilence.fromMinute, '2026-03-22T01:04:00.000Z');
  assert.equal(unknownSilence.toMinute, '2026-03-22T01:05:00.000Z');
  assert.deepEqual(unknownSilence.missingSignals, [
    'command lifecycle events',
    'worker-local probe evidence',
  ]);
  assert.equal(unknownSilence.confidence, 'low');
});

test('telemetry review renderer writes coordinator-readable markdown output', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-telemetry-review-'));
  const execution = 'plot-mode';
  process.env.NLSDD_PROJECT_ROOT = root;

  const stateDir = path.join(root, 'NLSDD', 'state', execution);
  fs.mkdirSync(stateDir, {recursive: true});
  fs.writeFileSync(
    path.join(stateDir, 'telemetry-summary.json'),
    `${JSON.stringify(
      {
        execution,
        firstActivityAt: '2026-03-22T01:00:00.000Z',
        lastActivityAt: '2026-03-22T01:05:00.000Z',
        wallClockDurationMs: 300000,
        minuteBuckets: [
          {
            minute: 0,
            minuteStartAt: '2026-03-22T01:00:00.000Z',
            activeWorkers: 5,
            productiveWorkers: 5,
          },
          {
            minute: 1,
            minuteStartAt: '2026-03-22T01:01:00.000Z',
            activeWorkers: 5,
            productiveWorkers: 4,
          },
        ],
        dropSegments: [
          {
            fromMinute: '2026-03-22T01:00:00.000Z',
            toMinute: '2026-03-22T01:01:00.000Z',
            metric: 'productiveWorkers',
            reason: 'handoff-wait',
            confidence: 'high',
            missingSignals: ['coordinator commit acknowledgement'],
          },
        ],
      },
      null,
      2,
    )}\n`,
    'utf8',
  );

  const {renderTelemetryReviewFile} = freshRequire(
    'NLSDD/scripts/nlsdd-render-telemetry-review.cjs',
  );
  const {outputPath, content} = renderTelemetryReviewFile(root, execution);

  assert.equal(outputPath, path.join(stateDir, 'telemetry-review.md'));
  assert.equal(fs.existsSync(outputPath), true);
  assert.match(content, /# plot-mode Telemetry Review/);
  assert.match(content, /Wall clock duration: 300000 ms/);
  assert.match(content, /\| Minute \| Active workers \| Productive workers \|/);
  assert.match(content, /handoff-wait \[productiveWorkers\]/);
  assert.match(content, /Missing signals: coordinator commit acknowledgement/);
});

test('telemetry summarizer keeps implementing lanes productive after command-finished and avoids generic blocked->dependency diagnosis', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-telemetry-edge-'));
  const execution = 'plot-mode';
  process.env.NLSDD_PROJECT_ROOT = root;

  writeTelemetryEvents(root, execution, [
    {
      execution,
      lane: 'Lane 1',
      role: 'coordinator',
      eventType: 'bootstrap-state',
      phaseAfter: 'implementing',
      summary: 'Lane 1 implementing',
      timestamp: '2026-03-22T02:00:00.000Z',
    },
    {
      execution,
      lane: 'Lane 1',
      role: 'worker',
      eventType: 'command-finished',
      command: 'npm test',
      cwd: '/tmp/lane-1',
      status: 'finished',
      durationMs: 1200,
      summary: 'Lane 1 command finished',
      timestamp: '2026-03-22T02:00:10.000Z',
    },
    {
      execution,
      lane: 'Lane 2',
      role: 'coordinator',
      eventType: 'bootstrap-state',
      phaseAfter: 'implementing',
      summary: 'Lane 2 implementing',
      timestamp: '2026-03-22T02:00:00.000Z',
    },
    {
      execution,
      lane: 'Lane 2',
      role: 'coordinator',
      eventType: 'state-update',
      phaseAfter: 'blocked',
      blockedBy: 'permission-prompt',
      summary: 'Lane 2 blocked on permission prompt',
      timestamp: '2026-03-22T02:01:00.000Z',
    },
  ]);

  const {summarizeTelemetry} = freshRequire('NLSDD/scripts/nlsdd-summarize-telemetry.cjs');
  const summary = summarizeTelemetry(root, execution);

  assert.equal(summary.minuteBuckets[0].productiveWorkers, 2);
  assert.equal(summary.minuteBuckets[1].productiveWorkers, 1);

  const unknownSilence = summary.dropSegments.find(
    (segment) =>
      segment.reason === 'unknown-silence' && segment.metric === 'productiveWorkers',
  );
  assert.ok(unknownSilence, 'expected unknown-silence productive drop');
});

test('envelope reducer projects tracked scoreboard and lane status from a single handoff event', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-envelope-'));
  process.env.NLSDD_PROJECT_ROOT = root;

  fs.mkdirSync(path.join(root, 'NLSDD', 'executions', 'plot-mode'), {recursive: true});
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

## C - Controller / Docs and Verification Surfaces

- [ ] Highlight the adopted routing target more clearly once compare-panel data becomes meaningful

## Current Lane Status

- [x] stale manual line

## Refill Order

- [ ] Later item
`,
    'utf8',
  );
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'scoreboard.md'),
    `# NLSDD Scoreboard

| Execution | Lane | Ownership | Current item | Phase | Item commit | Last verification | Blocked by | Next refill target | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| plot-mode | Lane 4 | Rust panels + docs | Recommendation-rich Compare panel on the recovery baseline | queued | \`51ac2eb\` | \`cargo test render_panels_builds_visible_summary_and_compare_blocks --manifest-path rust/plot-viewer/Cargo.toml\` | none | Extend panel structure only if later plot UX still needs richer side-panel content | stale note |
`,
    'utf8',
  );

  const {recordEnvelope, loadEnvelopeEvents} = freshRequire('NLSDD/scripts/nlsdd-envelope.cjs');
  recordEnvelope(root, {
    execution: 'plot-mode',
    lane: 'Lane 4',
    role: 'implementer',
    eventType: 'ready-to-commit',
    phaseBefore: 'queued',
    phaseAfter: 'coordinator-commit-pending',
    currentItem: 'Highlight the adopted routing target more clearly once compare-panel data becomes meaningful',
    nextRefillTarget: 'Extend panel structure only if later plot UX still needs richer side-panel content',
    relatedCommit: '6bb1fba',
    verification: [
      'cargo test render_panels_builds_visible_summary_and_compare_blocks --manifest-path rust/plot-viewer/Cargo.toml',
    ],
    summary: 'Lane 4 adopted-target emphasis is ready to commit',
    detail: 'READY_TO_COMMIT package from panels lane',
    nextExpectedPhase: 'spec-review-pending',
    'commit-title': 'feat(plot): 強化 compare panel 的 adopted target 標示',
    'commit-body': '讓 Compare panel 更清楚標示目前採用的 routing target',
    insights: [
      {
        lane: 'Lane 4',
        source: 'subagent',
        kind: 'improvement-opportunity',
        status: 'adopted',
        summary: 'Adopted target deserves stronger emphasis once compare data is present',
      },
    ],
    timestamp: '2026-03-21T12:20:00.000Z',
  });

  const events = loadEnvelopeEvents(root, 'plot-mode');
  assert.equal(events.length, 2);
  assert.equal(events.some((entry) => entry.eventType === 'ready-to-commit'), true);

  const state = JSON.parse(
    fs.readFileSync(path.join(root, 'NLSDD', 'state', 'plot-mode', 'lane-4.json'), 'utf8'),
  );
  assert.equal(state.phase, 'coordinator-commit-pending');
  assert.equal(state.currentItem, 'Highlight the adopted routing target more clearly once compare-panel data becomes meaningful');
  assert.equal(state.latestCommit, '6bb1fba');
  assert.equal(state.proposedCommitTitle, 'feat(plot): 強化 compare panel 的 adopted target 標示');

  const scoreboard = fs.readFileSync(path.join(root, 'NLSDD', 'scoreboard.md'), 'utf8');
  assert.match(scoreboard, /Highlight the adopted routing target more clearly once compare-panel data becomes meaningful/);
  assert.match(scoreboard, /\| plot-mode \| Lane 4 \| Rust panels \+ docs \| Highlight the adopted routing target more clearly once compare-panel data becomes meaningful \| coordinator-commit-pending \| `6bb1fba` \|/);

  const lanePlan = fs.readFileSync(path.join(root, 'NLSDD', 'executions', 'plot-mode', 'lane-4.md'), 'utf8');
  assert.match(lanePlan, /## Current Lane Status/);
  assert.match(lanePlan, /Projected phase: coordinator-commit-pending/);
  assert.match(lanePlan, /Latest event: ready-to-commit · Lane 4 adopted-target emphasis is ready to commit/);

  const insightsLines = fs
    .readFileSync(path.join(root, 'NLSDD', 'state', 'plot-mode', 'execution-insights.ndjson'), 'utf8')
    .trim()
    .split('\n');
  assert.equal(insightsLines.length, 1);
  const insight = JSON.parse(insightsLines[0]);
  assert.equal(insight.kind, 'improvement-opportunity');
  assert.equal(insight.summary, 'Adopted target deserves stronger emphasis once compare data is present');
});

test('envelope replay uses the execution root when deriving fallback lane content', () => {
  const executionRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-envelope-root-'));
  const cwdRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-envelope-cwd-'));
  const originalCwd = process.cwd();
  const originalProjectRoot = process.env.NLSDD_PROJECT_ROOT;
  const modulePath = path.join(originalCwd, 'NLSDD', 'scripts', 'nlsdd-envelope.cjs');

  try {
    fs.mkdirSync(path.join(executionRoot, 'NLSDD', 'executions', 'plot-mode'), {recursive: true});
    fs.writeFileSync(
      path.join(executionRoot, 'NLSDD', 'executions', 'plot-mode', 'lane-1.md'),
      `# Lane 1

> Ownership family:
> \`src/commands/root.ts\`
>
> NLSDD worktree: \`.worktrees/lane-1-node\`
>
> Lane-local verification:
> \`git status --short\`

## C - Controller

- [ ] Root A repair item
`,
      'utf8',
    );
    fs.writeFileSync(
      path.join(executionRoot, 'NLSDD', 'scoreboard.md'),
      `# NLSDD Scoreboard

| Execution | Lane | Ownership | Current item | Phase | Item commit | Last verification | Blocked by | Next refill target | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| plot-mode | Lane 1 | Node contract + handoff |  | queued | \`n/a\` | \`git status --short\` | none |  | root A row |
`,
      'utf8',
    );
    fs.mkdirSync(path.join(executionRoot, 'NLSDD', 'state', 'plot-mode'), {recursive: true});
    fs.writeFileSync(
      path.join(executionRoot, 'NLSDD', 'state', 'plot-mode', 'events.ndjson'),
      `${JSON.stringify({
        execution: 'plot-mode',
        lane: 'Lane 1',
        role: 'coordinator',
        eventType: 'bootstrap-state',
        phaseAfter: 'queued',
        summary: 'bootstrap',
        timestamp: '2026-03-21T08:00:00.000Z',
        insights: [],
      })}\n`,
      'utf8',
    );

    fs.mkdirSync(path.join(cwdRoot, 'NLSDD', 'executions', 'plot-mode'), {recursive: true});
    fs.writeFileSync(
      path.join(cwdRoot, 'NLSDD', 'executions', 'plot-mode', 'lane-1.md'),
      `# Lane 1

> Ownership family:
> \`src/commands/root.ts\`
>
> NLSDD worktree: \`.worktrees/lane-1-node\`
>
> Lane-local verification:
> \`git status --short\`

## C - Controller

- [ ] Root B repair item
`,
      'utf8',
    );

    delete process.env.NLSDD_PROJECT_ROOT;
    process.chdir(cwdRoot);

    delete require.cache[require.resolve(modulePath)];
    const {prepareExecutionState} = require(modulePath);
    prepareExecutionState(executionRoot, 'plot-mode');

    const laneState = JSON.parse(
      fs.readFileSync(
        path.join(executionRoot, 'NLSDD', 'state', 'plot-mode', 'lane-1.json'),
        'utf8',
      ),
    );
    const scoreboardText = fs.readFileSync(
      path.join(executionRoot, 'NLSDD', 'scoreboard.md'),
      'utf8',
    );

    assert.equal(laneState.currentItem, 'Root A repair item');
    assert.equal(laneState.nextRefillTarget, null);
    assert.match(scoreboardText, /Root A repair item/);
    assert.doesNotMatch(scoreboardText, /Root B repair item/);
  } finally {
    process.chdir(originalCwd);
    if (originalProjectRoot === undefined) {
      delete process.env.NLSDD_PROJECT_ROOT;
    } else {
      process.env.NLSDD_PROJECT_ROOT = originalProjectRoot;
    }
  }
});

test('parked replay clears stale projected fields from scoreboard projections', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-envelope-clear-'));
  const originalCwd = process.cwd();
  const originalProjectRoot = process.env.NLSDD_PROJECT_ROOT;
  const modulePath = path.join(originalCwd, 'NLSDD', 'scripts', 'nlsdd-envelope.cjs');

  try {
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

- [ ] Fresh parking item
`,
      'utf8',
    );
    fs.writeFileSync(
      path.join(root, 'NLSDD', 'scoreboard.md'),
      `# NLSDD Scoreboard

| Execution | Lane | Ownership | Current item | Phase | Item commit | Last verification | Blocked by | Next refill target | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| plot-mode | Lane 1 | Node contract + handoff | Stale projected item | implementing | \`abc1234\` | \`git status --short\` | none | Stale projected target | stale row |
`,
      'utf8',
    );
    fs.mkdirSync(path.join(root, 'NLSDD', 'state', 'plot-mode'), {recursive: true});
    fs.writeFileSync(
      path.join(root, 'NLSDD', 'state', 'plot-mode', 'events.ndjson'),
      [
        JSON.stringify({
          execution: 'plot-mode',
          lane: 'Lane 1',
          role: 'coordinator',
          eventType: 'bootstrap-state',
          phaseAfter: 'implementing',
          currentItem: 'Fresh parking item',
          nextRefillTarget: 'Fresh parking target',
          summary: 'bootstrap',
          timestamp: '2026-03-21T09:00:00.000Z',
          insights: [],
        }),
        JSON.stringify({
          execution: 'plot-mode',
          lane: 'Lane 1',
          role: 'coordinator',
          eventType: 'parked',
          phaseBefore: 'implementing',
          phaseAfter: 'parked',
          summary: 'Lane 1 is parked after the current work closed cleanly',
          detail: 'No honest refill item is ready yet.',
          timestamp: '2026-03-21T09:05:00.000Z',
          insights: [],
        }),
      ].join('\n') + '\n',
      'utf8',
    );

    delete process.env.NLSDD_PROJECT_ROOT;
    process.chdir(root);

    delete require.cache[require.resolve(modulePath)];
    const {prepareExecutionState} = require(modulePath);
    prepareExecutionState(root, 'plot-mode');

    const laneState = JSON.parse(
      fs.readFileSync(path.join(root, 'NLSDD', 'state', 'plot-mode', 'lane-1.json'), 'utf8'),
    );
    const scoreboardText = fs.readFileSync(path.join(root, 'NLSDD', 'scoreboard.md'), 'utf8');

    assert.equal(laneState.phase, 'parked');
    assert.equal(laneState.currentItem, null);
    assert.equal(laneState.nextRefillTarget, null);
    assert.equal(laneState.expectedNextPhase, null);
    assert.match(
      scoreboardText,
      /\| plot-mode \| Lane 1 \| Node contract \+ handoff \| n\/a \| parked \| `abc1234` \| n\/a \| none \| n\/a \| No honest refill item is ready yet\. \|/,
    );
    assert.doesNotMatch(scoreboardText, /Stale projected item/);
    assert.doesNotMatch(scoreboardText, /Stale projected target/);
  } finally {
    process.chdir(originalCwd);
    if (originalProjectRoot === undefined) {
      delete process.env.NLSDD_PROJECT_ROOT;
    } else {
      process.env.NLSDD_PROJECT_ROOT = originalProjectRoot;
    }
  }
});
test('review helper reduces execution state before reading projected review actions', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-review-reduce-'));
  process.env.NLSDD_PROJECT_ROOT = root;

  fs.mkdirSync(path.join(root, 'NLSDD', 'executions', 'plot-mode'), {recursive: true});
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'executions', 'plot-mode', 'lane-3.md'),
    `# Lane 3

> Ownership family:
> \`rust/plot-viewer/src/render/chart.rs\`
>
> NLSDD worktree: \`.worktrees/lane-3-chart\`
>
> Lane-local verification:
> \`cargo check --manifest-path rust/plot-viewer/Cargo.toml\`

## C - Controller

- [ ] Chart compatibility follow-up
`,
    'utf8',
  );
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'scoreboard.md'),
    `# NLSDD Scoreboard

| Execution | Lane | Ownership | Current item | Phase | Item commit | Last verification | Blocked by | Next refill target | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| plot-mode | Lane 3 | Rust chart surface | Chart compatibility follow-up | queued | \`abc1234\` | \`cargo check --manifest-path rust/plot-viewer/Cargo.toml\` | none | none | stale row |
`,
    'utf8',
  );

  const {recordEnvelope} = freshRequire('NLSDD/scripts/nlsdd-envelope.cjs');
  recordEnvelope(root, {
    execution: 'plot-mode',
    lane: 'Lane 3',
    role: 'implementer',
    eventType: 'pass',
    phaseBefore: 'implementing',
    phaseAfter: 'spec-review-pending',
    currentItem: 'Chart compatibility follow-up',
    relatedCommit: 'abc1234',
    verification: ['cargo check --manifest-path rust/plot-viewer/Cargo.toml'],
    summary: 'Lane 3 chart compatibility follow-up is ready for spec review',
    detail: 'READY_FOR_REVIEW from chart lane',
    nextExpectedPhase: 'quality-review-pending',
    timestamp: '2026-03-21T12:40:00.000Z',
  });

  const {driveReviewLoop} = freshRequire('NLSDD/scripts/nlsdd-drive-review-loop.cjs');
  const result = driveReviewLoop(root, 'plot-mode');

  assert.equal(result.actions.length, 1);
  assert.equal(result.actions[0].lane, 'Lane 3');
  assert.equal(result.actions[0].action, 'spec-review');
  assert.match(result.actions[0].message, /Review target commit: abc1234/);
});

test('commit intake helper reduces execution state before reading ready-to-commit handoff', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-intake-reduce-'));
  process.env.NLSDD_PROJECT_ROOT = root;

  fs.mkdirSync(path.join(root, 'NLSDD', 'executions', 'plot-mode'), {recursive: true});
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'executions', 'plot-mode', 'lane-5.md'),
    `# Lane 5

> Ownership family:
> \`README.md\`
>
> NLSDD worktree: \`.worktrees/lane-5-docs\`
>
> Lane-local verification:
> \`npm run build\`

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
| plot-mode | Lane 5 | Plot viewer docs + operator flow | Recovery-baseline README and local run instructions | queued | \`n/a\` | \`npm run build\` | none | none | stale row |
`,
    'utf8',
  );

  const {recordEnvelope} = freshRequire('NLSDD/scripts/nlsdd-envelope.cjs');
  recordEnvelope(root, {
    execution: 'plot-mode',
    lane: 'Lane 5',
    role: 'implementer',
    eventType: 'ready-to-commit',
    phaseBefore: 'implementing',
    phaseAfter: 'coordinator-commit-pending',
    currentItem: 'Recovery-baseline README and local run instructions',
    relatedCommit: null,
    verification: ['npm run build'],
    summary: 'Lane 5 docs refresh is ready to commit',
    detail: 'READY_TO_COMMIT from docs lane',
    nextExpectedPhase: 'spec-review-pending',
    'commit-title': 'docs(plot): 補上 recovery baseline 操作說明',
    'commit-body': '同步本地驗證與啟動步驟',
    timestamp: '2026-03-21T12:41:00.000Z',
  });

  const {intakeReadyToCommit} = freshRequire('NLSDD/scripts/nlsdd-intake-ready-to-commit.cjs');
  const result = intakeReadyToCommit(root, 'plot-mode');

  assert.equal(result.length, 1);
  assert.equal(result[0].lane, 'Lane 5');
  assert.equal(result[0].phase, 'coordinator-commit-pending');
  assert.equal(result[0].proposedCommitTitle, 'docs(plot): 補上 recovery baseline 操作說明');
  assert.equal(result[0].note, 'READY_TO_COMMIT from docs lane');
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

test('execution insight summary helper groups actionable insights and converged schema kinds', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-insight-summary-'));
  process.env.NLSDD_PROJECT_ROOT = root;

  const {recordInsight} = freshRequire('NLSDD/scripts/nlsdd-record-insight.cjs');
  const {summarizeExecutionInsights} = freshRequire('NLSDD/scripts/nlsdd-lib.cjs');
  const {renderInsightSummary} = freshRequire('NLSDD/scripts/nlsdd-summarize-insights.cjs');

  recordInsight(root, {
    execution: 'plot-mode',
    lane: '2',
    source: 'subagent',
    kind: 'blocker',
    status: 'open',
    summary: 'Need render boundary compare payload',
    timestamp: '2026-03-21T08:00:00.000Z',
  });
  recordInsight(root, {
    execution: 'plot-mode',
    lane: '3',
    source: 'coordinator',
    kind: 'noop-finding',
    status: 'adopted',
    summary: 'Lane 3 follow-up is a real no-op',
    timestamp: '2026-03-21T08:01:00.000Z',
  });
  recordInsight(root, {
    execution: 'plot-mode',
    lane: 'global',
    source: 'coordinator',
    kind: 'resolved-blocker',
    status: 'resolved',
    summary: 'Scheduler truth drift fixed',
    timestamp: '2026-03-21T08:02:00.000Z',
  });

  const summary = summarizeExecutionInsights(root, 'plot-mode');
  assert.equal(summary.total, 3);
  assert.equal(summary.actionableCount, 2);
  assert.equal(summary.durableLearningCount, 0);
  assert.equal(summary.resolvedHistoryCount, 1);
  assert.equal(summary.countsByKind.blocker, 1);
  assert.equal(summary.countsByKind['noop-finding'], 1);
  assert.equal(summary.countsByKind['resolved-blocker'], 1);
  assert.deepEqual(
    summary.actionable.map((entry) => [entry.lane, entry.kind, entry.status]),
    [
      ['Lane 3', 'noop-finding', 'adopted'],
      ['Lane 2', 'blocker', 'open'],
    ],
  );

  const rendered = renderInsightSummary(summary);
  assert.match(rendered, /Actionable insights: 2/);
  assert.match(rendered, /Durable global learnings: 0/);
  assert.match(rendered, /Resolved history: 1/);
  assert.match(rendered, /\[adopted\] Lane 3 · noop-finding · Lane 3 follow-up is a real no-op/);
  assert.match(rendered, /\[open\] Lane 2 · blocker · Need render boundary compare payload/);
});

test('execution insight summary separates durable global learnings from actionable lane work', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-insight-durable-'));
  process.env.NLSDD_PROJECT_ROOT = root;

  const {recordInsight} = freshRequire('NLSDD/scripts/nlsdd-record-insight.cjs');
  const {summarizeExecutionInsights} = freshRequire('NLSDD/scripts/nlsdd-lib.cjs');
  const {renderInsightSummary} = freshRequire('NLSDD/scripts/nlsdd-summarize-insights.cjs');

  recordInsight(root, {
    execution: 'plot-mode',
    lane: 'global',
    source: 'coordinator',
    kind: 'improvement-opportunity',
    status: 'adopted',
    summary: 'Keep active slots filled with real work, not stale implementing state',
    timestamp: '2026-03-21T08:00:00.000Z',
  });
  recordInsight(root, {
    execution: 'plot-mode',
    lane: '2',
    source: 'coordinator',
    kind: 'blocker',
    status: 'open',
    summary: 'Lane 2 still needs compare payload seam from Lane 4',
    timestamp: '2026-03-21T08:01:00.000Z',
  });

  const summary = summarizeExecutionInsights(root, 'plot-mode');
  assert.equal(summary.total, 2);
  assert.equal(summary.actionableCount, 1);
  assert.equal(summary.durableLearningCount, 1);
  assert.equal(summary.resolvedHistoryCount, 0);
  assert.equal(summary.actionable[0].lane, 'Lane 2');
  assert.equal(summary.durableLearnings[0].lane, 'global');

  const rendered = renderInsightSummary(summary);
  assert.match(rendered, /Actionable insights: 1/);
  assert.match(rendered, /Durable global learnings: 1/);
  assert.match(rendered, /\[adopted\] global · improvement-opportunity · Keep active slots filled with real work, not stale implementing state/);
  assert.match(rendered, /\[open\] Lane 2 · blocker · Lane 2 still needs compare payload seam from Lane 4/);
});

test('execution insight summary keeps only the latest status for the same lane+summary key', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-insight-collapse-'));
  process.env.NLSDD_PROJECT_ROOT = root;

  const {recordInsight} = freshRequire('NLSDD/scripts/nlsdd-record-insight.cjs');
  const {summarizeExecutionInsights} = freshRequire('NLSDD/scripts/nlsdd-lib.cjs');

  recordInsight(root, {
    execution: 'plot-mode',
    lane: '4',
    source: 'subagent',
    kind: 'suggestion',
    status: 'open',
    summary: 'Need render-boundary compare payload',
    timestamp: '2026-03-21T08:00:00.000Z',
  });
  recordInsight(root, {
    execution: 'plot-mode',
    lane: '4',
    source: 'coordinator',
    kind: 'resolved-blocker',
    status: 'resolved',
    summary: 'Need render-boundary compare payload',
    timestamp: '2026-03-21T08:05:00.000Z',
  });

  const summary = summarizeExecutionInsights(root, 'plot-mode');
  assert.equal(summary.total, 1);
  assert.equal(summary.actionableCount, 0);
  assert.equal(summary.durableLearningCount, 0);
  assert.equal(summary.resolvedHistoryCount, 1);
  assert.equal(summary.latest[0].kind, 'resolved-blocker');
  assert.equal(summary.latest[0].status, 'resolved');
  assert.equal(summary.latest[0].summary, 'Need render-boundary compare payload');
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

test('dispatch cycle promotes the next queued work from reduced execution state', () => {
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

  const {recordEnvelope} = freshRequire('NLSDD/scripts/nlsdd-envelope.cjs');
  recordEnvelope(root, {
    execution: 'plot-mode',
    lane: 'Lane 1',
    role: 'coordinator',
    eventType: 'parked',
    phaseBefore: 'implementing',
    phaseAfter: 'parked',
    currentItem: 'Future shell audit',
    relatedCommit: lane1Head,
    verification: ['git status --short'],
    summary: 'Lane 1 is parked after the previous node work closed cleanly',
    detail: 'No honest refill item is ready for Lane 1 yet.',
    nextExpectedPhase: null,
    timestamp: '2026-03-21T10:05:00.000Z',
  });

  const {runCycle} = freshRequire('NLSDD/scripts/nlsdd-run-cycle.cjs');
  const result = runCycle(root, 'plot-mode', 2, false);

  assert.deepEqual(result.reconciled.map((entry) => entry.lane), []);
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

test('launch helper returns assignment bundles for newly promoted lanes from reduced execution state', () => {
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

  const {recordEnvelope} = freshRequire('NLSDD/scripts/nlsdd-envelope.cjs');
  recordEnvelope(root, {
    execution: 'plot-mode',
    lane: 'Lane 1',
    role: 'coordinator',
    eventType: 'parked',
    phaseBefore: 'implementing',
    phaseAfter: 'parked',
    currentItem: 'Future shell audit',
    relatedCommit: lane1Head,
    verification: ['git status --short'],
    summary: 'Lane 1 is parked after the previous node work closed cleanly',
    detail: 'No honest refill item is ready for Lane 1 yet.',
    nextExpectedPhase: null,
    timestamp: '2026-03-21T10:05:00.000Z',
  });

  const {launchActiveSet} = freshRequire('NLSDD/scripts/nlsdd-launch-active-set.cjs');
  const result = launchActiveSet(root, 'plot-mode', 2, false);

  assert.deepEqual(result.completedLanes, []);
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

  assert.equal(result.actions.length, 2);
  assert.equal(result.insightSummary.actionableCount, 0);
  assert.equal(result.actions[0].lane, 'Lane 2');
  assert.equal(result.actions[0].action, 'spec-review');
  assert.match(result.actions[0].message, /Review target commit: abc1234/);

  assert.equal(result.actions[1].lane, 'Lane 4');
  assert.equal(result.actions[1].action, 'correction-loop');
  assert.match(result.actions[1].message, /Reviewer finding: Compare panel still re-derives label heuristics/);
  assert.match(result.actions[1].message, /Accepted write scope: rust\/plot-viewer\/src\/render\/panels\.rs/);
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

test('coordinator loop combines launch, review, and commit intake into one summary', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-loop-'));
  process.env.NLSDD_PROJECT_ROOT = root;

  const lane2Worktree = path.join(root, '.worktrees', 'lane-2-runtime');
  setupTempGitRepo(lane2Worktree);
  const lane2Head = run('git', ['rev-parse', '--short', 'HEAD'], lane2Worktree);

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

## V - View

- [ ] Runtime compare seam follow-up
`,
    'utf8',
  );
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'executions', 'plot-mode', 'lane-3.md'),
    `# Lane 3

> Ownership family:
> \`rust/plot-viewer/src/render/chart.rs\`
>
> NLSDD worktree: \`.worktrees/lane-3-chart\`
>
> Lane-local verification:
> \`cargo check --manifest-path rust/plot-viewer/Cargo.toml\`

## C - Controller

- [ ] Chart compatibility follow-up
`,
    'utf8',
  );
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'executions', 'plot-mode', 'lane-5.md'),
    `# Lane 5

> Ownership family:
> \`README.md\`
>
> NLSDD worktree: \`.worktrees/lane-5-docs\`
>
> Lane-local verification:
> \`npm run build\`

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
| plot-mode | Lane 2 | Rust runtime + boundary | Runtime compare seam follow-up | queued | \`${lane2Head}\` | \`cargo test --manifest-path rust/plot-viewer/Cargo.toml\` | none | none | queued row |
| plot-mode | Lane 3 | Rust chart surface | Chart compatibility follow-up | spec-review-pending | \`abc1234\` | \`cargo check --manifest-path rust/plot-viewer/Cargo.toml\` | none | none | review row |
| plot-mode | Lane 5 | Plot viewer docs + operator flow | Recovery-baseline README and local run instructions | coordinator-commit-pending | \`n/a\` | \`npm run build\` | none | none | commit row |
`,
    'utf8',
  );

  writeLaneState(root, 'plot-mode', 2, {
    execution: 'plot-mode',
    lane: 'Lane 2',
    phase: 'queued',
    'expected-next-phase': 'implementing',
    commit: lane2Head,
    verification: ['cargo test --manifest-path rust/plot-viewer/Cargo.toml'],
    note: 'ready to dispatch',
    'updated-at': '2026-03-21T11:30:00.000Z',
  });
  writeLaneState(root, 'plot-mode', 3, {
    execution: 'plot-mode',
    lane: 'Lane 3',
    phase: 'spec-review-pending',
    'expected-next-phase': 'quality-review-pending',
    commit: 'abc1234',
    verification: ['cargo check --manifest-path rust/plot-viewer/Cargo.toml'],
    note: 'spec review next',
    'updated-at': '2026-03-21T11:31:00.000Z',
  });
  writeLaneState(root, 'plot-mode', 5, {
    execution: 'plot-mode',
    lane: 'Lane 5',
    phase: 'coordinator-commit-pending',
    'expected-next-phase': 'spec-review-pending',
    'commit-title': 'docs(plot): 補上 recovery baseline 操作說明',
    'commit-body': '同步本地驗證與啟動步驟',
    verification: ['npm run build'],
    note: 'READY_TO_COMMIT from docs lane',
    'updated-at': '2026-03-21T11:32:00.000Z',
  });

  const {recordInsight} = freshRequire('NLSDD/scripts/nlsdd-record-insight.cjs');
  recordInsight(root, {
    execution: 'plot-mode',
    lane: '2',
    source: 'coordinator',
    kind: 'blocker',
    status: 'open',
    summary: 'Lane 4 still needs compare payload seam from Lane 2',
    timestamp: '2026-03-21T11:33:00.000Z',
  });

  const {runCoordinatorLoop} = freshRequire('NLSDD/scripts/nlsdd-run-coordinator-loop.cjs');
  const result = runCoordinatorLoop(root, 'plot-mode', 4, false);

  assert.deepEqual(result.promotedLanes, ['Lane 2']);
  assert.equal(result.launch.assignments.length, 1);
  assert.equal(result.reviewLaneCount, 2);
  assert.equal(result.commitLaneCount, 1);
  assert.equal(result.insightSummary.actionableCount, 1);
  assert.equal(result.insightSummary.actionable[0].summary, 'Lane 4 still needs compare payload seam from Lane 2');
  assert.equal(result.reviewActions.some((entry) => entry.lane === 'Lane 3' && entry.action === 'spec-review'), true);
  assert.equal(result.reviewActions.some((entry) => entry.lane === 'Lane 5' && entry.action === 'coordinator-commit-needed'), true);
  assert.equal(result.commitIntake[0].lane, 'Lane 5');
  assert.equal(result.commitIntake[0].proposedCommitTitle, 'docs(plot): 補上 recovery baseline 操作說明');
});

test('coordinator loop degrades cleanly when runtime scoreboard is malformed', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-loop-malformed-runtime-'));
  process.env.NLSDD_PROJECT_ROOT = root;

  const lane2Worktree = path.join(root, '.worktrees', 'lane-2-runtime');
  setupTempGitRepo(lane2Worktree);
  const lane2Head = run('git', ['rev-parse', '--short', 'HEAD'], lane2Worktree);

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

## V - View

- [ ] Runtime compare seam follow-up
`,
    'utf8',
  );
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'executions', 'plot-mode', 'lane-5.md'),
    `# Lane 5

> Ownership family:
> \`README.md\`
>
> NLSDD worktree: \`.worktrees/lane-5-docs\`
>
> Lane-local verification:
> \`npm run build\`

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
| plot-mode | Lane 2 | Rust runtime + boundary | Runtime compare seam follow-up | queued | \`${lane2Head}\` | \`cargo test --manifest-path rust/plot-viewer/Cargo.toml\` | none | none | queued row |
| plot-mode | Lane 5 | Plot viewer docs + operator flow | Recovery-baseline README and local run instructions | coordinator-commit-pending | \`n/a\` | \`npm run build\` | none | none | commit row |
`,
    'utf8',
  );

  writeLaneState(root, 'plot-mode', 2, {
    execution: 'plot-mode',
    lane: 'Lane 2',
    phase: 'queued',
    'expected-next-phase': 'implementing',
    commit: lane2Head,
    verification: ['cargo test --manifest-path rust/plot-viewer/Cargo.toml'],
    note: 'ready to dispatch',
    'updated-at': '2026-03-21T11:30:00.000Z',
  });
  writeLaneState(root, 'plot-mode', 5, {
    execution: 'plot-mode',
    lane: 'Lane 5',
    phase: 'coordinator-commit-pending',
    'expected-next-phase': 'spec-review-pending',
    'commit-title': 'docs(plot): 補上 recovery baseline 操作說明',
    'commit-body': '同步本地驗證與啟動步驟',
    verification: ['npm run build'],
    note: 'READY_TO_COMMIT from docs lane',
    'updated-at': '2026-03-21T11:32:00.000Z',
  });

  const runtimeScoreboardPath = path.join(root, 'NLSDD', 'state', 'scoreboard.runtime.md');
  fs.mkdirSync(path.dirname(runtimeScoreboardPath), {recursive: true});
  fs.writeFileSync(runtimeScoreboardPath, '# malformed runtime scoreboard\n', 'utf8');

  const envelopePath = repoRoot('NLSDD/scripts/nlsdd-envelope.cjs');
  delete require.cache[require.resolve(envelopePath)];
  const envelopeModule = require(envelopePath);
  const originalPrepareExecutionState = envelopeModule.prepareExecutionState;
  envelopeModule.prepareExecutionState = () => ({execution: 'plot-mode', laneCount: 0, insightCount: 0});
  try {
    delete require.cache[require.resolve(repoRoot('NLSDD/scripts/nlsdd-intake-ready-to-commit.cjs'))];
    delete require.cache[require.resolve(repoRoot('NLSDD/scripts/nlsdd-run-coordinator-loop.cjs'))];
    const {intakeReadyToCommit} = require(repoRoot('NLSDD/scripts/nlsdd-intake-ready-to-commit.cjs'));
    const {runCoordinatorLoop} = require(repoRoot('NLSDD/scripts/nlsdd-run-coordinator-loop.cjs'));

    assert.doesNotThrow(() => intakeReadyToCommit(root, 'plot-mode'));
    const commitIntake = intakeReadyToCommit(root, 'plot-mode');
    assert.equal(commitIntake.length, 1);
    assert.equal(commitIntake[0].lane, 'Lane 5');
    assert.equal(commitIntake[0].proposedCommitTitle, 'docs(plot): 補上 recovery baseline 操作說明');

    const result = runCoordinatorLoop(root, 'plot-mode', 4, false);

    assert.deepEqual(result.promotedLanes, ['Lane 2']);
    assert.equal(result.commitLaneCount, 1);
    assert.equal(result.reviewLaneCount, 1);
    assert.equal(
      result.reviewActions.some(
        (entry) => entry.lane === 'Lane 5' && entry.action === 'coordinator-commit-needed',
      ),
      true,
    );
    assert.equal(result.noDispatchReason, null);
  } finally {
    envelopeModule.prepareExecutionState = originalPrepareExecutionState;
  }
});

test('coordinator loop surfaces telemetry summary and review path when present', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-loop-telemetry-'));
  process.env.NLSDD_PROJECT_ROOT = root;

  const lane2Worktree = path.join(root, '.worktrees', 'lane-2-runtime');
  setupTempGitRepo(lane2Worktree);
  const lane2Head = run('git', ['rev-parse', '--short', 'HEAD'], lane2Worktree);

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
| plot-mode | Lane 2 | Rust runtime + boundary | Runtime compare seam follow-up | queued | \`${lane2Head}\` | \`cargo test --manifest-path rust/plot-viewer/Cargo.toml\` | none | none | queued row |
`,
    'utf8',
  );
  writeLaneState(root, 'plot-mode', 2, {
    execution: 'plot-mode',
    lane: 'Lane 2',
    phase: 'queued',
    'expected-next-phase': 'implementing',
    commit: lane2Head,
    verification: ['cargo test --manifest-path rust/plot-viewer/Cargo.toml'],
    note: 'ready to dispatch',
    'updated-at': '2026-03-21T11:30:00.000Z',
  });

  const stateDir = path.join(root, 'NLSDD', 'state', 'plot-mode');
  fs.mkdirSync(stateDir, {recursive: true});
  fs.writeFileSync(
    path.join(stateDir, 'telemetry-summary.json'),
    `${JSON.stringify(
      {
        execution: 'plot-mode',
        firstActivityAt: '2026-03-22T01:00:00.000Z',
        lastActivityAt: '2026-03-22T01:05:00.000Z',
        wallClockDurationMs: 300000,
        minuteBuckets: [
          {
            minute: 0,
            minuteStartAt: '2026-03-22T01:00:00.000Z',
            activeWorkers: 1,
            productiveWorkers: 1,
          },
        ],
        dropSegments: [
          {
            fromMinute: '2026-03-22T01:00:00.000Z',
            toMinute: '2026-03-22T01:01:00.000Z',
            metric: 'productiveWorkers',
            reason: 'handoff-wait',
            confidence: 'high',
            missingSignals: [],
          },
        ],
      },
      null,
      2,
    )}\n`,
    'utf8',
  );
  fs.writeFileSync(
    path.join(stateDir, 'telemetry-review.md'),
    '# plot-mode Telemetry Review\n',
    'utf8',
  );

  const {runCoordinatorLoop, renderCoordinatorLoop} = freshRequire(
    'NLSDD/scripts/nlsdd-run-coordinator-loop.cjs',
  );
  const packageJson = JSON.parse(fs.readFileSync(repoRoot('package.json'), 'utf8'));
  const result = runCoordinatorLoop(root, 'plot-mode', 4, false);
  const rendered = renderCoordinatorLoop(result);

  assert.equal(
    packageJson.scripts['nlsdd:telemetry:summarize'],
    'node NLSDD/scripts/nlsdd-summarize-telemetry.cjs',
  );
  assert.equal(
    packageJson.scripts['nlsdd:telemetry:review'],
    'node NLSDD/scripts/nlsdd-render-telemetry-review.cjs',
  );
  assert.ok(result.telemetrySummary.minuteBuckets.length >= 1);
  assert.equal(
    result.telemetryReviewPath,
    path.join(root, 'NLSDD', 'state', 'plot-mode', 'telemetry-review.md'),
  );
  assert.match(rendered, /Telemetry summary: \d+ minute bucket\(s\), \d+ drop segment\(s\)/);
  assert.match(rendered, /Telemetry review:/);
});

test('dispatch plan helper builds a prioritized action queue from autopilot output', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-dispatch-'));
  process.env.NLSDD_PROJECT_ROOT = root;

  const lane2Worktree = path.join(root, '.worktrees', 'lane-2-runtime');
  setupTempGitRepo(lane2Worktree);
  const lane2Head = run('git', ['rev-parse', '--short', 'HEAD'], lane2Worktree);

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

## V - View

- [ ] Runtime compare seam follow-up
`,
    'utf8',
  );
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'executions', 'plot-mode', 'lane-3.md'),
    `# Lane 3

> Ownership family:
> \`rust/plot-viewer/src/render/chart.rs\`
>
> NLSDD worktree: \`.worktrees/lane-3-chart\`
>
> Lane-local verification:
> \`cargo check --manifest-path rust/plot-viewer/Cargo.toml\`

## C - Controller

- [ ] Chart compatibility follow-up
`,
    'utf8',
  );
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'executions', 'plot-mode', 'lane-5.md'),
    `# Lane 5

> Ownership family:
> \`README.md\`
>
> NLSDD worktree: \`.worktrees/lane-5-docs\`
>
> Lane-local verification:
> \`npm run build\`

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
| plot-mode | Lane 2 | Rust runtime + boundary | Runtime compare seam follow-up | queued | \`${lane2Head}\` | \`cargo test --manifest-path rust/plot-viewer/Cargo.toml\` | none | none | queued row |
| plot-mode | Lane 3 | Rust chart surface | Chart compatibility follow-up | correction | \`abc1234\` | \`cargo check --manifest-path rust/plot-viewer/Cargo.toml\` | none | none | review row |
| plot-mode | Lane 5 | Plot viewer docs + operator flow | Recovery-baseline README and local run instructions | coordinator-commit-pending | \`n/a\` | \`npm run build\` | none | none | commit row |
`,
    'utf8',
  );

  writeLaneState(root, 'plot-mode', 2, {
    execution: 'plot-mode',
    lane: 'Lane 2',
    phase: 'queued',
    'expected-next-phase': 'implementing',
    commit: lane2Head,
    verification: ['cargo test --manifest-path rust/plot-viewer/Cargo.toml'],
    note: 'ready to dispatch',
    'updated-at': '2026-03-21T11:40:00.000Z',
  });
  writeLaneState(root, 'plot-mode', 3, {
    execution: 'plot-mode',
    lane: 'Lane 3',
    phase: 'correction',
    'expected-next-phase': 'spec-review-pending',
    commit: 'abc1234',
    verification: ['cargo check --manifest-path rust/plot-viewer/Cargo.toml'],
    note: 'Chart wording still needs correction',
    'updated-at': '2026-03-21T11:41:00.000Z',
  });
  writeLaneState(root, 'plot-mode', 5, {
    execution: 'plot-mode',
    lane: 'Lane 5',
    phase: 'coordinator-commit-pending',
    'expected-next-phase': 'spec-review-pending',
    'commit-title': 'docs(plot): 補上 recovery baseline 操作說明',
    'commit-body': '同步本地驗證與啟動步驟',
    verification: ['npm run build'],
    note: 'READY_TO_COMMIT from docs lane',
    'updated-at': '2026-03-21T11:42:00.000Z',
  });

  const {recordInsight} = freshRequire('NLSDD/scripts/nlsdd-record-insight.cjs');
  recordInsight(root, {
    execution: 'plot-mode',
    lane: 'global',
    source: 'coordinator',
    kind: 'improvement-opportunity',
    status: 'open',
    summary: 'Review queue should keep compare-panel blocker visible',
    timestamp: '2026-03-21T11:43:00.000Z',
  });

  const {buildDispatchPlan} = freshRequire('NLSDD/scripts/nlsdd-build-dispatch-plan.cjs');
  const result = buildDispatchPlan(root, 'plot-mode', 4, false);

  assert.equal(result.queue.length, 3);
  assert.equal(result.insightSummary.actionableCount, 1);
  assert.deepEqual(
    result.queue.map((entry) => [entry.kind, entry.lane]),
    [
      ['commit-intake', 'Lane 5'],
      ['review-action', 'Lane 3'],
      ['launch-assignment', 'Lane 2'],
    ],
  );
  assert.equal(result.queue[0].priority, 100);
  assert.equal(result.queue[1].priority, 200);
  assert.equal(result.queue[2].priority, 501);
  assert.match(result.queue[0].message, /Proposed commit title: docs\(plot\): 補上 recovery baseline 操作說明/);
  assert.match(result.queue[1].message, /Reviewer finding: Chart wording still needs correction/);
  assert.match(result.queue[2].message, /Lane item intent: Runtime compare seam follow-up/);
});

test('review helper surfaces actionable execution insights alongside review actions', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-review-insights-'));
  process.env.NLSDD_PROJECT_ROOT = root;

  fs.mkdirSync(path.join(root, 'NLSDD', 'executions', 'plot-mode'), {recursive: true});
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'executions', 'plot-mode', 'lane-3.md'),
    `# Lane 3

> Ownership family:
> \`rust/plot-viewer/src/render/chart.rs\`
>
> NLSDD worktree: \`.worktrees/lane-3-chart\`
>
> Lane-local verification:
> \`cargo check --manifest-path rust/plot-viewer/Cargo.toml\`

## C - Controller

- [ ] Chart compatibility follow-up
`,
    'utf8',
  );
  fs.writeFileSync(
    path.join(root, 'NLSDD', 'scoreboard.md'),
    `# NLSDD Scoreboard

| Execution | Lane | Ownership | Current item | Phase | Item commit | Last verification | Blocked by | Next refill target | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| plot-mode | Lane 3 | Rust chart surface | Chart compatibility follow-up | correction | \`abc1234\` | \`cargo check --manifest-path rust/plot-viewer/Cargo.toml\` | none | none | review row |
`,
    'utf8',
  );
  writeLaneState(root, 'plot-mode', 3, {
    execution: 'plot-mode',
    lane: 'Lane 3',
    phase: 'correction',
    'expected-next-phase': 'spec-review-pending',
    commit: 'abc1234',
    verification: ['cargo check --manifest-path rust/plot-viewer/Cargo.toml'],
    note: 'Chart wording still needs correction',
    'updated-at': '2026-03-21T11:41:00.000Z',
  });

  const {recordInsight} = freshRequire('NLSDD/scripts/nlsdd-record-insight.cjs');
  recordInsight(root, {
    execution: 'plot-mode',
    lane: '3',
    source: 'coordinator',
    kind: 'observed-issue',
    status: 'open',
    summary: 'Review prompts should inspect open execution insights',
    timestamp: '2026-03-21T11:44:00.000Z',
  });

  const {driveReviewLoop, renderActions} = freshRequire('NLSDD/scripts/nlsdd-drive-review-loop.cjs');
  const result = driveReviewLoop(root, 'plot-mode');

  assert.equal(result.actions.length, 1);
  assert.equal(result.insightSummary.actionableCount, 1);
  assert.match(renderActions(result), /Actionable insights: 1/);
  assert.match(renderActions(result), /Review prompts should inspect open execution insights/);
});
