import { BaseCommand } from "../lib/base-command";

export default class SingleCommandCompat extends BaseCommand {
  static hidden = true;
  static description = "Internal compatibility command for oclif lookup.";

  async run(): Promise<void> {
    this.error('Run "codex-auth" to start the interactive profile manager.');
  }
}
