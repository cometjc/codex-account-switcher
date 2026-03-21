#!/usr/bin/env node

const fs = require('node:fs');
const path = require('node:path');
const {
  INSIGHT_KINDS,
  INSIGHT_STATUSES,
  executionInsightsPath,
  findNextRefillItem,
  joinRow,
  lanePlanPath,
  laneStatePath,
  listExecutionLanes,
  loadExecutionInsights,
  loadLanePlan,
  loadLaneState,
  loadScoreboardTable,
  normalizeInsightLane,
  resolveProjectRoot,
  resolveRuntimeScoreboardPath,
  resolveScoreboardPath,
} = require('./nlsdd-lib.cjs');

const ENVELOPE_EVENT_TYPES = [
  'bootstrap-state',
  'bootstrap-insight',
  'state-update',
  'ready-to-commit',
  'pass',
  'fail',
  'blocked',
  'noop-satisfied',
  'promoted',
  'parked',
  'adopted-insight',
  'resolved-insight',
  'insight-recorded',
];

function normalizeLane(value) {
  if (!value) {
    return null;
  }
  if (value === 'global') {
    return value;
  }
  return `Lane ${value}`.replace(/^Lane\s+Lane\s+/, 'Lane ');
}

function eventLogPath(projectRoot, execution) {
  if (!projectRoot || !execution) {
    return null;
  }
  return path.join(projectRoot, 'NLSDD', 'state', execution, 'events.ndjson');
}

function formatVerification(commands) {
  const list = Array.isArray(commands) ? commands.filter(Boolean) : [];
  if (list.length === 0) {
    return 'n/a';
  }
  return list.map((command) => `\`${command}\``).join('; ');
}

function loadTrackedScoreboardRows(projectRoot, execution) {
  const scoreboardPath = resolveScoreboardPath(projectRoot);
  if (!fs.existsSync(scoreboardPath)) {
    return [];
  }
  const table = loadScoreboardTable(fs.readFileSync(scoreboardPath, 'utf8'), scoreboardPath);
  return table.objects.filter((row) => row.Execution === execution);
}

function buildScoreboardRowMap(projectRoot, execution) {
  return new Map(loadTrackedScoreboardRows(projectRoot, execution).map((row) => [row.Lane, row]));
}

function parseArgs(argv) {
  const args = {verification: [], insights: []};
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value === '--execution') {
      args.execution = argv[index + 1];
      index += 1;
    } else if (value === '--lane') {
      args.lane = argv[index + 1];
      index += 1;
    } else if (value === '--role') {
      args.role = argv[index + 1];
      index += 1;
    } else if (value === '--event-type') {
      args['event-type'] = argv[index + 1];
      index += 1;
    } else if (value === '--phase-before') {
      args['phase-before'] = argv[index + 1];
      index += 1;
    } else if (value === '--phase-after') {
      args['phase-after'] = argv[index + 1];
      index += 1;
    } else if (value === '--current-item') {
      args['current-item'] = argv[index + 1];
      index += 1;
    } else if (value === '--next-refill-target') {
      args['next-refill-target'] = argv[index + 1];
      index += 1;
    } else if (value === '--related-commit') {
      args['related-commit'] = argv[index + 1];
      index += 1;
    } else if (value === '--verification') {
      args.verification.push(argv[index + 1]);
      index += 1;
    } else if (value === '--summary') {
      args.summary = argv[index + 1];
      index += 1;
    } else if (value === '--detail') {
      args.detail = argv[index + 1];
      index += 1;
    } else if (value === '--next-expected-phase') {
      args['next-expected-phase'] = argv[index + 1];
      index += 1;
    } else if (value === '--blocked-by') {
      args['blocked-by'] = argv[index + 1];
      index += 1;
    } else if (value === '--commit-title') {
      args['commit-title'] = argv[index + 1];
      index += 1;
    } else if (value === '--commit-body') {
      args['commit-body'] = argv[index + 1];
      index += 1;
    } else if (value === '--correction-count') {
      args['correction-count'] = argv[index + 1];
      index += 1;
    } else if (value === '--timestamp') {
      args.timestamp = argv[index + 1];
      index += 1;
    } else if (value === '--insight') {
      args.insights.push(JSON.parse(argv[index + 1]));
      index += 1;
    }
  }
  return args;
}

function validateInsight(insight) {
  if (!insight || !INSIGHT_KINDS.includes(insight.kind)) {
    throw new Error(`Unknown insight kind "${insight?.kind || 'n/a'}"`);
  }
  if (insight.status && !INSIGHT_STATUSES.includes(insight.status)) {
    throw new Error(`Unknown insight status "${insight.status}"`);
  }
}

function normalizeEnvelope(projectRoot, rawEnvelope) {
  const execution = rawEnvelope.execution;
  const lane = normalizeLane(rawEnvelope.lane);
  const role = rawEnvelope.role || 'coordinator';
  const eventType = rawEnvelope.eventType || rawEnvelope['event-type'];
  const timestamp = rawEnvelope.timestamp || new Date().toISOString();

  if (!execution || !lane || !role || !eventType || !rawEnvelope.summary) {
    throw new Error(
      'execution, lane, role, eventType, and summary are required for an NLSDD lane handoff envelope',
    );
  }
  if (!ENVELOPE_EVENT_TYPES.includes(eventType)) {
    throw new Error(`Unknown NLSDD envelope eventType "${eventType}"`);
  }

  const normalizedInsights = (rawEnvelope.insights || []).map((insight) => {
    validateInsight(insight);
    return {
      timestamp: insight.timestamp || timestamp,
      execution,
      lane: normalizeInsightLane(insight.lane || lane),
      source: insight.source || role,
      kind: insight.kind,
      status: insight.status || 'open',
      summary: insight.summary,
      detail: insight.detail || null,
      relatedLane: normalizeInsightLane(insight.relatedLane || lane),
      relatedCommit: insight.relatedCommit || rawEnvelope.relatedCommit || null,
      relatedAgent: insight.relatedAgent || null,
      recordedBy: insight.recordedBy || role,
    };
  });

  return {
    execution,
    lane,
    role,
    eventType,
    phaseBefore: rawEnvelope.phaseBefore || rawEnvelope['phase-before'] || null,
    phaseAfter: rawEnvelope.phaseAfter || rawEnvelope['phase-after'] || null,
    currentItem: rawEnvelope.currentItem || rawEnvelope['current-item'] || null,
    nextRefillTarget:
      rawEnvelope.nextRefillTarget || rawEnvelope['next-refill-target'] || null,
    relatedCommit: rawEnvelope.relatedCommit || rawEnvelope['related-commit'] || null,
    verification: Array.isArray(rawEnvelope.verification) ? rawEnvelope.verification : [],
    summary: rawEnvelope.summary,
    detail: rawEnvelope.detail || null,
    nextExpectedPhase:
      rawEnvelope.nextExpectedPhase || rawEnvelope['next-expected-phase'] || null,
    blockedBy: rawEnvelope.blockedBy || rawEnvelope['blocked-by'] || null,
    proposedCommitTitle:
      rawEnvelope.proposedCommitTitle || rawEnvelope['commit-title'] || null,
    proposedCommitBody:
      rawEnvelope.proposedCommitBody || rawEnvelope['commit-body'] || null,
    correctionCount:
      rawEnvelope.correctionCount == null && rawEnvelope['correction-count'] == null
        ? null
        : Number(rawEnvelope.correctionCount || rawEnvelope['correction-count'] || 0),
    timestamp,
    insights: normalizedInsights,
    eventId:
      rawEnvelope.eventId ||
      `${timestamp}-${lane.replace(/\s+/g, '-').toLowerCase()}-${eventType}`,
  };
}

function loadEnvelopeEvents(projectRoot, execution) {
  const filePath = eventLogPath(projectRoot, execution);
  if (!filePath || !fs.existsSync(filePath)) {
    return [];
  }
  return fs
    .readFileSync(filePath, 'utf8')
    .split('\n')
    .filter(Boolean)
    .map((line) => {
      try {
        return JSON.parse(line);
      } catch {
        return null;
      }
    })
    .filter(Boolean);
}

function appendEnvelope(projectRoot, envelope) {
  const filePath = eventLogPath(projectRoot, envelope.execution);
  fs.mkdirSync(path.dirname(filePath), {recursive: true});
  fs.appendFileSync(filePath, `${JSON.stringify(envelope)}\n`, 'utf8');
  return filePath;
}

function buildBootstrapEnvelopes(projectRoot, execution) {
  const rowMap = buildScoreboardRowMap(projectRoot, execution);
  const lanes = new Set([
    ...listExecutionLanes(projectRoot, execution),
    ...rowMap.keys(),
  ]);
  const events = [];

  for (const lane of lanes) {
    const state = loadLaneState(projectRoot, execution, lane);
    const row = rowMap.get(lane);
    const rowCurrentItem = emptyToNull(row?.['Current item']);
    const rowNextRefillTarget = emptyToNull(row?.['Next refill target']);
    if (!state && !row) {
      continue;
    }
    events.push(
      normalizeEnvelope(projectRoot, {
        execution,
        lane,
        role: 'coordinator',
        eventType: 'bootstrap-state',
        phaseAfter: state?.phase || row?.Phase || 'queued',
        currentItem:
          state?.currentItem ??
          rowCurrentItem ??
          findNextRefillItem(projectRoot, execution, lane)?.text ??
          null,
        nextRefillTarget:
          state?.nextRefillTarget ?? rowNextRefillTarget ?? null,
        relatedCommit:
          state?.latestCommit ||
          String(row?.['Item commit'] || '')
            .replaceAll('`', '')
            .trim() ||
          null,
        verification: state?.lastVerification || [],
        summary: `Bootstrapped lane state for ${lane}`,
        detail: state?.note || row?.Notes || null,
        nextExpectedPhase: state?.expectedNextPhase || null,
        blockedBy: state?.blockedBy || row?.['Blocked by'] || null,
        timestamp: state?.updatedAt || new Date().toISOString(),
        'commit-title': state?.proposedCommitTitle || null,
        'commit-body': state?.proposedCommitBody || null,
        'correction-count': state?.correctionCount || 0,
      }),
    );
  }

  for (const insight of loadExecutionInsights(projectRoot, execution)) {
    events.push(
      normalizeEnvelope(projectRoot, {
        execution,
        lane: insight.lane || 'global',
        role: 'coordinator',
        eventType: 'bootstrap-insight',
        summary: insight.summary,
        detail: insight.detail,
        timestamp: insight.timestamp,
        insights: [insight],
      }),
    );
  }

  return events;
}

function ensureExecutionBootstrap(projectRoot, execution) {
  const filePath = eventLogPath(projectRoot, execution);
  if (filePath && fs.existsSync(filePath) && fs.readFileSync(filePath, 'utf8').trim()) {
    return;
  }

  const events = buildBootstrapEnvelopes(projectRoot, execution);
  if (events.length === 0) {
    return;
  }
  fs.mkdirSync(path.dirname(filePath), {recursive: true});
  fs.writeFileSync(filePath, events.map((event) => JSON.stringify(event)).join('\n') + '\n', 'utf8');
}

function emptyToNull(value) {
  if (typeof value !== 'string') {
    return value ?? null;
  }
  const trimmed = value.trim();
  return trimmed === '' ? null : value;
}

function applyEventToLaneState(projectRoot, previous, event, rowMap) {
  const row = rowMap.get(event.lane);
  const fallbackNextItem = findNextRefillItem(projectRoot, event.execution, event.lane);
  const rowCurrentItem = emptyToNull(row?.['Current item']);
  const rowNextRefillTarget = emptyToNull(row?.['Next refill target']);
  const clearsProjectedFields = ['parked', 'noop-satisfied', 'resolved-blocker'].includes(event.eventType);
  const explicitCurrentItem = Object.prototype.hasOwnProperty.call(event, 'currentItem');
  const explicitNextRefillTarget = Object.prototype.hasOwnProperty.call(event, 'nextRefillTarget');
  const explicitNextExpectedPhase = Object.prototype.hasOwnProperty.call(event, 'nextExpectedPhase');
  const nextState = {
    execution: event.execution,
    lane: event.lane,
    phase: event.phaseAfter || previous.phase || row?.Phase || 'queued',
    expectedNextPhase:
      explicitNextExpectedPhase
        ? event.nextExpectedPhase
        : clearsProjectedFields
          ? null
        : previous.expectedNextPhase || null,
    latestCommit: event.relatedCommit || previous.latestCommit || null,
    proposedCommitTitle:
      event.proposedCommitTitle ?? previous.proposedCommitTitle ?? null,
    proposedCommitBody:
      event.proposedCommitBody ?? previous.proposedCommitBody ?? null,
    lastReviewerResult:
      event.eventType === 'bootstrap-state'
        ? previous.lastReviewerResult || null
        : event.eventType,
    lastVerification:
      event.verification && event.verification.length > 0
        ? event.verification
        : previous.lastVerification || [],
    blockedBy: event.blockedBy ?? previous.blockedBy ?? null,
    note: event.detail || event.summary || previous.note || null,
    correctionCount:
      event.correctionCount == null
        ? previous.correctionCount || 0
        : Number(event.correctionCount),
    updatedAt: event.timestamp,
    currentItem:
      explicitCurrentItem
        ? event.currentItem
        : clearsProjectedFields
          ? null
          : previous.currentItem ??
            rowCurrentItem ??
            fallbackNextItem?.text ??
            null,
    nextRefillTarget:
      explicitNextRefillTarget
        ? event.nextRefillTarget
        : clearsProjectedFields
          ? null
          : previous.nextRefillTarget ??
            rowNextRefillTarget ??
            null,
    lastEventType: event.eventType,
    latestSummary: event.summary,
    latestDetail: event.detail || null,
  };

  if (nextState.phase !== 'blocked' && !event.blockedBy) {
    nextState.blockedBy = null;
  }
  return nextState;
}

function replayExecution(projectRoot, execution) {
  ensureExecutionBootstrap(projectRoot, execution);
  const rowMap = buildScoreboardRowMap(projectRoot, execution);
  const states = new Map();
  const insights = [];

  for (const event of loadEnvelopeEvents(projectRoot, execution)) {
    if (event.lane && event.lane !== 'global') {
      const previous = states.get(event.lane) || {};
      states.set(event.lane, applyEventToLaneState(projectRoot, previous, event, rowMap));
    }
    for (const insight of event.insights || []) {
      insights.push(insight);
    }
  }

  return {states, insights};
}

function projectLaneStateFiles(projectRoot, execution, states) {
  for (const [lane, state] of states.entries()) {
    const filePath = laneStatePath(projectRoot, execution, lane);
    fs.mkdirSync(path.dirname(filePath), {recursive: true});
    fs.writeFileSync(
      filePath,
      `${JSON.stringify(
        {
          execution: state.execution,
          lane: state.lane,
          phase: state.phase,
          expectedNextPhase: state.expectedNextPhase,
          latestCommit: state.latestCommit,
          proposedCommitTitle: state.proposedCommitTitle,
          proposedCommitBody: state.proposedCommitBody,
          lastReviewerResult: state.lastReviewerResult,
          lastVerification: state.lastVerification,
          blockedBy: state.blockedBy,
          note: state.note,
          correctionCount: state.correctionCount,
          updatedAt: state.updatedAt,
          currentItem: state.currentItem,
          nextRefillTarget: state.nextRefillTarget,
          lastEventType: state.lastEventType,
          latestSummary: state.latestSummary,
          latestDetail: state.latestDetail,
        },
        null,
        2,
      )}\n`,
      'utf8',
    );
  }
}

function projectExecutionInsights(projectRoot, execution, insights) {
  const filePath = executionInsightsPath(projectRoot, execution);
  fs.mkdirSync(path.dirname(filePath), {recursive: true});
  const content = insights.length === 0 ? '' : `${insights.map((entry) => JSON.stringify(entry)).join('\n')}\n`;
  fs.writeFileSync(filePath, content, 'utf8');
}

function updateScoreboardRows(projectRoot, execution, states) {
  const scoreboardPath = resolveScoreboardPath(projectRoot);
  if (!fs.existsSync(scoreboardPath)) {
    return;
  }
  const scoreboardText = fs.readFileSync(scoreboardPath, 'utf8');
  const table = loadScoreboardTable(scoreboardText, scoreboardPath);

  for (const row of table.objects) {
    if (row.Execution !== execution) {
      continue;
    }
    const state = states.get(row.Lane);
    if (!state) {
      continue;
    }
    row['Current item'] = state.currentItem ?? 'n/a';
    row.Phase = state.phase ?? row.Phase;
    row['Item commit'] = state.latestCommit ? `\`${state.latestCommit}\`` : row['Item commit'];
    row['Last verification'] = formatVerification(state.lastVerification);
    row['Blocked by'] = state.blockedBy ?? 'none';
    row['Next refill target'] = state.nextRefillTarget ?? 'n/a';
    row.Notes = state.note ?? state.latestSummary ?? row.Notes;
  }

  const nextLines = [
    ...table.lines.slice(0, table.headerIndex),
    table.header,
    table.separator,
    ...table.objects.map((row) => joinRow(table.columns.map((column) => String(row[column] || '').replace(/\|/g, '\\|')))),
    ...table.lines.slice(table.endIndex),
  ];
  fs.writeFileSync(scoreboardPath, nextLines.join('\n'), 'utf8');
}

function replaceCurrentLaneStatusSection(text, state) {
  const lines = text.split('\n');
  const sectionIndex = lines.findIndex((line) => line.trim() === '## Current Lane Status');
  if (sectionIndex === -1) {
    return text;
  }
  let endIndex = sectionIndex + 1;
  while (endIndex < lines.length && !lines[endIndex].startsWith('## ')) {
    endIndex += 1;
  }

  const generatedLines = [
    '## Current Lane Status',
    '',
    `- [x] Projected phase: ${state.phase || 'n/a'}`,
    `- [x] Current item: ${state.currentItem || 'n/a'}`,
    `- [x] Latest commit: ${state.latestCommit ? `\`${state.latestCommit}\`` : '`n/a`'}`,
    `- [x] Latest event: ${state.lastEventType || 'n/a'}${state.latestSummary ? ` · ${state.latestSummary}` : ''}`,
    `- [x] Next expected phase: ${state.expectedNextPhase || 'n/a'}`,
    `- [x] Next refill target: ${state.nextRefillTarget || 'n/a'}`,
  ];
  if (state.blockedBy) {
    generatedLines.push(`- [x] Blocked by: ${state.blockedBy}`);
  }
  if (state.note) {
    generatedLines.push(`- [x] Latest note: ${state.note}`);
  }
  generatedLines.push('');

  return [...lines.slice(0, sectionIndex), ...generatedLines, ...lines.slice(endIndex)].join('\n');
}

function projectLanePlanStatus(projectRoot, execution, states) {
  for (const [lane, state] of states.entries()) {
    const filePath = lanePlanPath(projectRoot, execution, lane);
    if (!filePath || !fs.existsSync(filePath)) {
      continue;
    }
    const nextText = replaceCurrentLaneStatusSection(fs.readFileSync(filePath, 'utf8'), state);
    fs.writeFileSync(filePath, nextText, 'utf8');
  }
}

function reduceExecution(projectRoot, execution) {
  const {states, insights} = replayExecution(projectRoot, execution);
  projectLaneStateFiles(projectRoot, execution, states);
  projectExecutionInsights(projectRoot, execution, insights);
  updateScoreboardRows(projectRoot, execution, states);
  projectLanePlanStatus(projectRoot, execution, states);
  delete require.cache[require.resolve('./nlsdd-refresh-scoreboard.cjs')];
  const {updateScoreboard} = require('./nlsdd-refresh-scoreboard.cjs');
  updateScoreboard(projectRoot);
  return {execution, laneCount: states.size, insightCount: insights.length};
}

function prepareExecutionState(projectRoot, execution) {
  ensureExecutionBootstrap(projectRoot, execution);
  return reduceExecution(projectRoot, execution);
}

function recordEnvelope(projectRoot, rawEnvelope) {
  const envelope = normalizeEnvelope(projectRoot, rawEnvelope);
  ensureExecutionBootstrap(projectRoot, envelope.execution);
  const filePath = appendEnvelope(projectRoot, envelope);
  const reduction = reduceExecution(projectRoot, envelope.execution);
  return {filePath, envelope, reduction};
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const result = recordEnvelope(resolveProjectRoot(), args);
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
}

if (require.main === module) {
  main();
}

module.exports = {
  ENVELOPE_EVENT_TYPES,
  eventLogPath,
  normalizeEnvelope,
  loadEnvelopeEvents,
  ensureExecutionBootstrap,
  replayExecution,
  reduceExecution,
  prepareExecutionState,
  recordEnvelope,
  replaceCurrentLaneStatusSection,
};
