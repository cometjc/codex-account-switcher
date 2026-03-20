import fsp from "node:fs/promises";
import { codexDir, uiStatePath } from "./paths";

export type PersistedWorkloadTier = "auto" | "low" | "medium" | "high";

interface UiStateFile {
  workloadTier?: string;
}

export class UiStateService {
  public async readWorkloadTier(): Promise<PersistedWorkloadTier> {
    const state = await this.readStateFile();
    return this.normalizeWorkloadTier(state?.workloadTier);
  }

  public async writeWorkloadTier(workloadTier: PersistedWorkloadTier): Promise<void> {
    await fsp.mkdir(codexDir, { recursive: true });
    const payload: UiStateFile = { workloadTier };
    await fsp.writeFile(uiStatePath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
  }

  private async readStateFile(): Promise<UiStateFile | null> {
    try {
      const raw = await fsp.readFile(uiStatePath, "utf8");
      return JSON.parse(raw) as UiStateFile;
    } catch (error) {
      const err = error as NodeJS.ErrnoException;
      if (err.code === "ENOENT") return null;
      if (error instanceof SyntaxError) return null;
      throw error;
    }
  }

  private normalizeWorkloadTier(value: string | undefined): PersistedWorkloadTier {
    switch (value) {
      case "low":
      case "medium":
      case "high":
      case "auto":
        return value;
      default:
        return "auto";
    }
  }
}

export const uiStateService = new UiStateService();
