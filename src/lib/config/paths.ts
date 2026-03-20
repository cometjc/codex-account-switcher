import os from "node:os";
import path from "node:path";

export const codexDir: string = path.join(os.homedir(), ".codex");
export const accountsDir: string = path.join(codexDir, "accounts");
export const authPath: string = path.join(codexDir, "auth.json");
export const currentNamePath: string = path.join(codexDir, "current");
export const limitCachePath: string = path.join(
  codexDir,
  "codex-auth-limit-cache.json",
);
export const uiStatePath: string = path.join(
  codexDir,
  "codex-auth-ui-state.json",
);
