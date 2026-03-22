const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {execFileSync} = require('node:child_process');

function repoRoot(...segments) {
  return path.join(process.cwd(), ...segments);
}

function run(command, args, cwd) {
  return execFileSync(command, args, {
    cwd,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trimEnd();
}

function setupTempGitRepo(dir) {
  fs.mkdirSync(dir, {recursive: true});
  run('git', ['init', '-q'], dir);
  run('git', ['config', 'user.name', 'Codex Test'], dir);
  run('git', ['config', 'user.email', 'codex@example.com'], dir);
  fs.writeFileSync(path.join(dir, 'tracked.txt'), 'initial\n', 'utf8');
  run('git', ['add', 'tracked.txt'], dir);
  run('git', ['commit', '-m', 'init'], dir);
}

function writeFixture(root) {
  const planDir = path.join(root, 'plan');
  const executionDir = path.join(root, 'NLSDD', 'executions', 'demo-flow');
  const stateDir = path.join(root, 'NLSDD', 'state', 'demo-flow');
  const laneWorktree = path.join(root, '.worktrees', 'lane-1-demo');

  fs.mkdirSync(planDir, {recursive: true});
  fs.mkdirSync(executionDir, {recursive: true});
  fs.mkdirSync(stateDir, {recursive: true});
  setupTempGitRepo(laneWorktree);

  fs.writeFileSync(
    path.join(planDir, 'AGENTS.md'),
    `# Plan Rules

- live plans must be imported into the executor
`,
    'utf8',
  );

  fs.writeFileSync(
    path.join(planDir, '2026-03-22-demo-plan.md'),
    `# Demo Migration Plan

- [x] Close drift from the old tracked scoreboard
- [ ] Build the central executor import path
- [ ] Route lane result exchange through result branches
`,
    'utf8',
  );

  fs.writeFileSync(
    path.join(root, 'NLSDD', 'scoreboard.md'),
    `# NLSDD Scoreboard

| Execution | Lane | Ownership | Current item | Phase | Item commit | Last verification | Blocked by | Next refill target | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| demo-flow | Lane 1 | Executor core | Build central executor import path | queued | \`abc1234\` | \`node --test tests/nlsdd-executor.test.js\` | none | Route lane result exchange through result branches | imported fixture |
`,
    'utf8',
  );

  fs.writeFileSync(
    path.join(executionDir, 'lane-1.md'),
    `# Lane 1 Plan - Executor Core

> Ownership family:
> \`NLSDD/scripts/nlsdd-executor.cjs\`
>
> NLSDD worktree: \`.worktrees/lane-1-demo\`
>
> Lane-local verification:
> \`node --test tests/nlsdd-executor.test.js\`

## M - Work Item

- [ ] Build central executor import path
- [ ] Route lane result exchange through result branches

## Current Lane Status

- [x] Projected phase: queued
`,
    'utf8',
  );

  fs.writeFileSync(
    path.join(stateDir, 'events.ndjson'),
    `${JSON.stringify({
      execution: 'demo-flow',
      lane: 'Lane 1',
      role: 'coordinator',
      eventType: 'state-update',
      phaseBefore: 'parked',
      phaseAfter: 'queued',
      currentItem: 'Build central executor import path',
      nextRefillTarget: 'Route lane result exchange through result branches',
      relatedCommit: 'abc1234',
      verification: ['node --test tests/nlsdd-executor.test.js'],
      summary: 'queued for executor migration',
      detail: 'queued for executor migration',
      timestamp: '2026-03-22T00:00:00.000Z',
      insights: [],
      eventId: 'evt-1',
    })}\n`,
    'utf8',
  );

  return {laneWorktree};
}

function runExecutor(root, ...args) {
  return run('node', [repoRoot('NLSDD', 'scripts', 'nlsdd-executor.cjs'), '--project-root', root, ...args], root);
}

function readScalar(dbPath, sql) {
  return run('sqlite3', [dbPath, sql], path.dirname(dbPath));
}

test('executor go blocks until plan directory is empty', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-executor-go-'));
  writeFixture(root);

  assert.throws(
    () => runExecutor(root, 'go', '--json'),
    /plan\/ must be empty before executor go can continue/,
  );
});

test('executor import-plans ingests legacy plans and cleans plan directory', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-executor-import-'));
  writeFixture(root);

  const result = JSON.parse(runExecutor(root, 'import-plans', '--cleanup', '--json'));
  const dbPath = path.join(root, '.nlsdd', 'executor.sqlite');

  assert.equal(result.importedPlanCount, 1);
  assert.equal(result.importedArtifactCount, 2);
  assert.equal(result.importedExecutionCount, 1);
  assert.equal(result.importedLaneCount, 1);
  assert.equal(fs.existsSync(dbPath), true);
  assert.deepEqual(fs.readdirSync(path.join(root, 'plan')), []);
  assert.equal(readScalar(dbPath, 'select count(*) from imported_plan_files;'), '2');
  assert.equal(readScalar(dbPath, "select count(*) from plans where title = 'Demo Migration Plan';"), '1');
  assert.equal(readScalar(dbPath, "select count(*) from plan_items where status = 'pending';"), '2');
  assert.equal(readScalar(dbPath, "select worktree_path from lanes where execution_name = 'demo-flow' and lane_name = 'Lane 1';"), path.join(root, '.worktrees', 'lane-1-demo'));
});

test('executor claim-assignment and report-result use the sqlite authority path', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-executor-result-'));
  writeFixture(root);
  runExecutor(root, 'import-plans', '--cleanup', '--json');

  const assignment = JSON.parse(runExecutor(root, 'claim-assignment', '--execution', 'demo-flow', '--lane', 'Lane 1', '--json'));
  const dbPath = path.join(root, '.nlsdd', 'executor.sqlite');

  assert.equal(assignment.execution, 'demo-flow');
  assert.equal(assignment.lane, 'Lane 1');
  assert.equal(assignment.phase, 'queued');
  assert.match(assignment.laneBranch, /^nlsdd\/demo-flow\/lane-1$/);
  assert.equal(assignment.worktreePath, path.join(root, '.worktrees', 'lane-1-demo'));

  const report = JSON.parse(
    runExecutor(
      root,
      'report-result',
      '--execution',
      'demo-flow',
      '--lane',
      'Lane 1',
      '--status',
      'READY_FOR_REVIEW',
      '--result-branch',
      'results/demo-flow/lane-1',
      '--verification-summary',
      'node --test tests/nlsdd-executor.test.js',
      '--json',
    ),
  );

  assert.equal(report.status, 'READY_FOR_REVIEW');
  assert.equal(report.resultBranch, 'results/demo-flow/lane-1');
  assert.equal(
    readScalar(
      dbPath,
      "select result_branch from lane_results where execution_name = 'demo-flow' and lane_name = 'Lane 1' order by id desc limit 1;",
    ),
    'results/demo-flow/lane-1',
  );
  assert.equal(
    readScalar(
      dbPath,
      "select result_status from lanes where execution_name = 'demo-flow' and lane_name = 'Lane 1';",
    ),
    'READY_FOR_REVIEW',
  );
});

test('legacy coordinator and dispatch helpers degrade to executor-backed summaries after migration', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-executor-compat-'));
  writeFixture(root);
  runExecutor(root, 'import-plans', '--cleanup', '--json');

  const coordinator = JSON.parse(
    run(
      'node',
      [
        repoRoot('NLSDD', 'scripts', 'nlsdd-run-coordinator-loop.cjs'),
        '--execution',
        'demo-flow',
        '--dry-run',
        '--json',
      ],
      root,
    ),
  );
  assert.equal(coordinator.source, 'executor');
  assert.deepEqual(coordinator.promotedLanes, ['Lane 1']);
  assert.equal(coordinator.launch.assignments.length, 1);
  assert.equal(coordinator.launch.assignments[0].lane, 'Lane 1');

  const dispatchPlan = JSON.parse(
    run(
      'node',
      [
        repoRoot('NLSDD', 'scripts', 'nlsdd-build-dispatch-plan.cjs'),
        '--execution',
        'demo-flow',
        '--dry-run',
        '--json',
      ],
      root,
    ),
  );
  assert.equal(dispatchPlan.source, 'executor');
  assert.equal(dispatchPlan.queue.length, 1);
  assert.equal(dispatchPlan.queue[0].kind, 'launch-assignment');
  assert.equal(dispatchPlan.queue[0].lane, 'Lane 1');
});

test('legacy review and intake helpers read executor-backed lane outcomes after migration', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-auth-executor-review-'));
  writeFixture(root);
  runExecutor(root, 'import-plans', '--cleanup', '--json');
  runExecutor(
    root,
    'report-result',
    '--execution',
    'demo-flow',
    '--lane',
    'Lane 1',
    '--status',
    'READY_FOR_REVIEW',
    '--result-branch',
    'results/demo-flow/lane-1-review',
    '--verification-summary',
    'node --test tests/nlsdd-executor.test.js',
    '--json',
  );

  const review = JSON.parse(
    run(
      'node',
      [
        repoRoot('NLSDD', 'scripts', 'nlsdd-drive-review-loop.cjs'),
        '--execution',
        'demo-flow',
        '--json',
      ],
      root,
    ),
  );
  assert.equal(review.source, 'executor');
  assert.equal(review.actions.length, 1);
  assert.equal(review.actions[0].lane, 'Lane 1');
  assert.equal(review.actions[0].action, 'spec-review');
  assert.match(review.actions[0].message, /results\/demo-flow\/lane-1-review/);

  runExecutor(
    root,
    'report-result',
    '--execution',
    'demo-flow',
    '--lane',
    'Lane 1',
    '--status',
    'DONE',
    '--result-branch',
    'results/demo-flow/lane-1-final',
    '--verification-summary',
    'node --test tests/nlsdd-executor.test.js',
    '--json',
  );

  const intake = JSON.parse(
    run(
      'node',
      [
        repoRoot('NLSDD', 'scripts', 'nlsdd-intake-ready-to-commit.cjs'),
        '--execution',
        'demo-flow',
        '--json',
      ],
      root,
    ),
  );
  assert.equal(intake.source, 'executor');
  assert.equal(intake.entries.length, 1);
  assert.equal(intake.entries[0].lane, 'Lane 1');
  assert.equal(intake.entries[0].phase, 'done');
  assert.equal(intake.entries[0].commit, 'results/demo-flow/lane-1-final');
  assert.match(intake.entries[0].note, /executor result branch/);
});
