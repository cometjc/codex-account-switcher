const fs = require('node:fs');
const path = require('node:path');
const {execFileSync} = require('node:child_process');
const {
  loadLanePlan,
  loadScoreboardTable,
  resolveProjectRoot,
  resolveScoreboardPath,
} = require('./nlsdd-lib.cjs');

function resolveExecutorDir(projectRoot = resolveProjectRoot()) {
  return path.join(projectRoot, '.nlsdd');
}

function resolveExecutorDbPath(projectRoot = resolveProjectRoot()) {
  return path.join(resolveExecutorDir(projectRoot), 'executor.sqlite');
}

function hasExecutorDb(projectRoot = resolveProjectRoot()) {
  return fs.existsSync(resolveExecutorDbPath(projectRoot));
}

function sqliteEscape(value) {
  if (value == null) {
    return 'NULL';
  }
  return `'${String(value).replace(/'/g, "''")}'`;
}

function runSql(projectRoot, sql) {
  const dbPath = resolveExecutorDbPath(projectRoot);
  fs.mkdirSync(path.dirname(dbPath), {recursive: true});
  return execFileSync('sqlite3', [dbPath, sql], {
    cwd: projectRoot,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trimEnd();
}

function runSqlRows(projectRoot, sql) {
  const dbPath = resolveExecutorDbPath(projectRoot);
  if (!fs.existsSync(dbPath)) {
    return [];
  }
  const output = execFileSync('sqlite3', ['-separator', '\t', dbPath, sql], {
    cwd: projectRoot,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trimEnd();
  if (!output) {
    return [];
  }
  return output.split('\n').map((line) => line.split('\t'));
}

function ensureExecutorDb(projectRoot = resolveProjectRoot()) {
  runSql(
    projectRoot,
    `
pragma journal_mode = wal;
create table if not exists metadata (
  key text primary key,
  value text not null
);
create table if not exists imported_plan_files (
  id integer primary key autoincrement,
  relative_path text not null unique,
  artifact_kind text not null,
  title text not null,
  body text not null,
  imported_at text not null
);
create table if not exists plans (
  id integer primary key autoincrement,
  source_path text not null unique,
  title text not null,
  status text not null,
  imported_at text not null
);
create table if not exists plan_items (
  id integer primary key autoincrement,
  plan_id integer not null references plans(id) on delete cascade,
  ordinal integer not null,
  body text not null,
  status text not null
);
create table if not exists executions (
  id integer primary key autoincrement,
  execution_name text not null unique,
  imported_at text not null
);
create table if not exists lanes (
  id integer primary key autoincrement,
  execution_name text not null,
  lane_name text not null,
  ownership text,
  current_item text,
  phase text not null,
  item_commit text,
  last_verification text,
  blocked_by text,
  next_refill_target text,
  notes text,
  worktree_path text,
  lane_branch text,
  base_branch text,
  result_status text,
  unique(execution_name, lane_name)
);
create table if not exists lane_events (
  id integer primary key autoincrement,
  execution_name text not null,
  lane_name text not null,
  event_type text not null,
  event_json text not null,
  imported_at text not null
);
create table if not exists lane_assignments (
  id integer primary key autoincrement,
  execution_name text not null,
  lane_name text not null,
  assignment_status text not null,
  worktree_path text,
  lane_branch text,
  base_branch text,
  current_item text,
  acceptance_checks text,
  updated_at text not null,
  unique(execution_name, lane_name)
);
create table if not exists lane_results (
  id integer primary key autoincrement,
  execution_name text not null,
  lane_name text not null,
  status text not null,
  result_branch text not null,
  verification_summary text,
  payload_json text,
  created_at text not null
);
    `,
  );
  return resolveExecutorDbPath(projectRoot);
}

function listPlanFiles(projectRoot = resolveProjectRoot()) {
  const planDir = path.join(projectRoot, 'plan');
  if (!fs.existsSync(planDir)) {
    return [];
  }
  return fs
    .readdirSync(planDir)
    .filter((name) => name.endsWith('.md'))
    .sort()
    .map((name) => path.join(planDir, name));
}

function parseHeadingTitle(body, fallback) {
  const match = body.match(/^#\s+(.+)$/m);
  return match ? match[1].trim() : fallback;
}

function parsePlanItems(body) {
  return body
    .split('\n')
    .map((line) => line.match(/^- \[( |x)\] (.+)$/))
    .filter(Boolean)
    .map((match, index) => ({
      ordinal: index + 1,
      body: match[2],
      status: match[1] === 'x' ? 'completed' : 'pending',
    }));
}

function parseCodeValue(value) {
  if (!value) {
    return null;
  }
  const trimmed = value.trim();
  if (trimmed === 'none' || trimmed === '`n/a`' || trimmed === 'n/a') {
    return null;
  }
  return trimmed.replace(/^`|`$/g, '');
}

function laneBranchName(execution, laneName) {
  const numeric = String(laneName).match(/(\d+)/)?.[1] || laneName.toLowerCase().replace(/\s+/g, '-');
  return `nlsdd/${execution}/lane-${numeric}`;
}

function importPlanFiles(projectRoot = resolveProjectRoot(), {cleanup = false} = {}) {
  ensureExecutorDb(projectRoot);
  const now = new Date().toISOString();
  const planFiles = listPlanFiles(projectRoot);
  let importedPlanCount = 0;
  let importedArtifactCount = 0;

  for (const filePath of planFiles) {
    const relativePath = path.relative(projectRoot, filePath);
    const body = fs.readFileSync(filePath, 'utf8');
    const artifactKind = path.basename(filePath) === 'AGENTS.md' ? 'plan-rules' : 'plan';
    const title = parseHeadingTitle(body, path.basename(filePath));

    runSql(
      projectRoot,
      `
insert or replace into imported_plan_files (relative_path, artifact_kind, title, body, imported_at)
values (${sqliteEscape(relativePath)}, ${sqliteEscape(artifactKind)}, ${sqliteEscape(title)}, ${sqliteEscape(body)}, ${sqliteEscape(now)});
      `,
    );
    importedArtifactCount += 1;

    if (artifactKind === 'plan') {
      runSql(
        projectRoot,
        `
insert or replace into plans (source_path, title, status, imported_at)
values (
  ${sqliteEscape(relativePath)},
  ${sqliteEscape(title)},
  ${sqliteEscape(parsePlanItems(body).some((item) => item.status === 'pending') ? 'pending' : 'completed')},
  ${sqliteEscape(now)}
);
delete from plan_items where plan_id = (select id from plans where source_path = ${sqliteEscape(relativePath)});
        `,
      );
      const planId = Number(
        runSql(projectRoot, `select id from plans where source_path = ${sqliteEscape(relativePath)};`),
      );
      for (const item of parsePlanItems(body)) {
        runSql(
          projectRoot,
          `
insert into plan_items (plan_id, ordinal, body, status)
values (${planId}, ${item.ordinal}, ${sqliteEscape(item.body)}, ${sqliteEscape(item.status)});
          `,
        );
      }
      importedPlanCount += 1;
    }

    if (cleanup) {
      fs.unlinkSync(filePath);
    }
  }

  if (cleanup) {
    const planDir = path.join(projectRoot, 'plan');
    if (fs.existsSync(planDir) && fs.readdirSync(planDir).length === 0) {
      // Keep the empty directory in temp fixtures; the real repo can remove it later.
    }
  }

  return {importedPlanCount, importedArtifactCount};
}

function importLegacyExecutionState(projectRoot = resolveProjectRoot()) {
  ensureExecutorDb(projectRoot);
  const scoreboardPath = resolveScoreboardPath(projectRoot);
  if (!fs.existsSync(scoreboardPath)) {
    return {importedExecutionCount: 0, importedLaneCount: 0, importedEventCount: 0};
  }

  const table = loadScoreboardTable(fs.readFileSync(scoreboardPath, 'utf8'), scoreboardPath);
  const executions = new Set();
  let importedLaneCount = 0;
  let importedEventCount = 0;
  const now = new Date().toISOString();

  for (const row of table.objects) {
    executions.add(row.Execution);
    runSql(
      projectRoot,
      `
insert or ignore into executions (execution_name, imported_at)
values (${sqliteEscape(row.Execution)}, ${sqliteEscape(now)});
      `,
    );

    const lanePlan = loadLanePlan(projectRoot, row.Execution, row.Lane);
    const verification = lanePlan?.verificationCommands?.length
      ? JSON.stringify(lanePlan.verificationCommands)
      : JSON.stringify(parseCodeValue(row['Last verification']) ? [parseCodeValue(row['Last verification'])] : []);

    runSql(
      projectRoot,
      `
insert or replace into lanes (
  execution_name,
  lane_name,
  ownership,
  current_item,
  phase,
  item_commit,
  last_verification,
  blocked_by,
  next_refill_target,
  notes,
  worktree_path,
  lane_branch,
  base_branch,
  result_status
)
values (
  ${sqliteEscape(row.Execution)},
  ${sqliteEscape(row.Lane)},
  ${sqliteEscape(row.Ownership || null)},
  ${sqliteEscape(row['Current item'] || null)},
  ${sqliteEscape(row.Phase || 'parked')},
  ${sqliteEscape(parseCodeValue(row['Item commit']))},
  ${sqliteEscape(verification)},
  ${sqliteEscape(parseCodeValue(row['Blocked by']))},
  ${sqliteEscape(row['Next refill target'] || null)},
  ${sqliteEscape(row.Notes || null)},
  ${sqliteEscape(lanePlan?.worktreePath || null)},
  ${sqliteEscape(laneBranchName(row.Execution, row.Lane))},
  'main',
  ${sqliteEscape(row.Phase === 'done' ? 'DONE' : null)}
);
      `,
    );
    importedLaneCount += 1;

    const numericLane = row.Lane.match(/(\d+)/)?.[1];
    const eventPath = numericLane
      ? path.join(projectRoot, 'NLSDD', 'state', row.Execution, 'events.ndjson')
      : null;
    if (eventPath && fs.existsSync(eventPath)) {
      const lines = fs
        .readFileSync(eventPath, 'utf8')
        .split('\n')
        .filter(Boolean)
        .map((line) => JSON.parse(line))
        .filter((entry) => entry.execution === row.Execution && entry.lane === row.Lane);

      for (const entry of lines) {
        runSql(
          projectRoot,
          `
insert into lane_events (execution_name, lane_name, event_type, event_json, imported_at)
values (
  ${sqliteEscape(entry.execution)},
  ${sqliteEscape(entry.lane)},
  ${sqliteEscape(entry.eventType || 'event')},
  ${sqliteEscape(JSON.stringify(entry))},
  ${sqliteEscape(now)}
);
          `,
        );
        importedEventCount += 1;
      }
    }
  }

  return {
    importedExecutionCount: executions.size,
    importedLaneCount,
    importedEventCount,
  };
}

function auditExecutor(projectRoot = resolveProjectRoot()) {
  ensureExecutorDb(projectRoot);
  const planFiles = listPlanFiles(projectRoot).map((filePath) => path.relative(projectRoot, filePath));
  const pendingPlanCount = Number(runSql(projectRoot, "select count(*) from plans where status != 'completed';") || '0');
  const queuedLaneCount = Number(runSql(projectRoot, "select count(*) from lanes where phase in ('queued', 'implementing');") || '0');
  const reviewLaneCount = Number(
    runSql(projectRoot, "select count(*) from lanes where result_status = 'READY_FOR_REVIEW';") || '0',
  );
  const blockingIssues = [];
  if (planFiles.length > 0) {
    blockingIssues.push('plan-directory-not-empty');
  }
  return {
    planFiles,
    pendingPlanCount,
    queuedLaneCount,
    reviewLaneCount,
    blockingIssues,
  };
}

function goExecutor(projectRoot = resolveProjectRoot()) {
  const audit = auditExecutor(projectRoot);
  if (audit.planFiles.length > 0) {
    throw new Error('plan/ must be empty before executor go can continue');
  }
  return {
    status: audit.pendingPlanCount === 0 && audit.reviewLaneCount === 0 && audit.queuedLaneCount === 0 ? 'idle' : 'active',
    ...audit,
  };
}

function emptyInsightSummary() {
  return {
    total: 0,
    actionableCount: 0,
    durableLearningCount: 0,
    resolvedHistoryCount: 0,
    countsByStatus: {},
    countsByKind: {},
    actionable: [],
    durableLearnings: [],
    resolvedHistory: [],
    latest: [],
  };
}

function listLanes(projectRoot = resolveProjectRoot(), execution) {
  return runSqlRows(
    projectRoot,
    `
select
  lane_name,
  coalesce(current_item, ''),
  phase,
  coalesce(worktree_path, ''),
  coalesce(lane_branch, ''),
  coalesce(base_branch, ''),
  coalesce(last_verification, '[]'),
  coalesce(result_status, '')
from lanes
where execution_name = ${sqliteEscape(execution)}
order by lane_name;
    `,
  ).map(([laneName, currentItem, phase, worktreePath, laneBranch, baseBranch, lastVerification, resultStatus]) => ({
    laneName,
    currentItem,
    phase,
    worktreePath: worktreePath || null,
    laneBranch: laneBranch || null,
    baseBranch: baseBranch || 'main',
    verification: JSON.parse(lastVerification || '[]'),
    resultStatus: resultStatus || null,
  }));
}

function buildCoordinatorLoopFromExecutor(
  projectRoot = resolveProjectRoot(),
  execution,
  maxActive = 4,
  dryRun = false,
) {
  ensureExecutorDb(projectRoot);
  const lanes = listLanes(projectRoot, execution);
  const activeCount = lanes.filter((entry) => entry.phase === 'implementing').length;
  const availableSlots = Math.max(0, maxActive - activeCount);
  const dispatchable = lanes.filter((entry) => entry.phase === 'queued').slice(0, availableSlots);
  const idleSlots = Math.max(0, maxActive - activeCount - dispatchable.length);
  const reviewActions = lanes
    .filter((entry) => entry.resultStatus === 'READY_FOR_REVIEW')
    .map((entry) => ({
      lane: entry.laneName,
      action: 'spec-review',
      phase: 'review-pending',
      message: [
        `Execution: ${execution}`,
        `Lane: ${entry.laneName}`,
        `Result branch ready for review.`,
        `Worktree: ${entry.worktreePath || 'n/a'}`,
      ].join('\n'),
    }));
  const assignments = dispatchable.map((entry) => ({
    execution,
    lane: entry.laneName,
    nextItem: entry.currentItem,
    promotedPhase: 'implementing',
    worktreePath: entry.worktreePath,
    verification: entry.verification,
    scope: entry.worktreePath ? [entry.worktreePath] : [],
    laneBranch: entry.laneBranch,
    baseBranch: entry.baseBranch,
    message: [
      `Execution: ${execution}`,
      `Lane: ${entry.laneName}`,
      `Lane item intent: ${entry.currentItem || 'n/a'}`,
      `Worktree: ${entry.worktreePath || 'n/a'}`,
      `Lane branch: ${entry.laneBranch || 'n/a'}`,
      `Base branch: ${entry.baseBranch || 'main'}`,
      `Verification: ${entry.verification.join('; ') || 'n/a'}`,
    ].join('\n'),
  }));

  return {
    source: 'executor',
    execution,
    maxActiveThreads: maxActive,
    dryRun,
    launch: {
      assignments,
      promoted: dispatchable.map((entry) => ({lane: entry.laneName})),
      completedLanes: [],
      idleSlots,
      noDispatchReason: dispatchable.length === 0 ? 'no-dispatchable-lane' : null,
    },
    reviewActions,
    commitIntake: [],
    insightSummary: emptyInsightSummary(),
    idleSlots,
    completedLanes: [],
    promotedLanes: dispatchable.map((entry) => entry.laneName),
    reviewLaneCount: reviewActions.length,
    commitLaneCount: 0,
    noDispatchReason: dispatchable.length === 0 ? 'no-dispatchable-lane' : null,
    degradedSurfaces: [],
    telemetrySummary: null,
    telemetryReviewPath: null,
  };
}

function buildCycleFromExecutor(
  projectRoot = resolveProjectRoot(),
  execution,
  maxActive = 4,
  dryRun = false,
) {
  const coordinator = buildCoordinatorLoopFromExecutor(projectRoot, execution, maxActive, dryRun);
  return {
    source: 'executor',
    execution,
    maxActiveThreads: maxActive,
    dryRun,
    reconciled: [],
    promoted: coordinator.launch.promoted.map((entry, index) => ({
      slot: index + 1,
      lane: entry.lane,
      from: 'queued',
      to: 'implementing',
      nextItem:
        coordinator.launch.assignments.find((assignment) => assignment.lane === entry.lane)?.nextItem || null,
      nextItemSection: null,
    })),
    observedDegradedScoreboardLoad: null,
    idleSlots: coordinator.idleSlots,
    completedLanes: [],
    noDispatchReason: coordinator.noDispatchReason,
    finalSchedule: {
      source: 'executor',
      activeRows: [],
      queuedRows: [],
      staleRows: [],
      availableSlots: coordinator.idleSlots,
      dispatchSuggestions: [],
    },
  };
}

function buildLaunchFromExecutor(
  projectRoot = resolveProjectRoot(),
  execution,
  maxActive = 4,
  dryRun = false,
) {
  const coordinator = buildCoordinatorLoopFromExecutor(projectRoot, execution, maxActive, dryRun);
  return {
    source: 'executor',
    execution,
    maxActiveThreads: maxActive,
    dryRun,
    reconciled: [],
    promoted: coordinator.launch.promoted.map((entry, index) => ({
      slot: index + 1,
      lane: entry.lane,
      from: 'queued',
      to: 'implementing',
      nextItem:
        coordinator.launch.assignments.find((assignment) => assignment.lane === entry.lane)?.nextItem || null,
      nextItemSection: null,
    })),
    observedDegradedScoreboardLoad: null,
    idleSlots: coordinator.idleSlots,
    completedLanes: [],
    noDispatchReason: coordinator.noDispatchReason,
    finalSchedule: {
      source: 'executor',
      activeRows: [],
      queuedRows: [],
      staleRows: [],
      availableSlots: coordinator.idleSlots,
      dispatchSuggestions: [],
    },
    assignments: coordinator.launch.assignments.map((assignment, index) => ({
      slot: index + 1,
      lane: assignment.lane,
      currentPhase: 'queued',
      promotedPhase: 'implementing',
      nextItem: assignment.nextItem,
      nextItemSection: null,
      scope: assignment.scope.join('; '),
      verification: assignment.verification,
      worktreePath: assignment.worktreePath,
      message: assignment.message,
    })),
  };
}

function buildReviewLoopFromExecutor(projectRoot = resolveProjectRoot(), execution, lane = null) {
  ensureExecutorDb(projectRoot);
  const actions = runSqlRows(
    projectRoot,
    `
select
  l.lane_name,
  coalesce(l.current_item, ''),
  coalesce(l.worktree_path, ''),
  coalesce(r.result_branch, ''),
  coalesce(r.verification_summary, '')
from lanes l
join lane_results r
  on r.id = (
    select lr.id
    from lane_results lr
    where lr.execution_name = l.execution_name and lr.lane_name = l.lane_name
    order by lr.id desc
    limit 1
  )
where l.execution_name = ${sqliteEscape(execution)}
  and l.result_status = 'READY_FOR_REVIEW'
  ${lane ? `and l.lane_name = ${sqliteEscape(lane)}` : ''}
order by l.lane_name;
    `,
  ).map(([laneName, currentItem, worktreePath, resultBranch, verificationSummary]) => ({
    execution,
    lane: laneName,
    phase: 'review-pending',
    item: currentItem,
    commit: resultBranch,
    action: 'spec-review',
    message: [
      `Execution: ${execution}`,
      `Lane: ${laneName}`,
      `Lane item: ${currentItem || 'n/a'}`,
      `Result branch: ${resultBranch || 'n/a'}`,
      `Worktree: ${worktreePath || 'n/a'}`,
      `Verification: ${verificationSummary || 'n/a'}`,
    ].join('\n'),
  }));

  return {
    source: 'executor',
    execution,
    lane: lane || null,
    actions,
    insightSummary: emptyInsightSummary(),
  };
}

function buildCommitIntakeFromExecutor(projectRoot = resolveProjectRoot(), execution, lane = null) {
  ensureExecutorDb(projectRoot);
  const entries = runSqlRows(
    projectRoot,
    `
select
  l.lane_name,
  coalesce(l.current_item, ''),
  coalesce(l.worktree_path, ''),
  coalesce(r.result_branch, ''),
  coalesce(r.verification_summary, ''),
  coalesce(l.ownership, '')
from lanes l
join lane_results r
  on r.id = (
    select lr.id
    from lane_results lr
    where lr.execution_name = l.execution_name and lr.lane_name = l.lane_name
    order by lr.id desc
    limit 1
  )
where l.execution_name = ${sqliteEscape(execution)}
  and l.result_status = 'DONE'
  ${lane ? `and l.lane_name = ${sqliteEscape(lane)}` : ''}
order by l.lane_name;
    `,
  ).map(([laneName, currentItem, worktreePath, resultBranch, verificationSummary, ownership]) => ({
    execution,
    lane: laneName,
    phase: 'done',
    item: currentItem,
    commit: resultBranch,
    proposedCommitTitle: null,
    proposedCommitBody: null,
    scope: ownership ? [ownership] : [],
    verification: verificationSummary ? [verificationSummary] : [],
    note: `executor result branch: ${resultBranch || 'n/a'}`,
    nextExpectedPhase: null,
    worktreePath: worktreePath || null,
  }));

  return {
    source: 'executor',
    entries,
    degradedSurfaces: [],
  };
}

function claimAssignment(projectRoot = resolveProjectRoot(), execution, lane) {
  ensureExecutorDb(projectRoot);
  const laneJson = runSql(
    projectRoot,
    `
select json_object(
  'execution', execution_name,
  'lane', lane_name,
  'phase', phase,
  'currentItem', current_item,
  'worktreePath', worktree_path,
  'laneBranch', lane_branch,
  'baseBranch', base_branch,
  'acceptanceChecks', json(last_verification)
)
from lanes
where execution_name = ${sqliteEscape(execution)} and lane_name = ${sqliteEscape(lane)}
limit 1;
    `,
  );
  if (!laneJson) {
    throw new Error(`Unknown lane ${execution} ${lane}`);
  }
  const assignment = JSON.parse(laneJson);
  runSql(
    projectRoot,
    `
insert or replace into lane_assignments (
  execution_name,
  lane_name,
  assignment_status,
  worktree_path,
  lane_branch,
  base_branch,
  current_item,
  acceptance_checks,
  updated_at
)
values (
  ${sqliteEscape(execution)},
  ${sqliteEscape(lane)},
  'claimed',
  ${sqliteEscape(assignment.worktreePath)},
  ${sqliteEscape(assignment.laneBranch)},
  ${sqliteEscape(assignment.baseBranch)},
  ${sqliteEscape(assignment.currentItem)},
  ${sqliteEscape(JSON.stringify(assignment.acceptanceChecks || []))},
  ${sqliteEscape(new Date().toISOString())}
);
    `,
  );
  return assignment;
}

function mapResultStatusToLanePhase(status) {
  switch (status) {
    case 'RUNNING':
      return 'implementing';
    case 'BLOCKED':
      return 'blocked';
    case 'READY_FOR_REVIEW':
      return 'review-pending';
    case 'DONE':
      return 'done';
    case 'FAILED':
      return 'failed';
    case 'CANCELLED':
      return 'parked';
    default:
      throw new Error(`Unknown result status ${status}`);
  }
}

function reportResult(
  projectRoot = resolveProjectRoot(),
  execution,
  lane,
  status,
  resultBranch,
  payload = {},
) {
  ensureExecutorDb(projectRoot);
  const now = new Date().toISOString();
  runSql(
    projectRoot,
    `
insert into lane_results (
  execution_name,
  lane_name,
  status,
  result_branch,
  verification_summary,
  payload_json,
  created_at
)
values (
  ${sqliteEscape(execution)},
  ${sqliteEscape(lane)},
  ${sqliteEscape(status)},
  ${sqliteEscape(resultBranch)},
  ${sqliteEscape(payload.verificationSummary || null)},
  ${sqliteEscape(JSON.stringify(payload))},
  ${sqliteEscape(now)}
);
update lanes
set phase = ${sqliteEscape(mapResultStatusToLanePhase(status))},
    result_status = ${sqliteEscape(status)}
where execution_name = ${sqliteEscape(execution)} and lane_name = ${sqliteEscape(lane)};
update lane_assignments
set assignment_status = ${sqliteEscape(status.toLowerCase())},
    updated_at = ${sqliteEscape(now)}
where execution_name = ${sqliteEscape(execution)} and lane_name = ${sqliteEscape(lane)};
    `,
  );
  return {
    execution,
    lane,
    status,
    resultBranch,
    verificationSummary: payload.verificationSummary || null,
  };
}

module.exports = {
  buildCycleFromExecutor,
  buildCoordinatorLoopFromExecutor,
  buildLaunchFromExecutor,
  buildCommitIntakeFromExecutor,
  buildReviewLoopFromExecutor,
  auditExecutor,
  claimAssignment,
  ensureExecutorDb,
  goExecutor,
  hasExecutorDb,
  importLegacyExecutionState,
  importPlanFiles,
  reportResult,
  resolveExecutorDbPath,
};
