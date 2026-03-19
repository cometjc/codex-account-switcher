import fs from "node:fs";
import fsp from "node:fs/promises";
import { codexDir, limitCachePath } from "../config/paths";

export interface UsageWindow {
  used_percent: number;
  limit_window_seconds: number;
  reset_after_seconds: number;
  reset_at: number;
}

export interface UsageRateLimit {
  primary_window: UsageWindow | null;
  secondary_window: UsageWindow | null;
}

export interface UsageResponse {
  email: string | null;
  plan_type: string | null;
  rate_limit: UsageRateLimit | null;
}

interface UsageCacheRecord {
  fetchedAt: number;
  usage: UsageResponse;
}

interface UsageCacheFile {
  byAccountId: Record<string, UsageCacheRecord>;
}

export interface UsageReadResult {
  usage: UsageResponse | null;
  source: "api" | "cache" | "none";
  fetchedAt: number | null;
  stale: boolean;
}

export class UsageLimitService {
  private readonly ttlSeconds = 300;

  public async readUsage(
    accountId: string | undefined,
    accessToken: string | undefined,
    options?: { forceRefresh?: boolean; cacheOnly?: boolean },
  ): Promise<UsageReadResult> {
    if (!accountId || !accessToken) {
      return { usage: null, source: "none", fetchedAt: null, stale: false };
    }

    const cache = await this.readCache();
    const cached = cache.byAccountId[accountId];
    const age = cached ? this.nowSeconds() - cached.fetchedAt : Number.POSITIVE_INFINITY;
    const forceRefresh = options?.forceRefresh ?? false;
    const cacheOnly = options?.cacheOnly ?? false;

    if (!forceRefresh && cached && (cacheOnly || age <= this.ttlSeconds)) {
      return {
        usage: cached.usage,
        source: "cache",
        fetchedAt: cached.fetchedAt,
        stale: !cacheOnly && age > this.ttlSeconds,
      };
    }

    try {
      const usage = await this.fetchUsage(accountId, accessToken);
      const fetchedAt = this.nowSeconds();
      cache.byAccountId[accountId] = { usage, fetchedAt };
      await this.writeCache(cache);
      return { usage, source: "api", fetchedAt, stale: false };
    } catch {
      if (cached) {
        return {
          usage: cached.usage,
          source: "cache",
          fetchedAt: cached.fetchedAt,
          stale: true,
        };
      }

      return { usage: null, source: "none", fetchedAt: null, stale: false };
    }
  }

  private async fetchUsage(accountId: string, accessToken: string): Promise<UsageResponse> {
    const response = await fetch("https://chatgpt.com/backend-api/wham/usage", {
      method: "GET",
      headers: {
        Authorization: `Bearer ${accessToken}`,
        "ChatGPT-Account-Id": accountId,
        "User-Agent": "codex-auth",
      },
    });

    if (!response.ok) {
      throw new Error(`wham/usage failed: ${response.status}`);
    }

    const payload = (await response.json()) as UsageResponse;
    return payload;
  }

  private async readCache(): Promise<UsageCacheFile> {
    if (!(await this.pathExists(limitCachePath))) {
      return { byAccountId: {} };
    }

    try {
      const raw = await fsp.readFile(limitCachePath, "utf8");
      const parsed = JSON.parse(raw) as UsageCacheFile;
      if (!parsed.byAccountId || typeof parsed.byAccountId !== "object") {
        return { byAccountId: {} };
      }
      return parsed;
    } catch {
      return { byAccountId: {} };
    }
  }

  private async writeCache(cache: UsageCacheFile): Promise<void> {
    await fsp.mkdir(codexDir, { recursive: true });
    await fsp.writeFile(limitCachePath, `${JSON.stringify(cache, null, 2)}\n`, "utf8");
  }

  private async pathExists(targetPath: string): Promise<boolean> {
    try {
      await fsp.access(targetPath, fs.constants.F_OK);
      return true;
    } catch {
      return false;
    }
  }

  private nowSeconds(): number {
    return Math.floor(Date.now() / 1000);
  }
}

export const usageLimitService = new UsageLimitService();
