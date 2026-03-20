export interface RootPanelRow {
  profile: string;
  lastUpdate: string;
  weeklyUsageLeft: string;
  weeklyTimeToReset: string;
  weeklyDelta: string;
  fiveHourUsageLeft: string;
  fiveHourTimeToReset: string;
  fiveHourDelta: string;
}

export interface RootPanelWidths {
  usageLeft: number;
  timeToReset: number;
  delta: number;
}

export function computePanelWidths(rows: RootPanelRow[]): RootPanelWidths {
  const width = (values: string[]): number =>
    values.reduce((max, value) => Math.max(max, value.length), 0);

  return {
    usageLeft: width(["91% left", ...rows.flatMap((row) => [row.weeklyUsageLeft, row.fiveHourUsageLeft])]),
    timeToReset: width(["6.8d", ...rows.flatMap((row) => [row.weeklyTimeToReset, row.fiveHourTimeToReset])]),
    delta: width(["+3.1%", ...rows.flatMap((row) => [row.weeklyDelta, row.fiveHourDelta])]),
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
  const line1 = `${row.profile}  ${row.lastUpdate}`;
  const line2 = renderDetailLine("W:", row.weeklyUsageLeft, row.weeklyTimeToReset, row.weeklyDelta, widths);
  const line3 = renderDetailLine("5H:", row.fiveHourUsageLeft, row.fiveHourTimeToReset, row.fiveHourDelta, widths);
  return `${line1}\n${line2}\n${line3}`;
}

function renderDetailLine(
  label: string,
  usageLeft: string,
  timeToReset: string,
  delta: string,
  widths: RootPanelWidths,
): string {
  return `${padRight(label, 3)} ${padRight(usageLeft, widths.usageLeft)}  ${padLeft(timeToReset, widths.timeToReset)}  ${padLeft(delta, widths.delta)}`;
}

function padRight(text: string, width: number): string {
  const pad = Math.max(0, width - text.length);
  return `${text}${" ".repeat(pad)}`;
}

function padLeft(text: string, width: number): string {
  const pad = Math.max(0, width - text.length);
  return `${" ".repeat(pad)}${text}`;
}
