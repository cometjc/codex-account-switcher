const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs/promises");
const path = require("node:path");

const RootCommand = require(path.join(process.cwd(), "dist/commands/root.js")).default;

function createRootCommand() {
  const command = Object.create(RootCommand.prototype);
  command.ansiEnabled = false;
  command.logs = [];
  command.log = (message) => {
    command.logs.push(message);
  };
  command.currentTerminalColumns = () => 88;
  command.currentTerminalRows = () => 26;
  command.nowSeconds = () => 1_701_234_567;
  command.resolvePlotViewerBinaryPath = async () => null;
  return command;
}

test("plot handoff keeps the plot mode seam and writes a temp snapshot when no viewer exists", async () => {
  const command = createRootCommand();

  assert.equal(command.nextBarStyle("quota"), "delta");
  assert.equal(command.nextBarStyle("delta"), "plot");
  assert.equal(command.nextBarStyle("plot"), "quota");
  assert.equal(command.formatBarStyle("plot"), "Plot");

  const items = [
    {
      savedName: "alpha",
      profileName: "Alpha",
      isCurrent: true,
      usage: null,
      accountId: "acct-alpha",
    },
  ];

  const context = command.buildPlotModeLaunchContext(items, "auto", "plot");
  assert.deepEqual(context, {
    barStyle: "plot",
    itemCount: 1,
    items,
    workloadTier: "auto",
    terminalColumns: 88,
    terminalRows: 26,
  });

  const launched = await command.maybeLaunchPlotMode(context);
  assert.equal(launched, false);
  assert.equal(command.logs.length, 1);

  const [logLine] = command.logs;
  const match = logLine.match(
    /^Plot snapshot prepared at (.+\/snapshot\.json) \(1 profiles; viewer binary not available yet\)\.$/,
  );
  assert.ok(match, "expected plot handoff log to include the temp snapshot path");

  const snapshotPath = match[1];
  const snapshotText = await fs.readFile(snapshotPath, "utf8");
  const snapshot = JSON.parse(snapshotText);

  assert.deepEqual(snapshot, {
    currentProfileId: "saved:alpha",
    generatedAt: 1_701_234_567,
    profiles: [
      {
        fiveHourBand: {
          available: false,
          bandHeight: null,
          delta5hPercent: null,
          delta7dPercent: null,
          lowerY: null,
          reason: "band-not-provided",
          upperY: null,
        },
        fiveHourWindow: {
          endAt: null,
          startAt: null,
        },
        id: "saved:alpha",
        isCurrent: true,
        name: "Alpha",
        sevenDayPoints: [],
        sevenDayWindow: {
          endAt: null,
          startAt: null,
        },
        summaryLabels: {
          drift: "Drift",
          pacingStatus: "Pacing Status",
          timeToReset: "Time to reset",
          usageLeft: "Usage Left",
        },
        usage: null,
      },
    ],
    schemaVersion: 1,
  });
});
