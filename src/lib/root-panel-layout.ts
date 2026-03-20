export interface RootPanelWindowRow {
  label: "W:" | "5H:";
  usageLeft: string;
  resetLabel: string;
  resetTime: string;
  resetPercent: string;
  pacingLabel: string;
  pacingValue: string;
  pacingDescription: string;
}

export interface RootPanelRow {
  profile: string;
  lastUpdate: string;
  weekly: RootPanelWindowRow | null;
  fiveHour: RootPanelWindowRow | null;
}

export interface RootPanelWidths {
  usageLeft: number;
  resetLabel: number;
  resetTime: number;
  resetPercent: number;
  pacingLabel: number;
  pacingValue: number;
  pacingDescription: number;
}

export function computePanelWidths(rows: RootPanelRow[]): RootPanelWidths {
  const width = (values: string[]): number =>
    values.reduce((max, value) => Math.max(max, visibleLength(value)), 0);

  const detailRows = rows.flatMap((row) => [row.weekly, row.fiveHour].filter(Boolean) as RootPanelWindowRow[]);

  return {
    usageLeft: width(detailRows.map((row) => row.usageLeft)),
    resetLabel: width(detailRows.map((row) => row.resetLabel)),
    resetTime: width(detailRows.map((row) => row.resetTime)),
    resetPercent: width(detailRows.map((row) => row.resetPercent)),
    pacingLabel: width(detailRows.map((row) => row.pacingLabel)),
    pacingValue: width(detailRows.map((row) => row.pacingValue)),
    pacingDescription: width(detailRows.map((row) => row.pacingDescription)),
  };
}

export function renderRootDetailPanel(
  rows: RootPanelRow[],
  widths: RootPanelWidths,
): string {
  return rows
    .map((row) => renderProfileBlock(row, widths))
    .join("\n");
}

function renderProfileBlock(row: RootPanelRow, widths: RootPanelWidths): string {
  const detailLines = [row.weekly, row.fiveHour]
    .filter((detail): detail is RootPanelWindowRow => Boolean(detail))
    .map((detail) => renderDetailLine(detail, widths));
  return [`${row.profile} ${row.lastUpdate}`, ...detailLines].join("\n");
}

function renderDetailLine(
  row: RootPanelWindowRow,
  widths: RootPanelWidths,
): string {
  return `    ${padRight(row.label, 3)} ${padRight(row.usageLeft, widths.usageLeft)}  ${padRight(row.resetLabel, widths.resetLabel)} ${padLeft(row.resetTime, widths.resetTime)} ${padLeft(row.resetPercent, widths.resetPercent)} ${padRight(row.pacingLabel, widths.pacingLabel)} ${padLeft(row.pacingValue, widths.pacingValue)} ${padRight(row.pacingDescription, widths.pacingDescription)}`;
}

function padRight(text: string, width: number): string {
  const pad = Math.max(0, width - visibleLength(text));
  return `${text}${" ".repeat(pad)}`;
}

function padLeft(text: string, width: number): string {
  const pad = Math.max(0, width - visibleLength(text));
  return `${" ".repeat(pad)}${text}`;
}

function visibleLength(text: string): number {
  return text.replace(/\u001b\[[0-9;]*m/g, "").length;
}
