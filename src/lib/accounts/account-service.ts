import fs from "node:fs";
import fsp from "node:fs/promises";
import path from "node:path";
import { accountsDir, authPath, codexDir, currentNamePath } from "../config/paths";
import {
  AccountAlreadyExistsError,
  AccountNotFoundError,
  AuthFileInvalidError,
  AuthFileMissingError,
  InvalidAccountNameError,
} from "./errors";

const ACCOUNT_NAME_PATTERN = /^[a-zA-Z0-9][a-zA-Z0-9._-]*$/;

export interface AuthTokens {
  access_token?: string;
  account_id?: string;
  id_token?: string;
  refresh_token?: string;
}

export interface AuthSnapshot {
  auth_mode?: string;
  last_refresh?: string;
  OPENAI_API_KEY?: string;
  tokens?: AuthTokens;
}

export interface SavedProfile {
  name: string;
  filePath: string;
  snapshot: AuthSnapshot;
}

export class AccountService {
  public async listAccountNames(): Promise<string[]> {
    if (!(await this.pathExists(accountsDir))) {
      return [];
    }

    const entries = await fsp.readdir(accountsDir, { withFileTypes: true });
    return entries
      .filter((entry) => entry.isFile() && entry.name.endsWith(".json"))
      .map((entry) => entry.name.replace(/\.json$/i, ""))
      .sort((a, b) => a.localeCompare(b, undefined, { sensitivity: "base" }));
  }

  public async getCurrentAccountName(): Promise<string | null> {
    const currentName = await this.readCurrentNameFile();
    if (currentName) return currentName;

    if (!(await this.pathExists(authPath))) return null;

    const stat = await fsp.lstat(authPath);
    if (!stat.isSymbolicLink()) return null;

    const rawTarget = await fsp.readlink(authPath);
    const resolvedTarget = path.resolve(path.dirname(authPath), rawTarget);
    const accountsRoot = path.resolve(accountsDir);
    const relative = path.relative(accountsRoot, resolvedTarget);
    if (relative.startsWith("..")) return null;

    const base = path.basename(resolvedTarget);
    return base.replace(/\.json$/i, "");
  }

  public async saveAccount(rawName: string): Promise<string> {
    const name = this.normalizeAccountName(rawName);
    await this.ensureAuthFileExists();
    await this.ensureDir(accountsDir);
    const destination = this.accountFilePath(name);
    await fsp.copyFile(authPath, destination);
    return name;
  }

  public async saveSnapshot(rawName: string, snapshot: AuthSnapshot): Promise<string> {
    const name = this.normalizeAccountName(rawName);
    await this.ensureDir(accountsDir);
    const destination = this.accountFilePath(name);
    if (await this.pathExists(destination)) {
      throw new AccountAlreadyExistsError(name);
    }

    await fsp.writeFile(destination, `${JSON.stringify(snapshot, null, 2)}\n`, "utf8");
    return name;
  }

  public async useAccount(rawName: string): Promise<string> {
    const name = this.normalizeAccountName(rawName);
    const source = this.accountFilePath(name);

    if (!(await this.pathExists(source))) {
      throw new AccountNotFoundError(name);
    }

    await this.ensureDir(accountsDir);
    await this.ensureDir(codexDir);

    if (process.platform === "win32") {
      await fsp.copyFile(source, authPath);
    } else {
      await this.replaceSymlink(source, authPath);
    }

    await this.writeCurrentName(name);
    return name;
  }

  public async deleteAccount(rawName: string): Promise<string> {
    const name = this.normalizeAccountName(rawName);
    const target = this.accountFilePath(name);
    if (!(await this.pathExists(target))) {
      throw new AccountNotFoundError(name);
    }
    await fsp.rm(target, { force: true });
    return name;
  }

  public async renameAccount(rawCurrentName: string, rawNextName: string): Promise<string> {
    const current = this.normalizeAccountName(rawCurrentName);
    const next = this.normalizeAccountName(rawNextName);

    const currentPath = this.accountFilePath(current);
    const nextPath = this.accountFilePath(next);
    if (!(await this.pathExists(currentPath))) {
      throw new AccountNotFoundError(current);
    }
    if (current !== next && (await this.pathExists(nextPath))) {
      throw new AccountAlreadyExistsError(next);
    }

    if (current !== next) {
      await fsp.rename(currentPath, nextPath);
    }

    const currentActive = await this.getCurrentAccountName();
    if (currentActive === current) {
      await this.writeCurrentName(next);
    }

    return next;
  }

  public async getCurrentSnapshot(): Promise<AuthSnapshot> {
    await this.ensureAuthFileExists();
    return this.readSnapshot(authPath);
  }

  public async listSavedProfiles(): Promise<SavedProfile[]> {
    const names = await this.listAccountNames();
    const profiles: SavedProfile[] = [];
    for (const name of names) {
      const filePath = this.accountFilePath(name);
      profiles.push({
        name,
        filePath,
        snapshot: await this.readSnapshot(filePath),
      });
    }
    return profiles;
  }

  private accountFilePath(name: string): string {
    return path.join(accountsDir, `${name}.json`);
  }

  private normalizeAccountName(rawName: string | undefined): string {
    if (typeof rawName !== "string") {
      throw new InvalidAccountNameError();
    }

    const trimmed = rawName.trim();
    if (!trimmed.length) {
      throw new InvalidAccountNameError();
    }

    const withoutExtension = trimmed.replace(/\.json$/i, "");
    if (!ACCOUNT_NAME_PATTERN.test(withoutExtension)) {
      throw new InvalidAccountNameError();
    }

    return withoutExtension;
  }

  private async ensureAuthFileExists(): Promise<void> {
    if (!(await this.pathExists(authPath))) {
      throw new AuthFileMissingError(authPath);
    }
  }

  private async readSnapshot(filePath: string): Promise<AuthSnapshot> {
    try {
      const text = await fsp.readFile(filePath, "utf8");
      return JSON.parse(text) as AuthSnapshot;
    } catch (error) {
      if (error instanceof SyntaxError) {
        throw new AuthFileInvalidError(filePath);
      }
      throw error;
    }
  }

  private async ensureDir(dirPath: string): Promise<void> {
    await fsp.mkdir(dirPath, { recursive: true });
  }

  private async replaceSymlink(target: string, linkPath: string): Promise<void> {
    await this.removeIfExists(linkPath);
    const absoluteTarget = path.resolve(target);
    await fsp.symlink(absoluteTarget, linkPath);
  }

  private async removeIfExists(target: string): Promise<void> {
    try {
      await fsp.rm(target, { force: true });
    } catch (error) {
      const err = error as NodeJS.ErrnoException;
      if (err.code !== "ENOENT") {
        throw error;
      }
    }
  }

  private async writeCurrentName(name: string): Promise<void> {
    await this.ensureDir(codexDir);
    await fsp.writeFile(currentNamePath, `${name}\n`, "utf8");
  }

  private async readCurrentNameFile(): Promise<string | null> {
    try {
      const contents = await fsp.readFile(currentNamePath, "utf8");
      const trimmed = contents.trim();
      return trimmed.length ? trimmed : null;
    } catch (error) {
      const err = error as NodeJS.ErrnoException;
      if (err.code === "ENOENT") {
        return null;
      }
      throw error;
    }
  }

  private async pathExists(targetPath: string): Promise<boolean> {
    try {
      await fsp.access(targetPath, fs.constants.F_OK);
      return true;
    } catch {
      return false;
    }
  }
}
