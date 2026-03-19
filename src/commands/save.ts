import { BaseCommand } from "../lib/base-command";

export default class SaveCommand extends BaseCommand {
  static hidden = true;
  static description = "Deprecated. Use root interactive command.";

  async run(): Promise<void> {
    this.error('This command is deprecated. Run "codex-auth" and use the interactive menu.');
  }
}
