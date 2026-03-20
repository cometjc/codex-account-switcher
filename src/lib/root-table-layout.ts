export interface RootTableWidths {
  profile: number;
  lastUpdate: number;
  status: number;
  bar: number;
  timeToReset: number;
  usageLeft: number;
  driftValue: number;
  driftLabel: number;
}

export interface WindowDetailRow {
  windowLabel: string;
  bar: string;
  timeToReset: string;
  usageLeft: string;
  drift: string;
  bottleneck: boolean;
}

export function renderRootHeaderBlock(widths: RootTableWidths): string {
  const line1 = joinColumns([
    padRight("Profile", widths.profile),
    padCenter("Last", widths.lastUpdate),
    padCenter("Pacing Status", widths.status),
  ]);

  const detailHeader = renderWindowDetailLine(
    {
      windowLabel: "",
      bar: "",
      timeToReset: "Time to reset",
      usageLeft: "Usage Left",
      drift: "Drift",
      bottleneck: false,
    },
    widths,
  );
  const line2 = joinColumns([
    padRight("", widths.profile),
    padCenter("", widths.lastUpdate),
    padRight(detailHeader, widths.status),
  ]);

  return `${line1}\n${line2}`;
}

export function renderWindowDetailLine(
  row: WindowDetailRow,
  widths: RootTableWidths,
): string {
  const driftParts = splitDrift(row.drift);
  return `${padRight(row.windowLabel, 3)} ${padRight(row.bar, widths.bar)}  ${padLeft(row.timeToReset, widths.timeToReset)}  ${padLeft(row.usageLeft, widths.usageLeft)}  ${padLeft(driftParts.value, widths.driftValue)} ${padRight(driftParts.label, widths.driftLabel)}${row.bottleneck ? "  <- Bottleneck" : ""}`;
}

function splitDrift(drift: string): { value: string; label: string } {
  const match = drift.match(/^([+-]?\d+(?:\.\d+)?%)(?:\s+(.+))?$/);
  if (!match) {
    return { value: "", label: drift };
  }

  return {
    value: match[1] ?? "",
    label: match[2] ?? "",
  };
}

function joinColumns(columns: string[]): string {
  return columns.join("  ");
}

function padRight(text: string, width: number): string {
  const pad = Math.max(0, width - visibleLength(text));
  return `${text}${" ".repeat(pad)}`;
}

function padLeft(text: string, width: number): string {
  const pad = Math.max(0, width - visibleLength(text));
  return `${" ".repeat(pad)}${text}`;
}

function padCenter(text: string, width: number): string {
  const pad = Math.max(0, width - visibleLength(text));
  const left = Math.floor(pad / 2);
  const right = pad - left;
  return `${" ".repeat(left)}${text}${" ".repeat(right)}`;
}

function visibleLength(text: string): number {
  return text.replace(/\u001b\[[0-9;]*m/g, "").length;
}
