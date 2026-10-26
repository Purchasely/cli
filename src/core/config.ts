import { chmodSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export interface Config {
  /** Sent in the User-Agent by an extension with `identity` only. */
  identity_email?: string;
  rate?: Record<string, number>;
  /** `false` turns off the automatic update check and notice. `purchasely update` still works. */
  auto_update?: boolean;
  /** `true` refuses every write before it is sent. Same as `PURCHASELY_READ_ONLY=1`. */
  read_only?: boolean;
  /** Per env, the only apps a write may change. A missing env means no limit; an empty list refuses every write. */
  write_allowlist?: { staging?: string[]; prod?: string[] };
}

export function readJson<T>(file: string, fallback: T): T {
  try {
    return JSON.parse(readFileSync(file, "utf8")) as T;
  } catch {
    return fallback;
  }
}

/** Write JSON with mode 0600 in a 0700 dir. chmod after write so an existing file is fixed too. */
export function writeSecretJson(file: string, data: unknown): void {
  const dir = join(file, "..");
  mkdirSync(dir, { recursive: true, mode: 0o700 });
  chmodSync(dir, 0o700);
  writeFileSync(file, JSON.stringify(data, null, 2) + "\n", { mode: 0o600 });
  chmodSync(file, 0o600);
}

export const configPath = (dir: string) => join(dir, "config.json");
export const credentialsPath = (dir: string) => join(dir, "credentials.json");

export const readConfig = (dir: string) => readJson<Config>(configPath(dir), {});
export const writeConfig = (dir: string, c: Config) => writeSecretJson(configPath(dir), c);
