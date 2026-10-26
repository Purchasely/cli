import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { unlinkSync } from "node:fs";
import { credentialsPath, readJson, writeSecretJson } from "./config.js";
import type { Deps, Keychain } from "./types.js";

/** A surface is one API and its credential. The core knows `management`; an extension defines more. */
export type Surface = string;
export type Source = "env" | "keychain" | "file";

export interface SurfaceDef {
  /** The environment variable that holds the credential. */
  envVar: string;
  /** What the surface calls its credential. */
  what: string;
  /** How to get a credential. `name` is the running program. */
  hint: (name: string) => string;
  /** Requests per second: the default, and the hard cap that only --i-asked-backend passes. */
  rate: { default: number; cap: number };
  /** What to print for a credential, when it is not a plain key. */
  display?: (key: string) => string;
}

const SURFACES = new Map<Surface, SurfaceDef>([
  ["management", {
    envVar: "PURCHASELY_API_KEY",
    what: "Management API key",
    hint: (name) => `Run \`${name} auth login\` or set PURCHASELY_API_KEY.`,
    rate: { default: 0.8, cap: 1 },
  }],
]);

export const defineSurface = (id: Surface, def: SurfaceDef): void => void SURFACES.set(id, def);
export function surfaceDef(id: Surface): SurfaceDef {
  const s = SURFACES.get(id);
  if (!s) throw new Error(`unknown surface ${id}`);
  return s;
}
/** What the surface calls its credential. */
export const what = (s: Surface): string => surfaceDef(s).what;
/** How to get a credential. */
export const hint = (s: Surface, name: string): string => surfaceDef(s).hint(name);

const SERVICE = "purchasely-cli";
type CredFile = Record<string, Record<string, string>>;

export const redact = (key: string | undefined | null) => (key ? `…${key.slice(-4)}` : "-");
/** What to print for a credential: its last 4 characters, unless the surface prints it another way. */
export function credentialDisplay(surface: Surface, key: string | undefined | null): string {
  const display = SURFACES.get(surface)?.display;
  return display && key ? display(key) : redact(key);
}
export const keyHash = (key: string) => createHash("sha256").update(key).digest("hex").slice(0, 12);

function run(cmd: string, args: string[]): Promise<string> {
  return new Promise((resolve, reject) =>
    // No shell. Limit: the secret is on the argv of the child for a few ms (see README).
    execFile(cmd, args, (err, stdout) => (err ? reject(err) : resolve(stdout))),
  );
}

export const macKeychain: Keychain = {
  async get(service, account) {
    try {
      return (await run("security", ["find-generic-password", "-s", service, "-a", account, "-w"])).trim() || null;
    } catch {
      return null;
    }
  },
  async set(service, account, secret) {
    await run("security", ["add-generic-password", "-U", "-s", service, "-a", account, "-w", secret]);
  },
  async delete(service, account) {
    await run("security", ["delete-generic-password", "-s", service, "-a", account]).catch(() => undefined);
  },
};

export const defaultKeychain = (): Keychain | null => (process.platform === "darwin" ? macKeychain : null);

export interface Credential {
  source: Source;
  key: string;
}

export async function resolveCredential(d: Deps, env: string, surface: Surface): Promise<Credential | null> {
  const fromEnv = d.env[surfaceDef(surface).envVar];
  if (fromEnv) return { source: "env", key: fromEnv };
  if (d.keychain) {
    try {
      const k = await d.keychain.get(SERVICE, `${env}:${surface}`);
      if (k) return { source: "keychain", key: k };
    } catch {
      // fall through to the file
    }
  }
  const k = readJson<CredFile>(credentialsPath(d.configDir), {})[env]?.[surface];
  return k ? { source: "file", key: k } : null;
}

export async function storeCredential(d: Deps, env: string, surface: Surface, key: string): Promise<Source> {
  if (d.keychain) {
    try {
      await d.keychain.set(SERVICE, `${env}:${surface}`, key);
      return "keychain";
    } catch {
      // keychain unavailable: use the file
    }
  }
  const file = credentialsPath(d.configDir);
  const all = readJson<CredFile>(file, {});
  all[env] = { ...all[env], [surface]: key };
  writeSecretJson(file, all);
  return "file";
}

export async function removeCredential(d: Deps, env: string, surface: Surface): Promise<void> {
  if (d.keychain) await d.keychain.delete(SERVICE, `${env}:${surface}`).catch(() => undefined);
  const file = credentialsPath(d.configDir);
  const all = readJson<CredFile>(file, {});
  if (all[env]) {
    delete all[env][surface];
    if (Object.keys(all[env]).length === 0) delete all[env];
    if (Object.keys(all).length === 0) {
      try { unlinkSync(file); } catch { /* already gone */ }
    } else writeSecretJson(file, all);
  }
}
