const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');
const RootCommand = require(path.join(process.cwd(), 'dist/commands/root.js')).default;

function createCommand(RootCommand) {
  const command = Object.create(RootCommand.prototype);
  command.ansiEnabled = false;
  return command;
}

function createUsage({
  weeklyUsed,
  weeklyResetAt,
  fiveHourUsed,
  fiveHourResetAt,
}) {
  return {
    email: 'demo@example.com',
    plan_type: 'plus',
    rate_limit: {
      primary_window: {
        used_percent: fiveHourUsed,
        limit_window_seconds: 18_000,
        reset_after_seconds: Math.max(0, fiveHourResetAt - 1_700_000_000),
        reset_at: fiveHourResetAt,
      },
      secondary_window: {
        used_percent: weeklyUsed,
        limit_window_seconds: 604_800,
        reset_after_seconds: Math.max(0, weeklyResetAt - 1_700_000_000),
        reset_at: weeklyResetAt,
      },
    },
  };
}

test('actions help text shows workload tier state', async () => {
  const command = createCommand(RootCommand);

  const helpText = command.buildActionsHelpText('delta', 'auto');

  assert.match(helpText, /\[Q\]uit/);
  assert.match(helpText, /\[W\]orkload: Auto/);
});

test('workload tiers map to concise routing bias hints', () => {
  const command = createCommand(RootCommand);

  assert.match(command.workloadTierHint('auto'), /default routing balance/i);
  assert.match(command.workloadTierHint('low'), /conserve short-window capacity/i);
  assert.match(command.workloadTierHint('medium'), /balanced short and long window/i);
  assert.match(command.workloadTierHint('high'), /favor aggressive weekly throughput/i);
});

test('status line shows the active workload tier hint without changing option semantics', () => {
  const command = createCommand(RootCommand);

  assert.match(
    command.renderStatusLine('delta', 'auto'),
    /Workload Auto: default routing balance/i,
  );
  assert.match(
    command.renderStatusLine('quota', 'high'),
    /Workload High: favor aggressive weekly throughput/i,
  );
});

test('root command reads persisted workload tier and writes updates back', async () => {
  const command = createCommand(RootCommand);
  let storedTier = 'medium';
  command.uiState = {
    readWorkloadTier: async () => storedTier,
    writeWorkloadTier: async (nextTier) => {
      storedTier = nextTier;
    },
  };

  assert.equal(await command.readInitialWorkloadTier(), 'medium');

  await command.persistWorkloadTier('high');
  assert.equal(storedTier, 'high');
});

test('auto workload remains the default scoring path', async () => {
  const command = createCommand(RootCommand);
  command.nowSeconds = () => 1_700_000_000;

  const usage = createUsage({
    weeklyUsed: 18,
    weeklyResetAt: 1_700_345_600,
    fiveHourUsed: 72,
    fiveHourResetAt: 1_700_014_400,
  });

  const implicitAuto = command.computeSummary(usage, false);
  const explicitAuto = command.computeSummary(usage, false, 'auto');

  assert.equal(implicitAuto.score, explicitAuto.score);
  assert.equal(implicitAuto.scoreLabel, explicitAuto.scoreLabel);
});

test('workload tiers shift routing scores from conservative to aggressive', async () => {
  const command = createCommand(RootCommand);
  command.nowSeconds = () => 1_700_000_000;

  const usage = createUsage({
    weeklyUsed: 18,
    weeklyResetAt: 1_700_345_600,
    fiveHourUsed: 72,
    fiveHourResetAt: 1_700_014_400,
  });

  const low = command.computeSummary(usage, false, 'low');
  const medium = command.computeSummary(usage, false, 'medium');
  const high = command.computeSummary(usage, false, 'high');

  assert.ok(low.score < medium.score);
  assert.ok(medium.score < high.score);
});
