#!/usr/bin/env node

function parseArgs(argv) {
  const args = {};
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value.startsWith('--')) {
      const key = value.slice(2);
      const next = argv[index + 1];
      if (!next || next.startsWith('--')) {
        args[key] = true;
      } else {
        args[key] = next;
        index += 1;
      }
    }
  }
  return args;
}

function composeMessage(args) {
  const lane = args.lane ? `Lane ${args.lane}`.replace(/^Lane\s+Lane\s+/, 'Lane ') : null;
  const context = {
    execution: args.execution || 'n/a',
    lane: lane || 'n/a',
    item: args.item || 'n/a',
    scope: args.scope || 'Use the lane ownership family only.',
    verification: args.verification || 'Run the lane-local verification commands.',
    failReason: args['fail-reason'] || 'n/a',
    files: args.files || 'n/a',
    commit: args.commit || 'n/a',
  };

  switch (args.phase) {
    case 'implementer-assignment':
      return [
        `Execution: ${context.execution}`,
        `Lane: ${context.lane}`,
        `Lane item intent: ${context.item}`,
        `Write scope: ${context.scope}`,
        `Required verification: ${context.verification}`,
        'Required handoff format: return only one strict NLSDD lane handoff envelope JSON object.',
        'Required envelope keys: execution, lane, role, eventType, phaseBefore, phaseAfter, currentItem, nextRefillTarget, relatedCommit, verification, summary, detail, nextExpectedPhase, blockedBy, proposedCommitTitle, proposedCommitBody, insights, timestamp.',
        'Do not run git commit yourself unless this lane explicitly says self-commit is allowed.',
        'Default NLSDD flow in this repo: hand back READY_TO_COMMIT with intended commit title/body summary so coordinator can commit for you.',
      ].join('\n');
    case 'spec-review':
      return [
        `Execution: ${context.execution}`,
        `Lane: ${context.lane}`,
        `Review target commit: ${context.commit}`,
        `Lane item: ${context.item}`,
        'Inspect only the lane-item commit diff.',
        'Ignore total dirty worktree state.',
        'Return the result as one strict NLSDD lane handoff envelope JSON object with role=spec-reviewer and eventType=pass or fail.',
      ].join('\n');
    case 'quality-review':
      return [
        `Execution: ${context.execution}`,
        `Lane: ${context.lane}`,
        `Review target commit: ${context.commit}`,
        `Lane item: ${context.item}`,
        'Inspect only the same lane-item commit diff.',
        'Return the result as one strict NLSDD lane handoff envelope JSON object with role=quality-reviewer and eventType=pass or fail.',
      ].join('\n');
    case 'correction-loop':
      return [
        `Execution: ${context.execution}`,
        `Lane: ${context.lane}`,
        `Failing commit: ${context.commit}`,
        `Lane item: ${context.item}`,
        `Reviewer finding: ${context.failReason}`,
        `Accepted write scope: ${context.scope}`,
        `Relevant files: ${context.files}`,
        `Required verification: ${context.verification}`,
        'Return a new strict NLSDD lane handoff envelope JSON object, or READY_TO_COMMIT as eventType=ready-to-commit with proposed commit title/body if coordinator commit is required.',
      ].join('\n');
    default:
      throw new Error(
        'Usage: node NLSDD/scripts/nlsdd-compose-message.cjs --phase <implementer-assignment|spec-review|quality-review|correction-loop> ...',
      );
  }
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  process.stdout.write(`${composeMessage(args)}\n`);
}

if (require.main === module) {
  main();
}

module.exports = {
  parseArgs,
  composeMessage,
};
