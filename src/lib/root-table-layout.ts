export interface RootTableWidths {
  profile: number;
  lastUpdate: number;
  status: number;
  bar: number;
  timeToReset: number;
  usageLeft: number;
  drift: number;
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

  const detailHeader = `${" ".repeat(4 + widths.bar + 2)}${padLeft("Time to reset", widths.timeToReset)}  ${padLeft("Usage Left", widths.usageLeft)}  ${padRight("Drift", widths.drift)}`;
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
  return `${padRight(row.windowLabel, 3)} ${padRight(row.bar, widths.bar)}  ${padLeft(row.timeToReset, widths.timeToReset)}  ${padLeft(row.usageLeft, widths.usageLeft)}  ${padRight(row.drift, widths.drift)}${row.bottleneck ? "  <- Bottleneck" : ""}`;
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
