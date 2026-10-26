import { join } from "node:path";
import type { Command } from "commander";
import type { Config } from "./config.js";
import { CliError, EXIT } from "./errors.js";
import type { Runtime } from "./runtime.js";
import { StateStore } from "./state.js";
import type { Deps } from "./types.js";

export type Kind = "managed" | "brew" | "npm";
/** Record `update.<program>.check.json`. Every write is a re-read + merge of the fields the writer owns. */
interface Record_ { checked_at?: number; latest?: string; notified_at?: number }

export const DAY = 24 * 3600_000;

export const installKind = (env: Deps["env"], moduleUrl: string): Kind =>
  env.PURCHASELY_MANAGED_INSTALL_ROOT ? "managed" : moduleUrl.includes("/Cellar/") ? "brew" : "npm";

const parts = (v: string) => v.replace(/^v/, "").split(".").map(Number);
/** `a` is a higher x.y.z than `b`. */
export function newer(a: string, b: string): boolean {
  const x = parts(a);
  const y = parts(b);
  for (let i = 0; i < 3; i++) if (x[i] !== y[i]) return x[i] > y[i];
  return false;
}

export const autoUpdateOff = (env: Deps["env"], cfg: Config): boolean =>
  env.CI !== undefined || env.PURCHASELY_DISABLE_AUTOUPDATE === "1" || env.PURCHASELY_OFFLINE === "1" || env.PURCHASELY_DEMO === "1" || cfg.auto_update === false;

/** An npm install of `purchasely` runs the install script again; an extension may name its own command. */
export const noticeLine = (d: Deps, kind: "brew" | "npm", latest: string): string =>
  `${d.name} ${latest} is available: ${kind === "brew" ? `brew upgrade ${d.name}` : d.ext?.npmUpdate ?? `curl -fsSL ${d.host}/install.sh | sh`}`;

/** A registered plugin agent whose daily check is due (record `ai.plugin.json`, owned by `core/ai.ts`). */
const pluginDue = (d: Deps, now: number): boolean => {
  const r = new StateStore(d, "ai").read<{ checked_at?: number; agents?: object } | null>("plugin", null);
  return !!r && typeof r.agents === "object" && r.agents !== null && Object.keys(r.agents).length > 0 && now - (r.checked_at ?? 0) >= DAY;
};

export const updateStore = (d: Deps) => new StateStore(d, "update." + d.name);
export function readRecord(s: StateStore): Record_ {
  const r = s.read<unknown>("check", {});
  return r !== null && typeof r === "object" && !Array.isArray(r) ? (r as Record_) : {};
}
/** Call inside `withLock`. */
export const mergeRecord = (s: StateStore, patch: Record_): void => s.write("check", { ...readRecord(s), ...patch });

export const stable = (v: string): string | null => (/^[0-9]+\.[0-9]+\.[0-9]+$/.test(v) ? v : null);

/** Latest public release: `<host>/latest.json`. Fixed message, the body is never forwarded. */
async function latestPublic(d: Deps): Promise<string | null> {
  const fail = () => new CliError(EXIT.GENERIC, "UPDATE_CHECK_FAILED", ["could not check the latest version. Check your network, then try again."]);
  let v: unknown;
  try {
    const r = await d.fetch(d.host + "/latest.json", { signal: AbortSignal.timeout(15000) });
    if (r.status !== 200) throw fail();
    v = ((await r.json()) as { version?: unknown }).version;
  } catch {
    throw fail();
  }
  if (typeof v !== "string") throw fail();
  return stable(v.replace(/^v/, ""));
}

/** Latest release version: the channel of the extension, or `<host>/latest.json`. `null` = no stable release to compare with. */
export const latestVersion = (d: Deps): Promise<string | null> => (d.ext?.latest ? d.ext.latest(d) : latestPublic(d));

/** The installer of the running release installs `ver`. `--upgrade-only`: it does nothing when `ver` is not newer. */
export const runInstaller = (d: Deps, root: string, ver: string, log?: string) =>
  d.exec("sh", [join(root, "releases", d.version, "install.sh"), "--version", ver, "--keep", d.version, "--root", root, "--upgrade-only", ...(log ? ["--log", log] : [])], { timeoutMs: 600_000 });

/**
 * `preAction` hook. The parent never waits: it reads the record, prints at most one notice, and spawns the
 * detached child `update --background`. Reservation and results are written by the child. An update check never
 * fails a command.
 */
export async function autoUpdate(rt: Runtime, action: Command): Promise<void> {
  const { d } = rt;
  try {
    let top = action;
    while (top.parent?.parent) top = top.parent;
    // `update` and the `ai` group do their work in the foreground: a child would race them for the lock
    if (autoUpdateOff(d.env, rt.cfg) || top.name() === "ai" || (action.parent?.parent == null && action.name() === "update")) return;
    const kind = installKind(d.env, import.meta.url);
    const s = updateStore(d);
    const now = d.now();
    const rec = readRecord(s);
    if (kind !== "managed" && rec.latest && newer(rec.latest, d.version) && now - (rec.notified_at ?? 0) >= DAY) {
      const latest = rec.latest;
      // timeout -1: one lock attempt, no wait. On contention the notice is skipped.
      await s.withLock(() => {
        if (now - (readRecord(s).notified_at ?? 0) < DAY) return;
        mergeRecord(s, { notified_at: now });
        d.stderr(noticeLine(d, kind, latest) + "\n");
      }, -1).catch(() => undefined);
    }
    if (now - (rec.checked_at ?? 0) < DAY && !pluginDue(d, now)) return;
    await d.exec(process.execPath, [process.argv[1], "update", "--background"], { timeoutMs: 0, detach: true });
  } catch {
    // never fails the command
  }
}
