#!/usr/bin/env node

const fs = require('node:fs');
const {
  groupTelemetryEventsByMinuteAndLane,
  normalizeTelemetryMinuteOffset,
  resolveProjectRoot,
  telemetrySummaryPath,
} = require('./nlsdd-lib.cjs');
const {loadEnvelopeEvents} = require('./nlsdd-envelope.cjs');

function parseArgs(argv) {
  const args = {};
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value === '--execution') {
      args.execution = argv[index + 1];
      index += 1;
    } else if (value === '--json') {
      args.json = true;
    }
  }
  return args;
}

function timestampToMs(timestamp) {
  const value = new Date(timestamp).getTime();
  return Number.isNaN(value) ? null : value;
}

function formatIsoTimestamp(timestamp) {
  return new Date(timestamp).toISOString();
}

function buildSupportEvent(event) {
  return {
    timestamp: event.timestamp,
    lane: event.lane || null,
    eventType: event.eventType || null,
    command: event.command || null,
    status: event.status || null,
    phaseAfter: event.phaseAfter || null,
    blockedBy: event.blockedBy || null,
    blockKind: event.blockKind || null,
    exitCode: event.exitCode ?? null,
    durationMs: event.durationMs ?? null,
    probeSummary: event.probeSummary || null,
    summary: event.summary || null,
  };
}

function isActivePhase(phase) {
  return [
    'implementing',
    'spec-review-pending',
    'quality-review-pending',
    'correction',
    'blocked',
    'coordinator-commit-pending',
    'ready-to-commit',
  ].includes((phase || '').trim());
}

function isProductivePhase(phase) {
  return (phase || '').trim() === 'implementing';
}

function isFastFailEvent(event) {
  const command = String(event.command || '').toLowerCase();
  const exitCode = Number(event.exitCode);
  if (!Number.isFinite(exitCode) || exitCode === 0) {
    return false;
  }
  return /curl|dns|lookup|resolve|fetch|google\.com/.test(command) || Number(event.durationMs || 0) < 2000;
}

function buildDropSignals(reason) {
  switch (reason) {
    case 'handoff-wait':
      return {
        confidence: 'high',
        missingSignals: ['coordinator commit acknowledgement'],
      };
    case 'dependency-blocked':
      return {
        confidence: 'high',
        missingSignals: ['dependency owner unblock signal'],
      };
    case 'fast-fail':
      return {
        confidence: 'high',
        missingSignals: ['retry evidence'],
      };
    case 'command-blocked-with-probe-evidence':
      return {
        confidence: 'high',
        missingSignals: ['dependency resolution'],
      };
    default:
      return {
        confidence: 'low',
        missingSignals: [
          'command lifecycle events',
          'worker-local probe evidence',
        ],
      };
  }
}

function classifyDropReason(previousEvents, currentEvents) {
  const allEvents = [...previousEvents, ...currentEvents];
  const currentByLane = new Map();
  for (const event of currentEvents) {
    const lane = event.lane || 'global';
    if (!currentByLane.has(lane)) {
      currentByLane.set(lane, []);
    }
    currentByLane.get(lane).push(event);
  }

  const hasReadyToCommit = currentEvents.some(
    (event) => event.eventType === 'ready-to-commit' || event.phaseAfter === 'coordinator-commit-pending',
  );
  if (hasReadyToCommit) {
    return {
      reason: 'handoff-wait',
      supportingEvents: allEvents.filter(
        (event) =>
          event.eventType === 'ready-to-commit' ||
          event.phaseAfter === 'coordinator-commit-pending' ||
          event.phaseAfter === 'ready-to-commit',
      ),
    };
  }

  const fastFailEvent = currentEvents.find((event) => event.eventType === 'command-failed' && isFastFailEvent(event));
  if (fastFailEvent) {
    return {
      reason: 'fast-fail',
      supportingEvents: allEvents.filter(
        (event) =>
          (event.lane || 'global') === (fastFailEvent.lane || 'global') &&
          (event.eventType === 'command-started' || event.eventType === 'command-failed'),
      ),
    };
  }

  for (const [lane, events] of currentByLane.entries()) {
    const probeEvent = events.find((event) => event.eventType === 'command-probe') ||
      previousEvents.find(
        (event) => (event.lane || 'global') === lane && event.eventType === 'command-probe',
      );
    const blockedEvent = events.find((event) => event.eventType === 'command-blocked');
    if (blockedEvent && probeEvent) {
      return {
        reason: 'command-blocked-with-probe-evidence',
        supportingEvents: [
          ...currentEvents.filter(
            (event) =>
              (event.lane || 'global') === lane &&
              (event.eventType === 'command-blocked' || event.eventType === 'command-probe'),
          ),
          ...previousEvents.filter(
            (event) => (event.lane || 'global') === lane && event.eventType === 'command-probe',
          ),
        ],
      };
    }
  }

  const dependencyBlockedEvent = currentEvents.find(
    (event) =>
      event.blockedBy === 'dependency' ||
      event.blockKind === 'dependency',
  );
  if (dependencyBlockedEvent) {
    return {
      reason: 'dependency-blocked',
      supportingEvents: [dependencyBlockedEvent],
    };
  }

  const silentEvent = currentEvents[currentEvents.length - 1] || previousEvents[previousEvents.length - 1] || null;
  return {
    reason: 'unknown-silence',
    supportingEvents: silentEvent ? [silentEvent] : [],
  };
}

function projectMinuteBuckets(events, firstActivityAt) {
  const grouped = groupTelemetryEventsByMinuteAndLane(events, firstActivityAt);
  const lastMinute = Math.max(...Array.from(grouped.keys(), (minute) => minute), 0);
  const laneStates = new Map();
  const minuteBuckets = [];

  for (let minute = 0; minute <= lastMinute; minute += 1) {
    const minuteGroup = grouped.get(minute) || new Map();
    const currentEvents = [];
    for (const laneEvents of minuteGroup.values()) {
      laneEvents.sort((left, right) => timestampToMs(left.timestamp) - timestampToMs(right.timestamp));
      currentEvents.push(...laneEvents);
    }
    currentEvents.sort((left, right) => timestampToMs(left.timestamp) - timestampToMs(right.timestamp));

    for (const event of currentEvents) {
      const lane = event.lane || 'global';
      const previous = laneStates.get(lane) || {active: false, productive: false, phase: null};
      let active = previous.active;
      let productive = previous.productive;
      const phase = event.phaseAfter || previous.phase || null;

      if (phase) {
        if (isActivePhase(phase)) {
          active = true;
        } else {
          active = false;
        }
        productive = isProductivePhase(phase);
      }

      switch (event.eventType) {
        case 'bootstrap-state':
          active = isActivePhase(phase);
          productive = isProductivePhase(phase);
          break;
        case 'state-update':
          active = isActivePhase(phase) || event.blockedBy === 'dependency';
          productive = isProductivePhase(phase);
          break;
        case 'ready-to-commit':
          active = true;
          productive = false;
          break;
        case 'command-started':
          active = true;
          productive = true;
          break;
        case 'command-finished':
          active = previous.active || isActivePhase(phase);
          productive = isProductivePhase(phase);
          break;
        case 'command-failed':
          active = false;
          productive = false;
          break;
        case 'command-blocked':
          active = true;
          productive = false;
          break;
        case 'command-probe':
          active = true;
          productive = false;
          break;
        default:
          break;
      }

      laneStates.set(lane, {
        active,
        productive,
        phase,
        latestEvent: event,
      });
    }

    const snapshot = Array.from(laneStates.values());
    const minuteStartMs = timestampToMs(firstActivityAt) + minute * 60_000;
    minuteBuckets.push({
      minute,
      minuteStartAt: formatIsoTimestamp(new Date(minuteStartMs).toISOString()),
      minuteOffset: minute,
      minuteEndAt: formatIsoTimestamp(
        new Date(minuteStartMs + 60_000).toISOString(),
      ),
      activeWorkers: snapshot.filter((entry) => entry.active).length,
      productiveWorkers: snapshot.filter((entry) => entry.productive).length,
      events: currentEvents.map(buildSupportEvent),
    });
  }

  return minuteBuckets;
}

function buildDropSegments(minuteBuckets) {
  const segments = [];
  for (let index = 1; index < minuteBuckets.length; index += 1) {
    const previousBucket = minuteBuckets[index - 1];
    const currentBucket = minuteBuckets[index];
    for (const metric of ['activeWorkers', 'productiveWorkers']) {
      if (currentBucket[metric] >= previousBucket[metric]) {
        continue;
      }

      const classification = classifyDropReason(previousBucket.events, currentBucket.events);
      const signalInfo = buildDropSignals(classification.reason);
      segments.push({
        fromMinute: previousBucket.minuteStartAt,
        toMinute: currentBucket.minuteStartAt,
        metric,
        reason: classification.reason,
        confidence: signalInfo.confidence,
        supportingEvents: classification.supportingEvents.map(buildSupportEvent),
        missingSignals: signalInfo.missingSignals,
      });
    }
  }
  return segments;
}

function summarizeTelemetry(projectRoot, execution) {
  const events = loadEnvelopeEvents(projectRoot, execution)
    .filter((event) => event.timestamp)
    .sort((left, right) => timestampToMs(left.timestamp) - timestampToMs(right.timestamp));

  const summaryPath = telemetrySummaryPath(projectRoot, execution);
  const summary = {
    execution,
    firstActivityAt: null,
    lastActivityAt: null,
    wallClockDurationMs: 0,
    minuteBuckets: [],
    dropSegments: [],
  };

  if (events.length > 0) {
    const firstActivityAt = events[0].timestamp;
    const lastActivityAt = events[events.length - 1].timestamp;
    const minuteBuckets = projectMinuteBuckets(events, firstActivityAt);
    summary.firstActivityAt = firstActivityAt;
    summary.lastActivityAt = lastActivityAt;
    summary.wallClockDurationMs = Math.max(0, timestampToMs(lastActivityAt) - timestampToMs(firstActivityAt));
    summary.minuteBuckets = minuteBuckets;
    summary.dropSegments = buildDropSegments(minuteBuckets);
  }

  fs.mkdirSync(require('node:path').dirname(summaryPath), {recursive: true});
  fs.writeFileSync(summaryPath, `${JSON.stringify(summary, null, 2)}\n`, 'utf8');
  return summary;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.execution) {
    throw new Error(
      'Usage: node NLSDD/scripts/nlsdd-summarize-telemetry.cjs --execution <id> [--json]',
    );
  }
  const summary = summarizeTelemetry(resolveProjectRoot(), args.execution);
  if (args.json) {
    process.stdout.write(`${JSON.stringify(summary, null, 2)}\n`);
    return;
  }
  process.stdout.write(
    `Telemetry summary: ${summary.execution}\n` +
      `First activity: ${summary.firstActivityAt || 'n/a'}\n` +
      `Last activity: ${summary.lastActivityAt || 'n/a'}\n` +
      `Minute buckets: ${summary.minuteBuckets.length}\n`,
  );
}

if (require.main === module) {
  main();
}

module.exports = {
  parseArgs,
  summarizeTelemetry,
  projectMinuteBuckets,
  buildDropSegments,
  classifyDropReason,
  telemetrySummaryPath,
};
