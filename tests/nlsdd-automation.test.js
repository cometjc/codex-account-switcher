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

| Execution | Lane | Ownership | Current item | Phase | Effective phase | Item commit | Branch HEAD | Last verification | Last probe | Latest event | Correction count | Last activity | Blocked by | Next refill target | Noise | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| plot-mode | Lane 1 | Node contract + handoff | Viewer launch confidence / shell messaging | spec-review-pending | manual-review-needed | \`abc1234\` | \`n/a\` | \`git status --short\` | n/a | n/a | 0 | n/a | none | Tighten snapshot builder semantics when real 7d history evolves | none | test row |
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

| Execution | Lane | Ownership | Current item | Phase | Effective phase | Item commit | Branch HEAD | Last verification | Last probe | Latest event | Correction count | Last activity | Blocked by | Next refill target | Noise | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| nlsdd-self-hosting | Lane 1 | Scheduler core | Normalize scheduling phases for multi-lane dispatch | implementing | implementing | \`1a2b3c4\` | \`1a2b3c4\` | \`node --test tests/nlsdd-automation.test.js\` | 2026-03-21 03:10:00Z · HEAD 1a2b3c4 · clean | PASS · Feynman · 2026-03-21 03:10:00Z | 0 | 2026-03-21 03:10:00Z | none | Scheduler edge cases | none | active lane |
| nlsdd-self-hosting | Lane 2 | Scoreboard integration | Keep scoreboard rows aligned with the active-cap model | spec-review-pending | spec-review-pending | \`2a2b3c4\` | \`2a2b3c4\` | \`npm run nlsdd:scoreboard:refresh\` | 2026-03-21 03:11:00Z · HEAD 2a2b3c4 · clean | PASS · Banach · 2026-03-21 03:11:00Z | 0 | 2026-03-21 03:11:00Z | none | Scoreboard wording polish | none | active lane |
| nlsdd-self-hosting | Lane 3 | Rules and communication | Rewrite remaining fixed-lane wording | refill-ready | refill-ready | \`3a2b3c4\` | \`3a2b3c4\` | \`rg -n "active lane count" spec/NLSDD; rg -n "4 active lanes" NLSDD\` | 2026-03-21 03:12:00Z · HEAD 3a2b3c4 · clean | PASS · Archimedes · 2026-03-21 03:12:00Z | 0 | 2026-03-21 03:12:00Z | none | Execution wording cleanup | none | ready to refill |
| nlsdd-self-hosting | Lane 4 | Regression and CLI surface | Add schedule regression coverage | refill-ready | refill-ready | \`4a2b3c4\` | \`4a2b3c4\` | \`node --test tests/nlsdd-automation.test.js\` | 2026-03-21 03:13:00Z · HEAD 4a2b3c4 · clean | PASS · Helmholtz · 2026-03-21 03:13:00Z | 0 | 2026-03-21 03:13:00Z | none | Scoreboard/schedule cross-check coverage | none | ready to refill |
| nlsdd-self-hosting | Lane 5 | Plot-mode migration | Adjust plot-mode docs to lane-pool language | queued | queued | \`n/a\` | \`n/a\` | \`rg -n "lane pool" NLSDD/executions/plot-mode\` | n/a | n/a | 0 | n/a | wait-slot | Plot-mode overview wording | none | queued lane |
| nlsdd-self-hosting | Lane 6 | Coordinator follow-up | Capture coordinator ergonomics follow-up | queued | queued | \`n/a\` | \`n/a\` | \`sed -n '1,220p' tasks/todo.md\` | n/a | n/a | 0 | n/a | wait-slot | Coordinator follow-up | none | queued lane |
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
  assert.match(runtimeText, /\| plot-mode \| Lane 1 \|/);
  assert.match(runtimeText, /\| quality-review-pending \|/);
  assert.match(runtimeText, /PASS · Meitner · 2026-03-21 03:00:00Z/);
  assert.match(runtimeText, /\| 1 \| 2026-03-20 18:37:48Z \|/);
  assert.match(runtimeText, /mixed/);
  assert.match(runtimeText, /## Recent Codex Threads/);
});

test('resolveProjectRoot returns the canonical repo root when called from a linked worktree', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-nlsdd-root-'));
  fs.mkdirSync(path.join(root, '.worktrees'), {recursive: true});
  run('git', ['init', '-q'], root);
  run('git', ['config', 'user.name', 'Codex Test'], root);
  run('git', ['config', 'user.email', 'codex@example.com'], root);
  fs.writeFileSync(path.join(root, 'tracked.txt'), 'root\n', 'utf8');
  run('git', ['add', 'tracked.txt'], root);
  run('git', ['commit', '-m', 'init'], root);
  run('git', ['worktree', 'add', path.join(root, '.worktrees', 'lane-1-node'), '-b', 'lane-1-node'], root);

  const originalCwd = process.cwd();
  const modulePath = path.join(originalCwd, 'NLSDD', 'scripts', 'nlsdd-lib.cjs');
  delete process.env.NLSDD_PROJECT_ROOT;
  try {
    process.chdir(path.join(root, '.worktrees', 'lane-1-node'));
    delete require.cache[require.resolve(modulePath)];
    const {resolveProjectRoot} = require(modulePath);
    assert.equal(resolveProjectRoot(), root);
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
  assert.match(message, /Return a new commit sha and verification results/);
  assert.doesNotMatch(message, /talk directly to reviewer/i);
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
  assert.equal(state.lastReviewerResult, 'PASS');
  assert.deepEqual(state.lastVerification, ['cargo test', 'cargo check']);
  assert.equal(state.blockedBy, 'none');
  assert.equal(state.note, 'ready for quality review');
  assert.equal(state.correctionCount, 3);
  assert.equal(state.updatedAt, '2026-03-21T04:20:00.000Z');
});
