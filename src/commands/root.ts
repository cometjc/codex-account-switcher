import confirm from "@inquirer/confirm";
import input from "@inquirer/input";
import { execFileSync } from "node:child_process";
import { BaseCommand } from "../lib/base-command";
import {
  renderRootHeaderBlock,
  renderWindowDetailLine,
  type RootTableWidths,
} from "../lib/root-table-layout";
import { computePanelWidths, renderRootDetailPanel } from "../lib/root-panel-layout";
import { renderSelectionOptionLabel } from "../lib/root-option-layout";
import actionSelect, { Separator } from "../lib/prompts/action-select";
import {
  AccountAlreadyExistsError,
  type AuthSnapshot,
  type SavedProfile,
} from "../lib/accounts/index";
import { type UsageResponse, type UsageWindow, usageLimitService } from "../lib/limits/index";

type ItemKind = "saved" | "unsaved-current";
type ActionKind =
  | "delete"
  | "rename"
  | "mode"
  | "workload"
  | "color"
  | "quit"
  | "update-current"
  | "update-all"
  | "redraw";
type BarStyle = "quota" | "delta";
type RefreshScope = "none" | "current" | "all";
type WorkloadTier = "auto" | "low" | "medium" | "high";
type PromptDensity = "full" | "condensed";

interface LimitAxis {
  startAt: number;
  endAt: number;
}

interface UsageView {
  usage: UsageResponse | null;
  source: "api" | "cache" | "none";
  fetchedAt: number | null;
  stale: boolean;
}

interface MenuItem {
  kind: ItemKind;
  savedName?: string;
  snapshot: AuthSnapshot;
  usage: UsageResponse | null;
  accountId?: string;
  isCurrent: boolean;
  profileName: string;
  paceSortKey: number;
  stableOrder: number;
  usageView: UsageView;
  renderedLine: string;
}

interface RowModel {
  profile: string;
  lastUpdate: string;
  status: string;
  statusValue: number | null;
  scoreLabel: string;
  weeklyBar: string;
  weeklyTimeToReset: string;
  weeklyTimeLeftPercent: string;
  weeklyUsageLeft: string;
  weeklyDrift: string;
  weeklyBottleneck: boolean;
  fiveHourBar: string;
  fiveHourTimeToReset: string;
  fiveHourTimeLeftPercent: string;
  fiveHourUsageLeft: string;
  fiveHourDrift: string;
  fiveHourBottleneck: boolean;
}

interface WindowSummary {
  source: "W" | "5H";
  used: number;
  left: number;
  totalSeconds: number;
  remainingSeconds: number;
  timeUsedPercent: number;
  timeLeftPercent: number;
  drift: number;
}

interface RecommendationSummary {
  windows: WindowSummary[];
  leftText: string;
  statusText: string;
  statusValue: number | null;
  statusSource: "W" | "5H" | "-";
  noActivity: boolean;
  score: number;
  scoreLabel: string;
}

export default class RootCommand extends BaseCommand {
  static description =
    "Interactive Codex auth profile manager (save/use/delete/rename with limits)";

  private ansiEnabled = this.defaultAnsiEnabled();
  private stableOrderByKey = new Map<string, number>();
  private nextStableOrder = 0;

  async run(): Promise<void> {
    await this.runSafe(async () => {
      this.ansiEnabled = true;
      let refreshScope: RefreshScope = "none";
      let refreshTargetAccountId: string | undefined = undefined;
      let barStyle: BarStyle = "delta";
      let workloadTier: WorkloadTier = "auto";
      let bootstrapRefreshCurrent = true;

      while (true) {
        const menu = await this.buildMenu(
          refreshScope,
          refreshTargetAccountId,
          barStyle,
          workloadTier,
          bootstrapRefreshCurrent,
        );
        bootstrapRefreshCurrent = false;
        refreshScope = "none";
        refreshTargetAccountId = undefined;

        const choices = menu.items.map((item) => ({
          value: item,
          name: item.renderedLine,
        }));

        if (!choices.length) {
          this.log("No active auth profile and no saved profiles found.");
          return;
        }

        const result = await this.runActionSelect(
          choices,
          menu.statusLine,
          menu.headerLine,
          menu.panelText,
          menu.elapsedFooter,
          barStyle,
          workloadTier,
        );
        if (!result) return;

        const selected = result.answer;
        const action = result.action;

        if (action === "mode") {
          barStyle = barStyle === "quota" ? "delta" : "quota";
          continue;
        }
        if (action === "quit") {
          return;
        }
        if (action === "workload") {
          workloadTier = this.nextWorkloadTier(workloadTier);
          continue;
        }
        if (action === "color") {
          this.ansiEnabled = !this.ansiEnabled;
          continue;
        }
        if (action === "update-current") {
          refreshScope = "current";
          refreshTargetAccountId = selected.accountId;
          continue;
        }
        if (action === "update-all") {
          refreshScope = "all";
          continue;
        }
        if (action === "redraw") {
          continue;
        }

        if (action === "delete") {
          if (selected.kind !== "saved" || !selected.savedName) continue;
          const ok = await this.safeConfirm(
            `Delete saved profile "${selected.savedName}"?`,
          );
          if (!ok) continue;
          await this.accounts.deleteAccount(selected.savedName);
          this.log(`Deleted "${selected.savedName}".`);
          continue;
        }

        if (action === "rename") {
          if (selected.kind !== "saved" || !selected.savedName) continue;
          const renamed = await this.promptRename(selected.savedName);
          if (renamed) this.log(`Renamed to "${renamed}".`);
          continue;
        }

        if (selected.kind === "unsaved-current") {
          const saved = await this.promptSaveCurrent(selected.snapshot, selected.usage);
          if (saved) this.log(`Saved current profile as "${saved}".`);
          continue;
        }

        if (selected.kind === "saved" && selected.savedName) {
          const activated = await this.accounts.useAccount(selected.savedName);
          this.log(`Switched Codex auth to "${activated}".`);
        }
      }
    });
  }

  private async runActionSelect(
    choices: Array<{ value: MenuItem; name: string }>,
    statusLine: string,
    headerLine: string,
    panelText: string,
    elapsedFooter: string | null,
    barStyle: BarStyle,
    workloadTier: WorkloadTier,
  ): Promise<{ answer: MenuItem; action?: ActionKind } | null> {
    try {
      const pageSize = this.computeSelectPageSize();
      const promptChoices: Array<{ value: MenuItem; name: string } | Separator> = [];
      if (statusLine.trim().length > 0) {
        promptChoices.push(new Separator(` ${statusLine}`));
      }
      if (headerLine.trim().length > 0) {
        promptChoices.push(new Separator(` ${headerLine}`));
      }
      promptChoices.push(...choices);
      if (elapsedFooter) {
        promptChoices.push(new Separator(` ${elapsedFooter}`));
      }

      return (await actionSelect<ActionKind, MenuItem>({
        message: "Select profile",
        helpText: this.buildActionsHelpText(barStyle, workloadTier),
        panelText,
        actions: [
          { value: "delete", name: "Delete", key: "d" },
          { value: "delete", name: "Delete", key: "delete" },
          { value: "rename", name: "Rename", key: "n" },
          { value: "update-current", name: "Update One", key: "u" },
          { value: "update-all", name: "Update All", key: "a" },
          { value: "redraw", name: "Redraw", key: "r" },
          { value: "mode", name: "Bar Style", key: "b" },
          { value: "workload", name: "Workload", key: "w" },
          { value: "color", name: "Color", key: "c" },
          { value: "quit", name: "Quit", key: "q" },
        ],
        choices: promptChoices,
        pageSize,
      })) as { answer: MenuItem; action?: ActionKind };
    } catch (error) {
      if (this.isPromptCancelled(error)) return null;
      throw error;
    }
  }

  private async buildMenu(
    refreshScope: RefreshScope,
    refreshTargetAccountId: string | undefined,
    barStyle: BarStyle,
    workloadTier: WorkloadTier,
    bootstrapRefreshCurrent: boolean,
  ): Promise<{
    items: MenuItem[];
    currentUnsaved: MenuItem | null;
    statusLine: string;
    headerLine: string;
    panelText: string;
    elapsedFooter: string | null;
  }> {
    const savedProfiles = await this.accounts.listSavedProfiles();
    const currentSnapshot = await this.accounts.getCurrentSnapshot();
    const currentAccountId = this.readAccountId(currentSnapshot);
    const hasCurrentSaved = Boolean(
      currentAccountId &&
        savedProfiles.some(
          (saved) => this.readAccountId(saved.snapshot) === currentAccountId,
        ),
    );

    const items: MenuItem[] = [];
    for (const saved of savedProfiles) {
      const savedAccountId = this.readAccountId(saved.snapshot);
      const forceRefresh = this.shouldForceRefresh(
        refreshScope,
        refreshTargetAccountId,
        savedAccountId,
      );
      let usageView = await this.readUsageView(saved.snapshot, forceRefresh);
      if (
        bootstrapRefreshCurrent &&
        refreshScope === "none" &&
        currentAccountId &&
        savedAccountId === currentAccountId &&
        this.isCacheOlderThanSeconds(usageView, 60)
      ) {
        usageView = await this.readUsageView(saved.snapshot, true);
      }
      items.push(this.createSavedItem(saved, usageView, currentAccountId, workloadTier));
    }

    let currentUnsaved: MenuItem | null = null;
    if (!hasCurrentSaved) {
      const unsavedAccountId = this.readAccountId(currentSnapshot);
      const forceRefresh = this.shouldForceRefresh(
        refreshScope,
        refreshTargetAccountId,
        unsavedAccountId,
      );
      let usageView = await this.readUsageView(currentSnapshot, forceRefresh);
      if (
        bootstrapRefreshCurrent &&
        refreshScope === "none" &&
        this.isCacheOlderThanSeconds(usageView, 60)
      ) {
        usageView = await this.readUsageView(currentSnapshot, true);
      }
      currentUnsaved = this.createUnsavedCurrentItem(currentSnapshot, usageView, workloadTier);
      items.push(currentUnsaved);
    }

    items.sort((a, b) => this.compareItems(a, b));
    const axes = this.computeAxes(items);
    const rowModels = items.map((item) => this.buildRowModel(item, axes, barStyle, workloadTier));
    const widths = this.computeColumnWidths(rowModels);
    const headerLine = "";
    const promptDensity = this.pickPromptDensity(
      this.countVisiblePromptDetailLines(rowModels),
      rowModels.length,
      this.currentTerminalRows(),
      barStyle,
    );
    const panelText = this.renderPromptPanelText(rowModels, barStyle, promptDensity);

    const renderedItems = items.map((item, index) => ({
      ...item,
      renderedLine: this.renderSelectionOption(item, rowModels[index]!, barStyle),
    }));

    return {
      items: renderedItems,
      currentUnsaved,
      statusLine: this.renderStatusLine(barStyle, workloadTier),
      headerLine,
      panelText,
      elapsedFooter: this.renderElapsedFooter(barStyle, axes, widths),
    };
  }

  private createSavedItem(
    profile: SavedProfile,
    usageView: UsageView,
    currentAccountId: string | undefined,
    workloadTier: WorkloadTier,
  ): MenuItem {
    const accountId = this.readAccountId(profile.snapshot);
    const isCurrent = Boolean(currentAccountId && accountId === currentAccountId);
    return {
      kind: "saved",
      savedName: profile.name,
      snapshot: profile.snapshot,
      usage: usageView.usage,
      accountId,
      isCurrent,
      profileName: profile.name,
      paceSortKey: this.computePaceSortKey(usageView.usage, isCurrent, workloadTier),
      stableOrder: this.getStableOrder(`saved:${profile.name}`),
      usageView,
      renderedLine: "",
    };
  }

  private createUnsavedCurrentItem(
    snapshot: AuthSnapshot,
    usageView: UsageView,
    workloadTier: WorkloadTier,
  ): MenuItem {
    const base = this.buildDefaultName(usageView.usage, snapshot);
    const accountId = this.readAccountId(snapshot);
    return {
      kind: "unsaved-current",
      snapshot,
      usage: usageView.usage,
      accountId,
      isCurrent: true,
      profileName: `${base} [UNSAVED]`,
      paceSortKey: this.computePaceSortKey(usageView.usage, true, workloadTier),
      stableOrder: this.getStableOrder(`unsaved:${accountId ?? base}`),
      usageView,
      renderedLine: "",
    };
  }

  private buildRowModel(
    item: MenuItem,
    axes: { weekly: LimitAxis | null; fiveHour: LimitAxis | null },
    barStyle: BarStyle,
    workloadTier: WorkloadTier,
  ): RowModel {
    const weekly = this.pickWeeklyWindow(item.usage);
    const fiveHour = this.pickFiveHourWindow(item.usage);
    const summary = this.computeSummary(item.usage, item.isCurrent, workloadTier);
    const weeklySummary = summary.windows.find((window) => window.source === "W");
    const fiveHourSummary = summary.windows.find((window) => window.source === "5H");
    const profileLabel = `${item.isCurrent ? "▶ " : "  "}${item.profileName}`;
    const statusStyled = this.colorizePace(summary.statusText, summary.statusValue);

    return {
      profile: profileLabel,
      lastUpdate: this.formatLastUpdate(item.usageView),
      status: `${statusStyled}  ${summary.scoreLabel}`,
      statusValue: summary.statusValue,
      scoreLabel: summary.scoreLabel,
      weeklyBar: this.renderWindowCell(weekly, axes.weekly, barStyle, summary.noActivity, weeklySummary?.drift ?? null),
      weeklyTimeToReset: this.formatTimeToResetFromSummary(weeklySummary),
      weeklyTimeLeftPercent: this.formatTimeLeftPercentFromSummary(weeklySummary),
      weeklyUsageLeft: this.formatUsageLeftFromSummary(weeklySummary),
      weeklyDrift: this.formatDrift(weeklySummary, summary.noActivity),
      weeklyBottleneck: summary.statusSource === "W",
      fiveHourBar: this.renderWindowCell(fiveHour, axes.fiveHour, barStyle, summary.noActivity, fiveHourSummary?.drift ?? null),
      fiveHourTimeToReset: this.formatTimeToResetFromSummary(fiveHourSummary),
      fiveHourTimeLeftPercent: this.formatTimeLeftPercentFromSummary(fiveHourSummary),
      fiveHourUsageLeft: this.formatUsageLeftFromSummary(fiveHourSummary),
      fiveHourDrift: this.formatDrift(fiveHourSummary, summary.noActivity),
      fiveHourBottleneck: summary.statusSource === "5H",
    };
  }

  private renderWindowCell(
    window: UsageWindow | null,
    axis: LimitAxis | null,
    barStyle: BarStyle,
    noActivity: boolean,
    driftValue: number | null,
  ): string {
    if (!window) {
      if (barStyle === "delta") {
        return `[${this.renderDeltaNaBar("N/A")}]`;
      }
      const centered = this.centerInBar("N/A", 28);
      return `[${this.overlayBarText(this.renderQuotaNaBar(), centered)}]`;
    }

    const bar =
      barStyle === "delta"
        ? this.renderDeltaBar(window, noActivity, driftValue)
        : this.renderQuotaBar(window);
    return `[${bar}]`;
  }

  private renderQuotaBar(window: UsageWindow): string {
    const width = 28;
    const used = this.clampPercent(window.used_percent);
    const usedLen = Math.max(0, Math.min(width, Math.round((used / 100) * width)));
    return `${"█".repeat(usedLen)}${"░".repeat(width - usedLen)}`;
  }

  private renderQuotaNaBar(): string {
    return " ".repeat(28);
  }

  private renderDeltaBar(
    window: UsageWindow,
    noActivity: boolean,
    driftValue: number | null,
  ): string {
    const width = 28;
    if (noActivity) {
      return this.colorizeBarByDrift(this.renderDeltaNaBar("No activity"), -999);
    }

    const totalSeconds = Math.max(1, window.limit_window_seconds || this.readResetSeconds(window));
    const remainingSeconds = this.readEffectiveRemainingSeconds(window);
    const elapsedSeconds = Math.max(0, totalSeconds - remainingSeconds);
    const usedPercent = this.clampPercent(window.used_percent);
    const timeUsedPercent = this.clampPercent((elapsedSeconds / totalSeconds) * 100);
    const deltaPercent = usedPercent - timeUsedPercent;

    const half = Math.floor(width / 2);
    const deltaLen = Math.max(0, Math.min(half, Math.round((Math.abs(deltaPercent) / 100) * half)));
    const cells = Array.from({ length: width }, () => " ");
    cells[half] = "|";

    if (deltaPercent > 0) {
      for (let index = half + 1; index <= Math.min(width - 1, half + deltaLen); index += 1) {
        cells[index] = "█";
      }
    } else if (deltaPercent < 0) {
      for (let index = half - 1; index >= Math.max(0, half - deltaLen); index -= 1) {
        cells[index] = "█";
      }
    }
    return this.colorizeBarByDrift(cells.join(""), driftValue);
  }

  private renderDeltaNaBar(label: string): string {
    const width = 28;
    return this.overlayBarText(" ".repeat(width), this.centerInBar(label, width));
  }

  private overlayLabelOnCells(
    cells: Array<{ char: string; elapsed: boolean }>,
    label: string | undefined,
  ): void {
    if (!label) return;
    const overlay = this.centerInBar(label, cells.length).split("");
    for (let index = 0; index < Math.min(cells.length, overlay.length); index += 1) {
      if (overlay[index] !== " ") {
        cells[index] = { ...cells[index], char: overlay[index]! };
      }
    }
  }

  private renderElapsedStyledCells(cells: Array<{ char: string; elapsed: boolean }>): string {
    if (!this.useColor()) return cells.map((cell) => cell.char).join("");

    const elapsedStyle = "\u001b[103m";
    const reset = "\u001b[49m";
    let result = "";
    let runElapsed: boolean | null = null;
    let run = "";

    const flush = () => {
      if (!run.length || runElapsed === null) return;
      result += runElapsed ? `${elapsedStyle}${run}${reset}` : run;
      run = "";
    };

    for (const cell of cells) {
      if (runElapsed === null) {
        runElapsed = cell.elapsed;
        run = cell.char;
        continue;
      }

      if (runElapsed === cell.elapsed) {
        run += cell.char;
      } else {
        flush();
        runElapsed = cell.elapsed;
        run = cell.char;
      }
    }

    flush();
    return result;
  }

  private computeSummary(
    usage: UsageResponse | null,
    isCurrent: boolean,
    workloadTier: WorkloadTier = "auto",
  ): RecommendationSummary {
    const windows = [this.pickWeeklyWindow(usage), this.pickFiveHourWindow(usage)].filter(
      (window): window is UsageWindow => Boolean(window),
    );

    if (!windows.length) {
      return {
        windows: [],
        leftText: "N/A",
        statusText: "N/A",
        statusValue: null,
        statusSource: "-",
        noActivity: false,
        score: Number.NEGATIVE_INFINITY,
        scoreLabel: "Neutral",
      };
    }

    const scored = windows.map((window): WindowSummary => {
      const source: "W" | "5H" =
        window.limit_window_seconds === 604_800 ? "W" : "5H";
      const used = this.clampPercent(window.used_percent);
      const left = Math.max(0, 100 - used);
      const remainingSeconds = this.readEffectiveRemainingSeconds(window);
      const totalSeconds = Math.max(1, window.limit_window_seconds || remainingSeconds);
      const elapsedSeconds = Math.max(0, totalSeconds - remainingSeconds);
      const timeUsedPercent = this.clampPercent((elapsedSeconds / totalSeconds) * 100);
      const timeLeftPercent = Math.max(0, 100 - timeUsedPercent);
      const delta = used - timeUsedPercent;
      return {
        source,
        used,
        left,
        totalSeconds,
        remainingSeconds,
        timeUsedPercent,
        timeLeftPercent,
        drift: delta,
      };
    });

    const bottleneckLeft = scored.reduce((min, row) => (row.left < min.left ? row : min));
    const noActivity = scored.every((row) => row.used <= 0);
    const worstDelta = scored.reduce((max, row) => (row.drift > max.drift ? row : max));

    const weekly = scored.find((row) => row.source === "W");
    const fiveHour = scored.find((row) => row.source === "5H");
    const weeklyNeed = weekly ? Math.max(0, weekly.timeUsedPercent - weekly.used) / 100 : 0;
    const fiveHourSlack = fiveHour ? Math.max(0, fiveHour.timeUsedPercent - fiveHour.used) / 100 : 0;
    const fiveHourSpikeRisk = fiveHour ? Math.max(0, fiveHour.used - fiveHour.timeUsedPercent) / 100 : 0;
    const switchCost = isCurrent ? 0 : 1;
    const unusedBonus = noActivity ? 0.05 : 0;
    const scoringProfile = this.getWorkloadScoringProfile(workloadTier);
    const score =
      scoringProfile.weeklyNeedWeight * weeklyNeed +
      scoringProfile.fiveHourSlackWeight * fiveHourSlack -
      scoringProfile.fiveHourSpikeRiskWeight * fiveHourSpikeRisk -
      scoringProfile.switchCostWeight * switchCost +
      (noActivity ? scoringProfile.unusedBonus : 0);

    const statusSource = noActivity
      ? (fiveHour ? "5H" : weekly ? "W" : "-")
      : worstDelta.source;
    const statusText = noActivity
      ? `Unused, good [${statusSource}]`
      : `${worstDelta.drift >= 0 ? "+" : ""}${worstDelta.drift.toFixed(1)}% ${worstDelta.drift >= 0 ? "Overuse" : "Under"} [${statusSource}]`;

    return {
      leftText: `${Math.round(bottleneckLeft.left).toString().padStart(3, " ")}%`,
      statusText,
      statusValue: noActivity ? null : worstDelta.drift,
      statusSource,
      noActivity,
      windows: scored,
      score,
      scoreLabel: this.scoreLabel(score),
    };
  }

  private computeColumnWidths(rows: RowModel[]): {
    profile: number;
    lastUpdate: number;
    status: number;
    bar: number;
    timeToReset: number;
    usageLeft: number;
    driftValue: number;
    driftLabel: number;
  } {
    const width = (values: string[]): number =>
      values.reduce((max, value) => Math.max(max, this.visibleLength(value)), 0);

    return {
      profile: width(["Profile", ...rows.map((row) => row.profile)]),
      lastUpdate: width(["Last", ...rows.map((row) => row.lastUpdate)]),
      status: width(["Pacing Status", ...rows.map((row) => row.status)]),
      bar: width(rows.flatMap((row) => [row.weeklyBar, row.fiveHourBar])),
      timeToReset: width(rows.flatMap((row) => [row.weeklyTimeToReset, row.fiveHourTimeToReset])),
      usageLeft: width(rows.flatMap((row) => [row.weeklyUsageLeft, row.fiveHourUsageLeft])),
      driftValue: width(rows.flatMap((row) => [
        this.extractDriftValue(row.weeklyDrift),
        this.extractDriftValue(row.fiveHourDrift),
      ])),
      driftLabel: width(rows.flatMap((row) => [
        this.extractDriftLabel(row.weeklyDrift),
        this.extractDriftLabel(row.fiveHourDrift),
      ])),
    };
  }

  private renderHeaderLine(widths: RootTableWidths): string {
    return renderRootHeaderBlock(widths);
  }

  private renderRowLine(
    row: RowModel,
    widths: RootTableWidths,
  ): string {
    const line1 = this.joinColumns([
      this.padRight(row.profile, widths.profile),
      this.padCenter(row.lastUpdate, widths.lastUpdate),
      this.padCenter(row.status, widths.status),
    ]);

    const line2 = this.joinColumns([
      this.padRight("", widths.profile),
      this.padCenter("", widths.lastUpdate),
      this.padRight(
        renderWindowDetailLine(
          {
            windowLabel: "W:",
            bar: row.weeklyBar,
            timeToReset: row.weeklyTimeToReset,
            usageLeft: row.weeklyUsageLeft,
            drift: row.weeklyDrift,
            bottleneck: row.weeklyBottleneck,
          },
          widths,
        ),
        widths.status,
      ),
    ]);

    const line3 = this.joinColumns([
      this.padRight("", widths.profile),
      this.padCenter("", widths.lastUpdate),
      this.padRight(
        renderWindowDetailLine(
          {
            windowLabel: "5H:",
            bar: row.fiveHourBar,
            timeToReset: row.fiveHourTimeToReset,
            usageLeft: row.fiveHourUsageLeft,
            drift: row.fiveHourDrift,
            bottleneck: row.fiveHourBottleneck,
          },
          widths,
        ),
        widths.status,
      ),
    ]);

    return `${line1}\n${line2}\n${line3}`;
  }

  private renderRowLineForWidth(
    item: Pick<MenuItem, "isCurrent" | "profileName">,
    row: RowModel,
    widths: RootTableWidths,
    terminalColumns: number,
  ): string {
    return this.renderRowLine(row, widths);
  }

  private renderElapsedFooter(
    barStyle: BarStyle,
    axes: { weekly: LimitAxis | null; fiveHour: LimitAxis | null },
    widths: {
      profile: number;
      lastUpdate: number;
      status: number;
      bar: number;
      timeToReset: number;
      usageLeft: number;
      driftValue: number;
      driftLabel: number;
    },
  ): string | null {
    return null;
  }

  private renderStatusLine(barStyle: BarStyle, workloadTier: WorkloadTier): string {
    return "";
  }

  private renderPromptPanelText(
    rows: RowModel[],
    barStyle: BarStyle,
    density: PromptDensity = "full",
  ): string {
    if (density === "condensed") {
      return barStyle === "quota"
        ? rows.map((row) => this.renderCondensedQuotaPromptBlock(row)).join("\n")
        : rows.map((row) => this.renderCondensedDeltaPromptBlock(row)).join("\n");
    }

    if (barStyle === "quota") {
      const widths = this.computeColumnWidths(rows);
      return rows.map((row) => this.renderQuotaPromptBlock(row, widths)).join("\n");
    }

    const panelRows = rows.map((row) => ({
      profile: row.profile,
      lastUpdate: this.formatPanelLastUpdate(row.lastUpdate),
      weekly: row.weeklyTimeToReset
        ? {
            label: "W:" as const,
            usageLeft: this.formatPanelUsage(row.weeklyUsageLeft),
            ...this.formatPanelTimeToReset(row.weeklyTimeToReset, row.weeklyTimeLeftPercent),
            ...this.formatPanelPacing(row.weeklyDrift, row.weeklyBottleneck),
          }
        : null,
      fiveHour: row.fiveHourTimeToReset
        ? {
            label: "5H:" as const,
            usageLeft: this.formatPanelUsage(row.fiveHourUsageLeft),
            ...this.formatPanelTimeToReset(row.fiveHourTimeToReset, row.fiveHourTimeLeftPercent),
            ...this.formatPanelPacing(row.fiveHourDrift, row.fiveHourBottleneck),
          }
        : null,
    }));
    return renderRootDetailPanel(panelRows, computePanelWidths(panelRows));
  }

  private pickPromptDensity(
    visibleDetailLines: number,
    profileCount: number,
    terminalRows: number,
    barStyle: BarStyle,
  ): PromptDensity {
    const reservedRows = barStyle === "delta" ? 8 : 7;
    const fullPanelLines = profileCount + visibleDetailLines;
    return fullPanelLines + reservedRows >= terminalRows
      ? "condensed"
      : "full";
  }

  private countVisiblePromptDetailLines(rows: RowModel[]): number {
    return rows.reduce((count, row) => {
      const fiveHourVisible = row.fiveHourTimeToReset ? 1 : 0;
      return count + 1 + fiveHourVisible;
    }, 0);
  }

  private renderQuotaPromptBlock(
    row: RowModel,
    widths: RootTableWidths,
  ): string {
    const header = `${row.profile} ${this.formatPanelLastUpdate(row.lastUpdate)}`;
    const weeklyLine = this.renderQuotaPromptLine(
      "W:",
      row.weeklyBar,
      row.weeklyTimeToReset,
      row.weeklyUsageLeft,
      widths,
    );
    const fiveHourLine = this.renderQuotaPromptLine(
      "5H:",
      row.fiveHourBar,
      row.fiveHourTimeToReset,
      row.fiveHourUsageLeft,
      widths,
    );
    return `${header}\n${weeklyLine}\n${fiveHourLine}`;
  }

  private renderQuotaPromptLine(
    windowLabel: "W:" | "5H:",
    bar: string,
    timeToReset: string,
    usageLeft: string,
    widths: RootTableWidths,
  ): string {
    return `    ${this.padRight(windowLabel, 3)} ${this.padRight(bar, widths.bar)}  ${this.padLeft(timeToReset, widths.timeToReset)}  ${this.padLeft(usageLeft, widths.usageLeft)}`;
  }

  private renderCondensedDeltaPromptBlock(row: RowModel): string {
    const header = `${row.profile} ${this.formatPanelLastUpdate(row.lastUpdate)}`;
    const detailParts = [
      this.renderCondensedDeltaWindow(
        "W:",
        row.weeklyUsageLeft,
        row.weeklyTimeToReset,
        row.weeklyTimeLeftPercent,
        row.weeklyDrift,
        row.weeklyBottleneck,
      ),
      row.fiveHourTimeToReset
        ? this.renderCondensedDeltaWindow(
            "5H:",
            row.fiveHourUsageLeft,
            row.fiveHourTimeToReset,
            row.fiveHourTimeLeftPercent,
            row.fiveHourDrift,
            row.fiveHourBottleneck,
          )
        : null,
    ].filter((part): part is string => Boolean(part));
    return `${header}\n    ${detailParts.join("  ·  ")}`;
  }

  private renderCondensedDeltaWindow(
    label: "W:" | "5H:",
    usageLeft: string,
    timeToReset: string,
    timeLeftPercent: string,
    drift: string,
    isAdopted: boolean,
  ): string {
    const usage = this.formatPanelUsage(usageLeft);
    const reset = this.formatPanelTimeToReset(timeToReset, timeLeftPercent);
    const pacing = this.formatPanelPacing(drift, isAdopted);
    return `${label} ${usage} ${reset.resetLabel} ${reset.resetTime} ${reset.resetPercent} ${pacing.pacingLabel} ${pacing.pacingValue}${pacing.pacingDescription ? ` ${pacing.pacingDescription}` : ""}`;
  }

  private renderCondensedQuotaPromptBlock(row: RowModel): string {
    const header = `${row.profile} ${this.formatPanelLastUpdate(row.lastUpdate)}`;
    const detailParts = [
      this.renderCondensedQuotaWindow("W:", row.weeklyBar, row.weeklyTimeToReset, row.weeklyUsageLeft),
      row.fiveHourTimeToReset
        ? this.renderCondensedQuotaWindow(
            "5H:",
            row.fiveHourBar,
            row.fiveHourTimeToReset,
            row.fiveHourUsageLeft,
          )
        : null,
    ].filter((part): part is string => Boolean(part));
    return `${header}\n    ${detailParts.join("  ·  ")}`;
  }

  private renderCondensedQuotaWindow(
    label: "W:" | "5H:",
    bar: string,
    timeToReset: string,
    usageLeft: string,
  ): string {
    return `${label} ${bar} ${timeToReset} ${usageLeft}`;
  }

  private renderSelectionOption(
    item: Pick<MenuItem, "isCurrent" | "profileName">,
    row: RowModel,
    barStyle: BarStyle,
  ): string {
    return renderSelectionOptionLabel({
      indicator: item.isCurrent ? "▶" : " ",
      profile: item.profileName,
      delta: barStyle === "delta" ? this.optionDeltaValue(row) : "",
    });
  }

  private optionDeltaValue(row: RowModel): string {
    if (row.fiveHourBottleneck) return this.shortDeltaValue(row.fiveHourDrift);
    if (row.weeklyBottleneck) return this.shortDeltaValue(row.weeklyDrift);
    return this.shortDeltaValue(row.fiveHourDrift !== "N/A" ? row.fiveHourDrift : row.weeklyDrift);
  }

  private shortDeltaValue(value: string): string {
    const match = value.match(/[+-]?\d+(?:\.\d+)?%/);
    if (match) return match[0];
    if (value.startsWith("Unused")) return "Unused";
    return value;
  }

  private extractDriftValue(value: string): string {
    const match = value.match(/^([+-]?\d+(?:\.\d+)?%)/);
    return match?.[1] ?? "";
  }

  private extractDriftLabel(value: string): string {
    const match = value.match(/^[+-]?\d+(?:\.\d+)?%\s+(.+)$/);
    return match?.[1] ?? value;
  }

  private currentTerminalColumns(): number {
    const columnCandidates: number[] = [];
    const pushColumns = (value: number | undefined) => {
      if (typeof value !== "number" || !Number.isFinite(value)) return;
      const columns = Math.floor(value);
      if (columns > 0) columnCandidates.push(columns);
    };

    pushColumns(process.stdout.columns);
    pushColumns(process.stderr.columns);

    const envColumns = Number(process.env.COLUMNS);
    pushColumns(envColumns);

    return columnCandidates.length ? Math.max(...columnCandidates) : 80;
  }

  private currentTerminalRows(): number {
    const rowCandidates: number[] = [];
    const pushRows = (value: number | undefined) => {
      if (typeof value !== "number" || !Number.isFinite(value)) return;
      const rows = Math.floor(value);
      if (rows > 0) rowCandidates.push(rows);
    };

    pushRows(process.stdout.rows);
    pushRows(process.stderr.rows);

    if (typeof process.stdout.getWindowSize === "function") {
      const [, rows] = process.stdout.getWindowSize();
      pushRows(rows);
    }
    if (typeof process.stderr.getWindowSize === "function") {
      const [, rows] = process.stderr.getWindowSize();
      pushRows(rows);
    }

    const envRows = Number(process.env.LINES);
    pushRows(envRows);
    pushRows(this.readTputRows());

    return rowCandidates.length ? Math.max(...rowCandidates) : 24;
  }

  private buildActionsHelpText(barStyle: BarStyle, workloadTier: WorkloadTier = "auto"): string {
    const barStyleValue = barStyle === "delta" ? "Delta" : "Quota";
    const workloadValue = this.formatWorkloadTier(workloadTier);
    const colorValue = this.ansiEnabled ? "On" : "Off";
    const actionStyle = "30;106";
    const buttons = [
      this.renderActionButton("[D]elete", actionStyle),
      this.renderActionButton("Re[n]ame", actionStyle),
      this.renderActionButton("[U]pdate one", actionStyle),
      this.renderActionButton("Update [A]ll", actionStyle),
      this.renderActionButton("[R]edraw", actionStyle),
      this.renderActionButton(`[B]ar Style: ${barStyleValue}`, actionStyle),
      this.renderActionButton(`[W]orkload: ${workloadValue}`, actionStyle),
      this.renderActionButton(`[C]olor: ${colorValue}`, actionStyle),
      this.renderActionButton(`[Q]uit`, actionStyle),
    ];
    return buttons.join("  ");
  }

  private renderActionButton(label: string, styleCode: string): string {
    if (!this.useColor()) return label;
    return `\u001b[${styleCode}m ${label} \u001b[0m`;
  }

  private colorizePace(text: string, pace: number | null): string {
    if (!this.useColor()) return text;
    if (pace === null) return `\u001b[2m${text}\u001b[0m`;

    const style = this.pickPaceStyle(pace);
    if (!style) return text;
    return `${style}${text}\u001b[0m`;
  }

  private pickPaceStyle(pace: number): string | null {
    if (pace >= 20) return "\u001b[97;48;5;52m";
    if (pace >= 5) return "\u001b[97;48;5;88m";
    if (pace > -5) return "\u001b[97;48;5;238m";
    if (pace > -20) return "\u001b[97;48;5;28m";
    return "\u001b[97;48;5;22m";
  }

  private formatLastUpdate(view: UsageView): string {
    if (view.fetchedAt === null) return "N/A";

    const ageSeconds = Math.max(0, this.nowSeconds() - view.fetchedAt);
    return this.formatCompactTime(ageSeconds);
  }

  private joinColumns(columns: string[]): string {
    return columns.join("  ");
  }

  private padRight(text: string, width: number): string {
    const pad = Math.max(0, width - this.visibleLength(text));
    return `${text}${" ".repeat(pad)}`;
  }

  private padLeft(text: string, width: number): string {
    const pad = Math.max(0, width - this.visibleLength(text));
    return `${" ".repeat(pad)}${text}`;
  }

  private padCenter(text: string, width: number): string {
    const pad = Math.max(0, width - this.visibleLength(text));
    const left = Math.floor(pad / 2);
    const right = pad - left;
    return `${" ".repeat(left)}${text}${" ".repeat(right)}`;
  }

  private centerInBar(text: string, width: number): string {
    const clean = text.trim();
    const pad = Math.max(0, width - clean.length);
    const left = Math.floor(pad / 2);
    const right = pad - left;
    return `${" ".repeat(left)}${clean}${" ".repeat(right)}`;
  }

  private overlayBarText(bar: string, overlay: string): string {
    const chars = bar.split("");
    const textChars = overlay.split("");
    for (let index = 0; index < Math.min(chars.length, textChars.length); index += 1) {
      if (textChars[index] !== " ") {
        chars[index] = textChars[index]!;
      }
    }
    return chars.join("");
  }

  private visibleLength(text: string): number {
    return text.replace(/\u001b\[[0-9;]*m/g, "").length;
  }

  private padVisibleRight(text: string, width: number): string {
    const pad = Math.max(0, width - this.visibleLength(text));
    return `${text}${" ".repeat(pad)}`;
  }

  private async readUsageView(
    snapshot: AuthSnapshot,
    forceRefresh: boolean,
  ): Promise<UsageView> {
    return usageLimitService.readUsage(
      this.readAccountId(snapshot),
      this.readAccessToken(snapshot),
      { forceRefresh, cacheOnly: !forceRefresh },
    );
  }

  private computeAxes(items: MenuItem[]): { weekly: LimitAxis | null; fiveHour: LimitAxis | null } {
    return {
      weekly: this.computeAxis(items, "weekly"),
      fiveHour: this.computeAxis(items, "five-hour"),
    };
  }

  private computeAxis(items: MenuItem[], type: "weekly" | "five-hour"): LimitAxis | null {
    const windows = items
      .map((item) => (type === "weekly" ? this.pickWeeklyWindow(item.usage) : this.pickFiveHourWindow(item.usage)))
      .filter((window): window is UsageWindow => Boolean(window));

    if (!windows.length) return null;
    const starts = windows.map((window) => this.windowStartAt(window));
    const ends = windows.map((window) => window.reset_at);
    return { startAt: Math.min(...starts), endAt: Math.max(...ends) };
  }

  private pickFiveHourWindow(usage: UsageResponse | null): UsageWindow | null {
    if (!usage?.rate_limit) return null;
    const { primary_window: primary, secondary_window: secondary } = usage.rate_limit;
    if (primary?.limit_window_seconds === 18_000) return primary;
    if (secondary?.limit_window_seconds === 18_000) return secondary;
    return null;
  }

  private pickWeeklyWindow(usage: UsageResponse | null): UsageWindow | null {
    if (!usage?.rate_limit) return null;
    const { primary_window: primary, secondary_window: secondary } = usage.rate_limit;
    if (secondary?.limit_window_seconds === 604_800) return secondary;
    if (primary?.limit_window_seconds === 604_800) return primary;
    return null;
  }

  private computeScore(usage: UsageResponse | null): number {
    const windows = [this.pickFiveHourWindow(usage), this.pickWeeklyWindow(usage)].filter(
      (window): window is UsageWindow => Boolean(window),
    );
    if (!windows.length) return -1;

    const values = windows.map((window) => {
      const left = Math.max(0, 100 - this.clampPercent(window.used_percent));
      const seconds = this.readResetSeconds(window);
      if (seconds <= 0) return 0;
      return left / seconds;
    });
    return Math.min(...values);
  }

  private compareItems(left: MenuItem, right: MenuItem): number {
    if (left.paceSortKey !== right.paceSortKey) return right.paceSortKey - left.paceSortKey;
    return left.stableOrder - right.stableOrder;
  }

  private computePaceSortKey(
    usage: UsageResponse | null,
    isCurrent: boolean,
    workloadTier: WorkloadTier,
  ): number {
    const summary = this.computeSummary(usage, isCurrent, workloadTier);
    return summary.score;
  }

  private nextWorkloadTier(current: WorkloadTier): WorkloadTier {
    switch (current) {
      case "auto":
        return "low";
      case "low":
        return "medium";
      case "medium":
        return "high";
      case "high":
        return "auto";
    }
  }

  private formatWorkloadTier(tier: WorkloadTier): string {
    switch (tier) {
      case "auto":
        return "Auto";
      case "low":
        return "Low";
      case "medium":
        return "Medium";
      case "high":
        return "High";
    }
  }

  private getWorkloadScoringProfile(tier: WorkloadTier): {
    weeklyNeedWeight: number;
    fiveHourSlackWeight: number;
    fiveHourSpikeRiskWeight: number;
    switchCostWeight: number;
    unusedBonus: number;
  } {
    switch (tier) {
      case "low":
        return {
          weeklyNeedWeight: 0.35,
          fiveHourSlackWeight: 0.25,
          fiveHourSpikeRiskWeight: 0.35,
          switchCostWeight: 0.08,
          unusedBonus: 0.02,
        };
      case "medium":
        return {
          weeklyNeedWeight: 0.55,
          fiveHourSlackWeight: 0.2,
          fiveHourSpikeRiskWeight: 0.2,
          switchCostWeight: 0.05,
          unusedBonus: 0.05,
        };
      case "high":
        return {
          weeklyNeedWeight: 0.75,
          fiveHourSlackWeight: 0.1,
          fiveHourSpikeRiskWeight: 0.1,
          switchCostWeight: 0.02,
          unusedBonus: 0.02,
        };
      case "auto":
        return {
          weeklyNeedWeight: 0.55,
          fiveHourSlackWeight: 0.2,
          fiveHourSpikeRiskWeight: 0.2,
          switchCostWeight: 0.05,
          unusedBonus: 0.05,
        };
    }
  }

  private getStableOrder(key: string): number {
    const existing = this.stableOrderByKey.get(key);
    if (existing !== undefined) return existing;
    const assigned = this.nextStableOrder;
    this.nextStableOrder += 1;
    this.stableOrderByKey.set(key, assigned);
    return assigned;
  }

  private shouldForceRefresh(
    scope: RefreshScope,
    refreshTargetAccountId: string | undefined,
    targetAccountId: string | undefined,
  ): boolean {
    if (scope === "all") return true;
    if (scope !== "current") return false;
    return Boolean(
      refreshTargetAccountId &&
        targetAccountId &&
        refreshTargetAccountId === targetAccountId,
    );
  }

  private isCacheOlderThanSeconds(usageView: UsageView, ageSeconds: number): boolean {
    if (usageView.fetchedAt === null) return false;
    if (usageView.source !== "cache") return false;
    return this.nowSeconds() - usageView.fetchedAt > ageSeconds;
  }

  private readResetSeconds(window: UsageWindow): number {
    return Math.max(0, window.reset_at - this.nowSeconds());
  }

  private clampPercent(value: number): number {
    return Math.max(0, Math.min(100, value));
  }

  private windowStartAt(window: UsageWindow): number {
    const windowSeconds = Math.max(1, window.limit_window_seconds || this.readResetSeconds(window));
    return window.reset_at - windowSeconds;
  }

  private formatTimeToResetFromSummary(window: WindowSummary | undefined): string {
    if (!window) return "";
    return this.formatCompactTime(window.remainingSeconds);
  }

  private formatTimeLeftPercentFromSummary(window: WindowSummary | undefined): string {
    if (!window) return "";
    return `${Math.round(window.timeLeftPercent)}%`;
  }

  private formatUsageLeftFromSummary(window: WindowSummary | undefined): string {
    if (!window) return "";
    return `${Math.round(window.left)}% left`;
  }

  private formatDrift(window: WindowSummary | undefined, noActivity: boolean): string {
    if (!window) return "N/A";
    if (noActivity) return "Unused, good";
    return `${window.drift >= 0 ? "+" : ""}${window.drift.toFixed(1)}% ${window.drift >= 0 ? "Overuse" : "Under"}`;
  }

  private formatPanelUsage(value: string): string {
    const match = value.match(/^(\d+)% left$/);
    if (!match) return value;
    return `${match[1]!.padStart(3, " ")}% left`;
  }

  private formatPanelTimeToReset(timeToReset: string, timeLeftPercent: string): {
    resetLabel: string;
    resetTime: string;
    resetPercent: string;
  } {
    return {
      resetLabel: "reset",
      resetTime: timeToReset,
      resetPercent: `(${timeLeftPercent})`,
    };
  }

  private formatPanelPacing(value: string, isAdopted: boolean): {
    pacingLabel: string;
    pacingValue: string;
    pacingDescription: string;
  } {
    const match = value.match(/^([+-]?\d+(?:\.\d+)?%)\s+(.+)$/);
    if (match) {
      return {
        pacingLabel: "Pacing",
        pacingValue: this.colorizePanelPacingField(match[1] ?? "", value, isAdopted),
        pacingDescription: this.colorizePanelPacingField(match[2] ?? "", value, isAdopted),
      };
    }

    return {
      pacingLabel: "Pacing",
      pacingValue: value,
      pacingDescription: "",
    };
  }

  private colorizePanelPacingField(
    field: string,
    rawValue: string,
    isAdopted: boolean,
  ): string {
    if (!this.useColor() || !isAdopted) return field;

    const pace = this.parsePaceDelta(rawValue);
    if (pace === null) return field;
    const style = this.pickPaceStyle(pace);
    if (!style) return field;
    return `${style}${field}\u001b[0m`;
  }

  private formatPanelLastUpdate(value: string): string {
    const label = `last update: ${value} ago`;
    if (!this.useColor()) return label;
    return `\u001b[90m${label}\u001b[0m`;
  }

  private parsePaceDelta(value: string): number | null {
    const match = value.match(/[+-]?\d+(?:\.\d+)?%/);
    if (!match) return null;
    return Number.parseFloat(match[0].replace("%", ""));
  }

  private scoreLabel(score: number): string {
    if (!Number.isFinite(score)) return "Neutral";
    if (score >= 0.45) return "Strong";
    if (score >= 0.25) return "Good";
    if (score >= 0.1) return "Neutral";
    if (score >= -0.05) return "Caution";
    return "Risky";
  }

  private readEffectiveRemainingSeconds(window: UsageWindow): number {
    const used = this.clampPercent(window.used_percent);
    const fullWindow = Math.max(0, window.limit_window_seconds || 0);
    if (used <= 0 && fullWindow > 0) return fullWindow;
    return this.readResetSeconds(window);
  }

  private formatCompactTime(totalSecondsInput: number): string {
    const totalSeconds = Math.max(0, Math.floor(totalSecondsInput));
    const dayFloat = totalSeconds / 86_400;
    const hourFloat = totalSeconds / 3_600;
    const minuteFloat = totalSeconds / 60;
    const seconds = totalSeconds % 60;

    if (totalSeconds >= 86_400) return `${dayFloat.toFixed(1)}d`;
    if (totalSeconds >= 3_600) return `${hourFloat.toFixed(1)}h`;
    if (totalSeconds >= 60) return `${minuteFloat.toFixed(1)}m`;
    return `${seconds}s`;
  }

  private useColor(): boolean {
    return this.ansiEnabled;
  }

  private colorizeBarByDrift(bar: string, drift: number | null): string {
    if (!this.useColor() || drift === null) return bar;
    const style = this.pickPaceStyle(drift);
    if (!style) return bar;
    return `${style}${bar}\u001b[0m`;
  }

  private colorizeRecommendationProfile(text: string, score: number): string {
    if (!this.useColor()) return text;
    const style =
      score >= 0.45
        ? "\u001b[42m"
        : score >= 0.25
          ? "\u001b[46m"
          : score >= 0.1
            ? "\u001b[48;5;240m"
            : score >= -0.05
              ? "\u001b[48;5;208m"
              : "\u001b[41m";
    return `${style}${text}\u001b[0m`;
  }

  private defaultAnsiEnabled(): boolean {
    return true;
  }

  private readAccountId(snapshot: AuthSnapshot): string | undefined {
    const accountId = snapshot.tokens?.account_id?.trim();
    return accountId ? accountId : undefined;
  }

  private readAccessToken(snapshot: AuthSnapshot): string | undefined {
    const accessToken = snapshot.tokens?.access_token?.trim();
    return accessToken ? accessToken : undefined;
  }

  private formatCacheStatus(view: UsageView): string {
    if (view.fetchedAt === null) return "no-cache";
    if (view.source === "api") return "00:00s";

    const age = Math.max(0, this.nowSeconds() - view.fetchedAt);
    const hours = Math.floor(age / 3600);
    const minutes = Math.floor((age % 3600) / 60);
    const seconds = age % 60;

    if (hours > 0) {
      return `${this.pad2(hours)}h${this.pad2(minutes)}m`;
    }
    return `${this.pad2(minutes)}m${this.pad2(seconds)}s`;
  }

  private pad2(value: number): string {
    return String(value).padStart(2, "0");
  }

  private buildDefaultName(usage: UsageResponse | null, snapshot: AuthSnapshot): string {
    const emailPart = this.sanitizeNamePart(usage?.email ?? null);
    const planPart = this.sanitizeNamePart(usage?.plan_type ?? null);
    const accountPart = this.sanitizeNamePart(this.readAccountId(snapshot) ?? null);

    if (emailPart && planPart) return `${emailPart}-${planPart}`;
    if (emailPart) return emailPart;
    if (accountPart) return `profile-${accountPart.slice(0, 8)}`;
    return "profile";
  }

  private sanitizeNamePart(inputValue: string | null): string | null {
    if (!inputValue) return null;
    const normalized = inputValue
      .trim()
      .toLowerCase()
      .replace(/@/g, "-")
      .replace(/[^a-z0-9._-]+/g, "-")
      .replace(/^-+|-+$/g, "");
    return normalized.length ? normalized : null;
  }

  private async promptSaveCurrent(
    snapshot: AuthSnapshot,
    usage: UsageResponse | null,
  ): Promise<string | null> {
    const defaultName = this.buildDefaultName(usage, snapshot);
    try {
      const rawName = await input({
        message: "Save current profile as",
        default: defaultName,
      });
      return await this.accounts.saveSnapshot(rawName, snapshot);
    } catch (error) {
      if (this.isPromptCancelled(error)) return null;
      if (error instanceof AccountAlreadyExistsError) {
        this.log(error.message);
        return null;
      }
      throw error;
    }
  }

  private async promptRename(currentName: string): Promise<string | null> {
    try {
      const nextName = await input({
        message: `Rename "${currentName}" to`,
        default: currentName,
      });
      return await this.accounts.renameAccount(currentName, nextName);
    } catch (error) {
      if (this.isPromptCancelled(error)) return null;
      if (error instanceof AccountAlreadyExistsError) {
        this.log(error.message);
        return null;
      }
      throw error;
    }
  }

  private async safeConfirm(message: string): Promise<boolean> {
    try {
      return await confirm({ message, default: false });
    } catch (error) {
      if (this.isPromptCancelled(error)) return false;
      throw error;
    }
  }

  private isPromptCancelled(error: unknown): boolean {
    return Boolean(
      error &&
        typeof error === "object" &&
        "name" in error &&
        (error as { name: string }).name === "ExitPromptError",
    );
  }

  private nowSeconds(): number {
    return Math.floor(Date.now() / 1000);
  }

  private computeSelectPageSize(): number {
    const override = Number(process.env.CODEX_AUTH_PAGE_SIZE);
    if (Number.isFinite(override) && override > 0) {
      return Math.floor(override);
    }

    const terminalRows = this.currentTerminalRows();
    const reserveRows = 2;
    return Math.max(12, terminalRows - reserveRows);
  }

  private readTputRows(): number | undefined {
    try {
      const output = execFileSync("tput", ["lines"], {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "ignore"],
      }).trim();
      const rows = Number(output);
      return Number.isFinite(rows) && rows > 0 ? Math.floor(rows) : undefined;
    } catch {
      return undefined;
    }
  }
}
