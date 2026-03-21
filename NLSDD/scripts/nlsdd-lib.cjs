const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {execFileSync} = require('node:child_process');

function findNearestNlsddRoot(startPath = process.cwd()) {
  let currentPath = path.resolve(startPath);

  while (true) {
    const scoreboardPath = path.join(currentPath, 'NLSDD', 'scoreboard.md');
    const executionsPath = path.join(currentPath, 'NLSDD', 'executions');
    if (fs.existsSync(scoreboardPath) || fs.existsSync(executionsPath)) {
      return currentPath;
    }

    const parentPath = path.dirname(currentPath);
    if (parentPath === currentPath) {
      return null;
    }
    currentPath = parentPath;
  }
}

function resolveProjectRoot() {
  if (process.env.NLSDD_PROJECT_ROOT) {
    return process.env.NLSDD_PROJECT_ROOT;
  }

  const localNlsddRoot = findNearestNlsddRoot();
  if (localNlsddRoot) {
    return localNlsddRoot;
  }

  try {
    const commonDir = execFileSync(
      'git',
      ['rev-parse', '--path-format=absolute', '--git-common-dir'],
      {
        cwd: process.cwd(),
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'pipe'],
      },
    ).trim();
    if (commonDir) {
      return path.basename(commonDir) === '.git' ? path.dirname(commonDir) : commonDir;
    }
  } catch {
    // Fall back to cwd when not in a git worktree.
  }

  return process.cwd();
}

function resolveWorktreePoolRoot(projectRoot = resolveProjectRoot()) {
  if (process.env.NLSDD_WORKTREE_POOL_ROOT) {
    return process.env.NLSDD_WORKTREE_POOL_ROOT;
  }

  try {
    const commonDir = execFileSync(
      'git',
      ['rev-parse', '--path-format=absolute', '--git-common-dir'],
      {
        cwd: projectRoot,
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'pipe'],
      },
    ).trim();
    if (commonDir) {
      return path.basename(commonDir) === '.git' ? path.dirname(commonDir) : commonDir;
    }
  } catch {
    // Fall back to the execution root when git metadata is unavailable.
  }

  return projectRoot;
}

function isPathWithin(basePath, candidatePath) {
  const resolvedBase = path.resolve(basePath);
  const resolvedCandidate = path.resolve(candidatePath);
  return (
    resolvedCandidate === resolvedBase ||
    resolvedCandidate.startsWith(`${resolvedBase}${path.sep}`)
  );
}

function resolveScoreboardPath(projectRoot = resolveProjectRoot()) {
  const envScoreboardPath = process.env.NLSDD_SCOREBOARD_PATH;
  if (envScoreboardPath && isPathWithin(projectRoot, envScoreboardPath)) {
    return envScoreboardPath;
  }
  return path.join(projectRoot, 'NLSDD', 'scoreboard.md');
}

function resolveRuntimeScoreboardPath(projectRoot = resolveProjectRoot()) {
  const envRuntimeScoreboardPath = process.env.NLSDD_RUNTIME_SCOREBOARD_PATH;
  if (envRuntimeScoreboardPath && isPathWithin(projectRoot, envRuntimeScoreboardPath)) {
    return envRuntimeScoreboardPath;
  }
  return path.join(projectRoot, 'NLSDD', 'state', 'scoreboard.runtime.md');
}

function resolvePreferredScoreboardPath(projectRoot = resolveProjectRoot()) {
  const runtimeScoreboardPath = resolveRuntimeScoreboardPath(projectRoot);
  if (fs.existsSync(runtimeScoreboardPath)) {
    return runtimeScoreboardPath;
  }
  return resolveScoreboardPath(projectRoot);
}

function executionInsightsPath(projectRoot, execution) {
  if (!projectRoot || !execution) {
    return null;
  }
  return path.join(projectRoot, 'NLSDD', 'state', execution, 'execution-insights.ndjson');
}

function resolveCodexStateDbPath() {
  return process.env.CODEX_STATE_DB_PATH || path.join(os.homedir(), '.codex', 'state_5.sqlite');
}

function resolveCodexSessionsRoot() {
  return process.env.CODEX_SESSIONS_ROOT || path.join(os.homedir(), '.codex', 'sessions');
}

function run(command, args, cwd = resolveProjectRoot()) {
  return execFileSync(command, args, {
    cwd,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trimEnd();
}

function tryRun(command, args, cwd = resolveProjectRoot()) {
  try {
    return run(command, args, cwd);
  } catch {
    return '';
  }
}

function parseMarkdownTable(lines, startIndex) {
  const header = lines[startIndex];
  const separator = lines[startIndex + 1];
  const rows = [];
  let endIndex = startIndex + 2;
  while (endIndex < lines.length && lines[endIndex].startsWith('|')) {
    rows.push(lines[endIndex]);
    endIndex += 1;
  }
  return {header, separator, rows, endIndex};
}

function splitMarkdownTableRow(row) {
  const cells = [];
  let current = '';
  let inCodeSpan = false;
  let escaped = false;

  for (let index = 0; index < row.length; index += 1) {
    const char = row[index];
    if (escaped) {
      current += char;
      escaped = false;
      continue;
    }
    if (char === '\\') {
      current += char;
      escaped = true;
      continue;
    }
    if (char === '`') {
      current += char;
      inCodeSpan = !inCodeSpan;
      continue;
    }
    if (char === '|' && !inCodeSpan) {
      cells.push(current.trim());
      current = '';
      continue;
    }
    current += char;
  }

  cells.push(current.trim());
  if (cells[0] === '') {
    cells.shift();
  }
  if (cells[cells.length - 1] === '') {
    cells.pop();
  }
  return cells;
}

function joinRow(cells) {
  return `| ${cells.join(' | ')} |`;
}

function loadScoreboardTable(scoreboardText, scoreboardPath = resolveScoreboardPath()) {
  const lines = scoreboardText.split('\n');
  const headerIndex = lines.findIndex((line) => line.startsWith('| Execution |'));
  if (headerIndex === -1) {
    throw new Error(`Could not find scoreboard table in ${scoreboardPath}`);
  }
  const {header, separator, rows, endIndex} = parseMarkdownTable(lines, headerIndex);
  const columns = splitMarkdownTableRow(header);
  const objects = rows.map((row) => {
    const values = splitMarkdownTableRow(row);
    return Object.fromEntries(columns.map((column, index) => [column, values[index] || '']));
  });
  return {lines, headerIndex, endIndex, header, separator, columns, objects};
}

function lanePlanPath(projectRoot, execution, lane) {
  const laneMatch = /^Lane\s+(\d+)$/.exec(lane);
  if (!laneMatch) {
    return null;
  }
  return path.join(projectRoot, 'NLSDD', 'executions', execution, `lane-${laneMatch[1]}.md`);
}

function parseLanePlan(text) {
  const worktreeMatch = text.match(/NLSDD worktree:\s*`([^`]+)`/);
  const worktreeRelativePath = worktreeMatch ? worktreeMatch[1] : null;

  const verificationCommands = [];
  const verificationBlock = text.match(
    />\s*Lane-local verification:\n((?:>\s*`[^`]+`\n?)*)/m,
  );
  if (verificationBlock) {
    for (const line of verificationBlock[1].split('\n')) {
      const commandMatch = line.match(/>\s*`([^`]+)`/);
      if (commandMatch) {
        verificationCommands.push(commandMatch[1]);
      }
    }
  }

  const actionableItems = [];
  let currentSection = '';
  let inRefillSection = false;
  let inCurrentStatusSection = false;
  for (const line of text.split('\n')) {
    const headingMatch = line.match(/^##\s+(.*)$/);
    if (headingMatch) {
      currentSection = headingMatch[1];
      inRefillSection = currentSection === 'Refill Order';
      inCurrentStatusSection = currentSection === 'Current Lane Status';
      continue;
    }

    const itemMatch = line.match(/^- \[( |x)\] (.+)$/);
    if (!itemMatch) {
      continue;
    }

    if (inRefillSection || inCurrentStatusSection) {
      continue;
    }

    actionableItems.push({
      checked: itemMatch[1] === 'x',
      text: itemMatch[2],
      section: currentSection || 'Unsectioned',
    });
  }

  return {
    worktreeRelativePath,
    verificationCommands,
    actionableItems,
  };
}

function loadLanePlan(projectRoot, execution, lane) {
  const filePath = lanePlanPath(projectRoot, execution, lane);
  if (!filePath || !fs.existsSync(filePath)) {
    return null;
  }
  const text = fs.readFileSync(filePath, 'utf8');
  const parsed = parseLanePlan(text);
  const worktreePoolRoot = resolveWorktreePoolRoot(projectRoot);
  const worktreePath = !parsed.worktreeRelativePath
    ? null
    : path.isAbsolute(parsed.worktreeRelativePath)
      ? parsed.worktreeRelativePath
      : path.join(worktreePoolRoot, parsed.worktreeRelativePath);
  return {
    filePath,
    text,
    ...parsed,
    worktreePath,
  };
}

function laneStatePath(projectRoot, execution, lane) {
  const laneMatch = /^Lane\s+(\d+)$/.exec(lane);
  if (!laneMatch) {
    return null;
  }
  return path.join(projectRoot, 'NLSDD', 'state', execution, `lane-${laneMatch[1]}.json`);
}

function loadLaneState(projectRoot, execution, lane) {
  const filePath = laneStatePath(projectRoot, execution, lane);
  if (!filePath || !fs.existsSync(filePath)) {
    return null;
  }

  try {
    const parsed = JSON.parse(fs.readFileSync(filePath, 'utf8'));
    return {
      ...parsed,
      filePath,
      execution: parsed.execution || execution,
      lane: parsed.lane || lane,
    };
  } catch {
    return null;
  }
}

function classifyNoise(statusOutput) {
  const lines = statusOutput.split('\n').map((line) => line.trimEnd()).filter(Boolean);
  if (lines.length === 0) {
    return 'none';
  }
  const entries = lines.map((line) => {
    const status = line.slice(0, 2);
    const filePath = line.slice(3);
    const isArtifact = filePath.startsWith('rust/plot-viewer/target/');
    const isUntracked = status === '??';
    return {status, filePath, isArtifact, isUntracked};
  });

  const hasArtifact = entries.some((entry) => entry.isArtifact);
  const hasSource = entries.some((entry) => !entry.isArtifact);
  if (hasArtifact && hasSource) {
    return 'mixed';
  }
  if (!hasArtifact) {
    return 'none';
  }
  return entries.every((entry) => entry.isUntracked)
    ? 'untracked-artifact-noise'
    : 'tracked-artifact-noise';
}

function splitStatusEntries(statusOutput) {
  const lines = statusOutput.split('\n').map((line) => line.trimEnd()).filter(Boolean);
  const sourcePaths = [];
  const artifactPaths = [];

  for (const line of lines) {
    const filePath = line.slice(3);
    if (filePath.startsWith('rust/plot-viewer/target/')) {
      artifactPaths.push(filePath);
    } else {
      sourcePaths.push(filePath);
    }
  }

  return {sourcePaths, artifactPaths};
}

function inspectLaneWorktree(projectRoot, execution, lane) {
  const lanePlan = loadLanePlan(projectRoot, execution, lane);
  if (!lanePlan || !lanePlan.worktreePath || !fs.existsSync(lanePlan.worktreePath)) {
    return null;
  }

  const head = tryRun('git', ['rev-parse', '--short', 'HEAD'], lanePlan.worktreePath) || null;
  const statusOutput = tryRun('git', ['status', '--short'], lanePlan.worktreePath);
  const {sourcePaths, artifactPaths} = splitStatusEntries(statusOutput);

  return {
    head,
    sourcePaths,
    artifactPaths,
    noise: classifyNoise(statusOutput),
  };
}

function detectStaleImplementing(laneState, worktreeInspection) {
  if (!laneState || laneState.phase !== 'implementing' || !worktreeInspection) {
    return null;
  }

  if (worktreeInspection.sourcePaths.length > 0) {
    return null;
  }

  if (!laneState.latestCommit || !worktreeInspection.head) {
    return null;
  }

  if (String(laneState.latestCommit).trim() !== String(worktreeInspection.head).trim()) {
    return null;
  }

  const updatedAt = laneState.updatedAt ? new Date(laneState.updatedAt) : null;
  const staleAfterMs = Number(process.env.NLSDD_STALE_IMPLEMENTING_AFTER_MS || 60_000);
  if (!updatedAt || Number.isNaN(updatedAt.getTime())) {
    return null;
  }
  if (Date.now() - updatedAt.getTime() < staleAfterMs) {
    return null;
  }

  return {
    kind: 'stale-implementing',
    summary: 'lane journal still says implementing, but the worktree is clean at the same HEAD',
  };
}

function formatIsoTimestamp(value) {
  if (!value) {
    return 'n/a';
  }
  const date = typeof value === 'number' ? new Date(value * 1000) : new Date(value);
  if (Number.isNaN(date.getTime())) {
    return 'n/a';
  }
  return date.toISOString().replace('T', ' ').replace(/\.\d+Z$/, 'Z');
}

function refreshProbe(head, statusOutput) {
  const timestamp = formatIsoTimestamp(new Date().toISOString());
  const lineCount = statusOutput.split('\n').filter(Boolean).length;
  const cleanliness = lineCount === 0 ? 'clean' : `${lineCount} changed path(s)`;
  return `${timestamp} · HEAD ${head || 'n/a'} · ${cleanliness}`;
}

function listExecutionLanes(projectRoot, execution) {
  const executionDir = path.join(projectRoot, 'NLSDD', 'executions', execution);
  if (!fs.existsSync(executionDir)) {
    return [];
  }
  return fs
    .readdirSync(executionDir)
    .filter((entry) => /^lane-\d+\.md$/.test(entry))
    .sort((a, b) => a.localeCompare(b, 'en'))
    .map((entry) => {
      const laneNumber = entry.match(/^lane-(\d+)\.md$/)[1];
      return `Lane ${laneNumber}`;
    });
}

function readRecentThreads(projectRoot, limit = 12) {
  const codexStateDbPath = resolveCodexStateDbPath();
  if (!fs.existsSync(codexStateDbPath)) {
    return [];
  }
  const sql =
    'select id, coalesce(agent_nickname, \'\'), coalesce(agent_role, \'\'), ' +
    'coalesce(title, \'\'), updated_at ' +
    `from threads where cwd='${projectRoot.replace(/'/g, "''")}' ` +
    "and coalesce(agent_nickname,'') <> '' order by updated_at desc limit " +
    Number(limit);
  const csv = tryRun('sqlite3', ['-csv', codexStateDbPath, sql]);
  if (!csv) {
    return [];
  }

  return csv
    .split('\n')
    .filter(Boolean)
    .map((line) => {
      const [id, nickname, role, title, updatedAt] = parseCsvLine(line);
      return {
        id,
        nickname,
        role: role || 'n/a',
        title: title || '',
        updatedAtEpoch: updatedAt ? Number(updatedAt) : 0,
        updated: formatIsoTimestamp(updatedAt ? Number(updatedAt) : null),
      };
    });
}

function parseCsvLine(line) {
  const values = [];
  let current = '';
  let inQuotes = false;
  for (let index = 0; index < line.length; index += 1) {
    const char = line[index];
    if (char === '"') {
      if (inQuotes && line[index + 1] === '"') {
        current += '"';
        index += 1;
      } else {
        inQuotes = !inQuotes;
      }
      continue;
    }
    if (char === ',' && !inQuotes) {
      values.push(current);
      current = '';
      continue;
    }
    current += char;
  }
  values.push(current);
  return values;
}

function indexSessionFiles(sessionsRoot = resolveCodexSessionsRoot()) {
  const filePaths = [];
  function walk(currentPath) {
    if (!fs.existsSync(currentPath)) {
      return;
    }
    for (const entry of fs.readdirSync(currentPath, {withFileTypes: true})) {
      const fullPath = path.join(currentPath, entry.name);
      if (entry.isDirectory()) {
        walk(fullPath);
      } else if (entry.isFile() && entry.name.endsWith('.jsonl')) {
        filePaths.push(fullPath);
      }
    }
  }

  walk(sessionsRoot);
  return filePaths;
}

function findSessionFileForThread(threadId, indexedFiles) {
  return indexedFiles.find((filePath) => path.basename(filePath).includes(threadId)) || null;
}

function extractLaneHints(text) {
  const lanes = new Set();
  for (const match of text.matchAll(/lane name:\s*(Lane\s+\d+)/gi)) {
    lanes.add(match[1].replace(/\s+/g, ' ').trim());
  }
  for (const match of text.matchAll(/(?:^|\/)\.worktrees\/lane-(\d+)-[^\s/]+/g)) {
    lanes.add(`Lane ${match[1]}`);
  }
  for (const match of text.matchAll(/\b(Lane\s+\d+)\b/g)) {
    if (/lane name:/i.test(text)) {
      lanes.add(match[1].replace(/\s+/g, ' ').trim());
    }
  }
  return [...lanes];
}

function extractStatusEvent(text, fallbackTimestamp, thread) {
  const statusMatch =
    text.match(/(^|\n)status:\s*(IN_PROGRESS|DONE_WITH_CONCERNS|DONE|BLOCKED|NEEDS_CONTEXT|PASS|FAIL)\b/i) ||
    text.match(/(^|\n)(PASS|FAIL)\b/);
  if (!statusMatch) {
    return null;
  }

  const status = (statusMatch[2] || statusMatch[1] || '').trim().toUpperCase();
  return {
    status,
    timestamp: fallbackTimestamp,
    nickname: thread.nickname,
    role: thread.role,
    context: compactText(text),
  };
}

function compactText(text) {
  return text.replace(/\s+/g, ' ').trim().slice(0, 200);
}

function parseThreadSession(thread, indexedFiles) {
  const sessionFile = findSessionFileForThread(thread.id, indexedFiles);
  const laneHints = new Set();
  let latestStatusEvent = null;
  let correctionCount = 0;

  if (!sessionFile || !fs.existsSync(sessionFile)) {
    return {thread, sessionFile: null, laneHints: [], latestStatusEvent: null, correctionCount};
  }

  const lines = fs.readFileSync(sessionFile, 'utf8').split('\n').filter(Boolean);
  for (const line of lines) {
    let record;
    try {
      record = JSON.parse(line);
    } catch {
      continue;
    }
    if (record.type !== 'event_msg') {
      continue;
    }

    const payload = record.payload || {};
    let text = '';
    if (payload.type === 'agent_message' && payload.message) {
      text = payload.message;
    } else if (payload.type === 'task_complete' && payload.last_agent_message) {
      text = payload.last_agent_message;
    } else {
      continue;
    }

    for (const lane of extractLaneHints(text)) {
      laneHints.add(lane);
    }

    const event = extractStatusEvent(text, record.timestamp, thread);
    if (!event) {
      continue;
    }
    if (event.status === 'FAIL') {
      correctionCount += 1;
    }
    if (!latestStatusEvent || new Date(event.timestamp) >= new Date(latestStatusEvent.timestamp)) {
      latestStatusEvent = event;
    }
  }

  for (const lane of extractLaneHints(thread.title || '')) {
    laneHints.add(lane);
  }

  return {
    thread,
    sessionFile,
    laneHints: [...laneHints],
    latestStatusEvent,
    correctionCount,
  };
}

function deriveEffectivePhase(manualPhase, latestStatusEvent) {
  if (!latestStatusEvent) {
    return manualPhase || 'manual-review-needed';
  }

  const normalizedPhase = (manualPhase || '').toLowerCase();
  const context = (latestStatusEvent.context || '').toLowerCase();
  switch (latestStatusEvent.status) {
    case 'BLOCKED':
    case 'NEEDS_CONTEXT':
      return 'blocked';
    case 'FAIL':
      return 'correction';
    case 'DONE':
    case 'DONE_WITH_CONCERNS':
      return 'spec-review-pending';
    case 'PASS':
      if (context.includes('quality review') || normalizedPhase.includes('quality-review')) {
        return 'refill-ready';
      }
      if (context.includes('spec review') || normalizedPhase.includes('spec-review')) {
        return 'quality-review-pending';
      }
      if (normalizedPhase.includes('correction')) {
        return 'spec-review-pending';
      }
      return 'manual-review-needed';
    case 'IN_PROGRESS':
      return 'implementing';
    default:
      return manualPhase || 'manual-review-needed';
  }
}

function formatLatestEvent(latestStatusEvent) {
  if (!latestStatusEvent) {
    return 'n/a';
  }
  return `${latestStatusEvent.status} · ${latestStatusEvent.nickname || 'n/a'} · ${formatIsoTimestamp(latestStatusEvent.timestamp)}`;
}

function laneStateLatestEventText(laneState) {
  if (!laneState) {
    return 'n/a';
  }
  const result = laneState.lastReviewerResult || laneState.phase || 'n/a';
  return `${result} · journal · ${formatIsoTimestamp(laneState.updatedAt)}`;
}

function computeLaneAutomation(projectRoot, execution, lane, manualPhase) {
  const laneState = loadLaneState(projectRoot, execution, lane);
  if (laneState) {
    return {
      laneThreads: [],
      latestStatusEvent: null,
      correctionCount: Number(laneState.correctionCount || 0),
      effectivePhase: laneState.phase || manualPhase || 'manual-review-needed',
      nextExpectedPhase: laneState.expectedNextPhase || null,
      latestEventText: laneStateLatestEventText(laneState),
      lastActivityText: formatIsoTimestamp(laneState.updatedAt),
      laneState,
    };
  }

  const threads = readRecentThreads(projectRoot, 20);
  const indexedFiles = indexSessionFiles(resolveCodexSessionsRoot());
  const parsedThreads = threads.map((thread) => parseThreadSession(thread, indexedFiles));
  const laneThreads = parsedThreads.filter((entry) => entry.laneHints.includes(lane));
  const latestStatusEvent = laneThreads
    .map((entry) => entry.latestStatusEvent)
    .filter(Boolean)
    .sort((left, right) => new Date(right.timestamp) - new Date(left.timestamp))[0] || null;
  const correctionCount = laneThreads.reduce((sum, entry) => sum + entry.correctionCount, 0);
  const lastActivityEpoch = laneThreads.reduce(
    (max, entry) => Math.max(max, entry.thread.updatedAtEpoch || 0),
    0,
  );

  return {
    laneThreads,
    latestStatusEvent,
    correctionCount,
    effectivePhase: deriveEffectivePhase(manualPhase, latestStatusEvent),
    nextExpectedPhase: null,
    latestEventText: formatLatestEvent(latestStatusEvent),
    lastActivityText: lastActivityEpoch ? formatIsoTimestamp(lastActivityEpoch) : 'n/a',
    laneState: null,
  };
}

function findNextRefillItem(projectRoot, execution, lane) {
  const lanePlan = loadLanePlan(projectRoot, execution, lane);
  if (!lanePlan) {
    return null;
  }
  return lanePlan.actionableItems.find((item) => !item.checked) || null;
}

function phaseForScheduling(row) {
  return (row['Effective phase'] || row.Phase || '').trim();
}

function phaseConsumesThread(phase) {
  return [
    'implementing',
    'spec-review-pending',
    'quality-review-pending',
    'correction',
  ].includes((phase || '').trim());
}

function phaseIsDispatchable(phase) {
  return ['refill-ready', 'queued', 'lane-ready'].includes((phase || '').trim());
}

function computeExecutionSchedule(projectRoot, execution, maxActiveThreads = 4) {
  const scoreboardPath = resolvePreferredScoreboardPath(projectRoot);
  const scoreboardText = fs.readFileSync(scoreboardPath, 'utf8');
  const table = loadScoreboardTable(scoreboardText, scoreboardPath);
  const rows = table.objects.filter((row) => row.Execution === execution);

  const enrichedRows = rows.map((row) => {
    const laneState = loadLaneState(projectRoot, execution, row.Lane);
    const worktreeInspection = inspectLaneWorktree(projectRoot, execution, row.Lane);
    const staleImplementing = detectStaleImplementing(laneState, worktreeInspection);
    const schedulingPhase = staleImplementing
      ? 'stale-implementing'
      : laneState?.phase || phaseForScheduling(row);
    const nextItem = findNextRefillItem(projectRoot, execution, row.Lane);
    return {
      ...row,
      laneState,
      worktreeInspection,
      staleImplementing,
      schedulingPhase,
      nextExpectedPhase: laneState?.expectedNextPhase || null,
      nextItem: nextItem ? nextItem.text : null,
      nextItemSection: nextItem ? nextItem.section : null,
    };
  });

  const activeRows = enrichedRows.filter((row) => phaseConsumesThread(row.schedulingPhase));
  const refillReadyRows = enrichedRows.filter((row) => row.schedulingPhase === 'refill-ready');
  const queuedRows = enrichedRows.filter((row) =>
    ['queued', 'lane-ready'].includes(row.schedulingPhase),
  );
  const blockedRows = enrichedRows.filter((row) => row.schedulingPhase === 'blocked');
  const staleRows = enrichedRows.filter((row) => row.schedulingPhase === 'stale-implementing');

  const availableSlots = Math.max(0, maxActiveThreads - activeRows.length);
  const dispatchSuggestions = [...refillReadyRows, ...queuedRows]
    .filter((row) => row.nextItem)
    .slice(0, availableSlots)
    .map((row, index) => ({
      slot: activeRows.length + index + 1,
      lane: row.Lane,
      phase: row.schedulingPhase,
      nextExpectedPhase: row.nextExpectedPhase,
      currentItem: row['Current item'],
      nextItem: row.nextItem,
      nextItemSection: row.nextItemSection,
    }));

  return {
    execution,
    maxActiveThreads,
    activeRows,
    refillReadyRows,
    queuedRows,
    blockedRows,
    staleRows,
    availableSlots,
    dispatchSuggestions,
  };
}

module.exports = {
  findNearestNlsddRoot,
  resolveProjectRoot,
  resolveWorktreePoolRoot,
  resolveScoreboardPath,
  resolveRuntimeScoreboardPath,
  resolvePreferredScoreboardPath,
  executionInsightsPath,
  resolveCodexStateDbPath,
  resolveCodexSessionsRoot,
  run,
  tryRun,
  loadScoreboardTable,
  joinRow,
  lanePlanPath,
  parseLanePlan,
  loadLanePlan,
  laneStatePath,
  loadLaneState,
  classifyNoise,
  splitStatusEntries,
  inspectLaneWorktree,
  detectStaleImplementing,
  refreshProbe,
  readRecentThreads,
  buildSessionIndex: indexSessionFiles,
  parseThreadSession,
  deriveEffectivePhase,
  formatLatestEvent,
  formatIsoTimestamp,
  listExecutionLanes,
  computeLaneAutomation,
  findNextRefillItem,
  phaseForScheduling,
  phaseConsumesThread,
  phaseIsDispatchable,
  computeExecutionSchedule,
};
