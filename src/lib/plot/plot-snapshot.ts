import type { UsageResponse } from "../limits";

export interface PlotWindowPoint {
  offsetSeconds: number;
  usedPercent: number;
}

export interface PlotWindowBounds {
  startAt: number | null;
  endAt: number | null;
}

export interface PlotSummaryLabels {
  timeToReset: string;
  usageLeft: string;
  drift: string;
  pacingStatus: string;
}

export interface PlotFiveHourBand {
  available: boolean;
  lowerY: number | null;
  upperY: number | null;
  bandHeight: number | null;
  delta7dPercent: number | null;
  delta5hPercent: number | null;
  reason?: string;
}

export interface PlotProfile {
  id: string;
  name: string;
  isCurrent: boolean;
  usage: UsageResponse | null;
  sevenDayWindow: PlotWindowBounds;
  sevenDayPoints: PlotWindowPoint[];
  fiveHourWindow: PlotWindowBounds;
  fiveHourBand: PlotFiveHourBand;
  summaryLabels: PlotSummaryLabels;
}

export interface PlotSnapshot {
  schemaVersion: 1;
  generatedAt: number;
  currentProfileId: string | null;
  profiles: PlotProfile[];
}

export interface PlotFiveHourBandInput {
  available?: boolean;
  lowerY?: number | null;
  upperY?: number | null;
  bandHeight?: number | null;
  delta7dPercent?: number | null;
  delta5hPercent?: number | null;
  reason?: string;
}

export interface PlotProfileInput {
  id: string;
  name: string;
  isCurrent?: boolean;
  usage: UsageResponse | null;
  sevenDayWindow: PlotWindowBounds;
  sevenDayPoints: PlotWindowPoint[];
  fiveHourWindow: PlotWindowBounds;
  fiveHourBand?: PlotFiveHourBandInput | null;
  summaryLabels?: Partial<PlotSummaryLabels>;
}

export interface PlotSnapshotInput {
  generatedAt?: number;
  profiles: PlotProfileInput[];
}

const DEFAULT_SUMMARY_LABELS: PlotSummaryLabels = {
  timeToReset: "Time to reset",
  usageLeft: "Usage Left",
  drift: "Drift",
  pacingStatus: "Pacing Status",
};

const DEFAULT_UNAVAILABLE_BAND_REASON = "band-not-provided";

export function buildPlotSnapshot(input: PlotSnapshotInput): PlotSnapshot {
  // This builder only normalizes already-prepared plot data.
  // The CLI layer is expected to supply the per-profile curve points and
  // band geometry; we do not re-derive usage math here.
  const profiles = input.profiles.map(normalizePlotProfile);

  return {
    schemaVersion: 1,
    generatedAt: input.generatedAt ?? Math.floor(Date.now() / 1000),
    currentProfileId: profiles.find((profile) => profile.isCurrent)?.id ?? null,
    profiles,
  };
}

export function serializePlotSnapshot(snapshot: PlotSnapshot): string {
  return `${JSON.stringify(normalizeForJson(snapshot), null, 2)}\n`;
}

function normalizePlotProfile(input: PlotProfileInput): PlotProfile {
  const orderedPoints = [...input.sevenDayPoints].sort(
    (left, right) => left.offsetSeconds - right.offsetSeconds,
  );

  return {
    id: input.id,
    name: input.name,
    isCurrent: input.isCurrent ?? false,
    usage: input.usage,
    sevenDayWindow: {
      startAt: input.sevenDayWindow.startAt,
      endAt: input.sevenDayWindow.endAt,
    },
    sevenDayPoints: orderedPoints.map((point) => ({
      offsetSeconds: point.offsetSeconds,
      usedPercent: point.usedPercent,
    })),
    fiveHourWindow: {
      startAt: input.fiveHourWindow.startAt,
      endAt: input.fiveHourWindow.endAt,
    },
    fiveHourBand: normalizeFiveHourBand(input.fiveHourBand),
    summaryLabels: {
      ...DEFAULT_SUMMARY_LABELS,
      ...input.summaryLabels,
    },
  };
}

function normalizeFiveHourBand(input: PlotFiveHourBandInput | null | undefined): PlotFiveHourBand {
  if (!input) {
    return {
      available: false,
      lowerY: null,
      upperY: null,
      bandHeight: null,
      delta7dPercent: null,
      delta5hPercent: null,
      reason: DEFAULT_UNAVAILABLE_BAND_REASON,
    };
  }

  const available = input.available ?? true;

  return {
    available,
    lowerY: input.lowerY ?? null,
    upperY: input.upperY ?? null,
    bandHeight: input.bandHeight ?? null,
    delta7dPercent: input.delta7dPercent ?? null,
    delta5hPercent: input.delta5hPercent ?? null,
    reason: available ? input.reason : input.reason ?? DEFAULT_UNAVAILABLE_BAND_REASON,
  };
}

function normalizeForJson(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map((item) => normalizeForJson(item));
  }

  if (!isPlainObject(value)) {
    return value;
  }

  const result: Record<string, unknown> = {};
  for (const key of Object.keys(value).sort()) {
    result[key] = normalizeForJson(value[key]);
  }
  return result;
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  return Object.getPrototypeOf(value) === Object.prototype;
}
