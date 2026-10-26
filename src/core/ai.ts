import { cpSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { CliError, EXIT, usage } from "./errors.js";
import type { Runtime } from "./runtime.js";
import { StateStore } from "./state.js";
import type { Deps, ExecResult, Globals } from "./types.js";
import { DAY, newer } from "./update.js";

export const AGENTS = ["claude", "codex", "copilot", "gemini", "cursor", "opencode"] as const;
export type Agent = (typeof AGENTS)[number];
type NativeAgent = "claude" | "codex" | "copilot" | "gemini";
export type Verb = "install" | "update" | "uninstall" | "status";
type Rec = Record<string, string>;
type Step = [label: string, argv: string[]];

const REPO = "Purchasely/Purchasely-AI-Plugin";
const API = `https://api.github.com/repos/${REPO}/releases/latest`;
const tarballUrl = (tag: string) => `https://codeload.github.com/${REPO}/tar.gz/refs/tags/${tag}`;
const MARKER = ".purchasely-cli";
const AGENT_TIMEOUT = 120_000;

/** What `ai install` recorded for one agent. `probe` = the identity fields of the agent's own list entry. */
interface Reg { method: "native" | "copy"; id?: string; scope?: string; probe?: Rec; targets?: string[]; version: string; updated_at: number; last_error?: string }
interface Plugin { checked_at?: number; latest?: string; agents: Partial<Record<Agent, Reg>> }

export type Action = "installed" | "updated" | "up_to_date" | "removed" | "not_detected" | "installed_by_hand" | "refused" | "failed" | "planned";
export interface Row { agent: Agent; action: Action; version: string | null; follows: "release tag" | "default branch"; message?: string; ops?: string[] }

const json = (s: string): unknown => { try { return JSON.parse(s); } catch { return null; } };
const pick = (o: Record<string, unknown>, keys: string[]): Rec => Object.fromEntries(keys.filter((k) => typeof o[k] === "string").map((k) => [k, o[k] as string]));
const objs = (v: unknown): Record<string, unknown>[] | null => (Array.isArray(v) ? v.filter((x) => x && typeof x === "object") : null);

/** An agent that has its own plugin commands. The argv table of the plan, section 3.0. */
interface Native {
  id: string;
  list: string[];
  /** Our plugin's entries in the output of `list`, as identity fields only. `null` = not parsable. */
  find: (out: string) => Rec[] | null;
  mkt?: { add: string[]; list: string[]; has: RegExp };
  install: Step[];
  update: Step[];
  uninstall: Step[];
  /** The list of this agent proves only the name: update and uninstall are left to its own command. */
  noIdentity?: boolean;
}
const MKT = "Purchasely-AI-Plugin";
const CLAUDE_ID = `purchasely@${MKT}`;
const jsonName = (n: string) => new RegExp(`"name"\\s*:\\s*"${n}"`);
const NATIVE: Record<NativeAgent, Native> = {
  claude: {
    id: CLAUDE_ID,
    list: ["plugin", "list", "--json"],
    find: (o) => objs(json(o))?.filter((e) => e.id === CLAUDE_ID).map((e) => pick(e, ["id", "scope", "installedAt"])) ?? null,
    mkt: { add: ["plugin", "marketplace", "add", REPO], list: ["plugin", "marketplace", "list", "--json"], has: jsonName(MKT) },
    install: [["install", ["plugin", "install", CLAUDE_ID, "-s", "user"]]],
    update: [["marketplace update", ["plugin", "marketplace", "update", MKT]], ["update", ["plugin", "update", CLAUDE_ID, "-s", "user"]]],
    uninstall: [["uninstall", ["plugin", "uninstall", CLAUDE_ID, "-s", "user"]]],
  },
  codex: {
    id: "purchasely@purchasely",
    list: ["plugin", "list", "--json", "-m", "purchasely"],
    find: (o) => {
      const j = json(o);
      return objs(Array.isArray(j) ? j : (j as { installed?: unknown } | null)?.installed)?.filter((e) => e.pluginId === "purchasely@purchasely" && e.installed !== false).map((e) => pick(e, ["pluginId", "marketplaceName"])) ?? null;
    },
    mkt: { add: ["plugin", "marketplace", "add", REPO], list: ["plugin", "marketplace", "list", "--json"], has: jsonName("purchasely") },
    install: [["install", ["plugin", "add", "purchasely@purchasely"]]],
    update: [["marketplace upgrade", ["plugin", "marketplace", "upgrade", "purchasely"]]],
    uninstall: [["uninstall", ["plugin", "remove", "purchasely@purchasely"]]],
  },
  copilot: {
    id: CLAUDE_ID,
    list: ["plugin", "list", "--json"],
    find: (o) => objs(json(o))?.filter((e) => e.name === "purchasely" && e.marketplace === MKT).map((e) => pick(e, ["name", "marketplace"])) ?? null,
    mkt: { add: ["plugin", "marketplace", "add", REPO], list: ["plugin", "marketplace", "list"], has: new RegExp(`(?<![\\w-])${MKT}(?![\\w-])`) },
    install: [["install", ["plugin", "install", CLAUDE_ID]]],
    update: [["update", ["plugin", "update", CLAUDE_ID]]],
    uninstall: [["uninstall", ["plugin", "uninstall", CLAUDE_ID]]],
  },
  gemini: {
    id: "purchasely",
    list: ["extensions", "list"],
    find: (o) => o.split("\n").filter((l) => /^[^A-Za-z0-9]*purchasely(\s|\(|$)/.test(l)).map(() => ({ name: "purchasely" })),
    install: [["install", ["extensions", "install", `https://github.com/${REPO}`, "--consent"]]],
    update: [["update", ["extensions", "update", "purchasely"]]],
    uninstall: [["uninstall", ["extensions", "uninstall", "purchasely"]]],
    noIdentity: true,
  },
};
const isNative = (a: Agent): a is NativeAgent => a in NATIVE;

const home = (d: Deps) => d.env.HOME ?? "";
const fail = (msg: string) => new CliError(EXIT.GENERIC, "AI_FAILED", [msg]);
const cmdline = (bin: string, argv: string[]) => [bin, ...argv].join(" ");
const stepFailed = (a: Agent, step: string, argv: string[], code: number) => fail(`${a}: the plugin ${step} failed (exit ${code}). Run "${cmdline(a, argv)}" yourself to see why.`);
const refusal = (a: Agent, reason: string) => `${a}: ${reason}. Nothing was changed.`;
const follows = (a: Agent): Row["follows"] => (isNative(a) ? "default branch" : "release tag");
const row = (agent: Agent, action: Action, version: string | null, extra: Partial<Row> = {}): Row => ({ agent, action, version, follows: follows(agent), ...extra });

/** Run an agent program: never a shell, never forwarded output. `null` = the program is missing. */
const sh = (d: Deps, bin: string, argv: string[]) => d.exec(bin, argv, { timeoutMs: AGENT_TIMEOUT }).catch(() => null);
const good = (r: ExecResult | null): r is ExecResult => !!r && r.code === 0 && !r.timedOut;
const codeOf = (r: ExecResult | null): number => r?.code ?? 127;

export async function detect(d: Deps, a: Agent): Promise<boolean> {
  if (a === "cursor") return !!home(d) && existsSync(join(home(d), ".cursor")) && statSync(join(home(d), ".cursor")).isDirectory();
  return good(await sh(d, a, ["--version"]));
}

const cursorTarget = (d: Deps) => join(home(d), ".cursor", "plugins", "local", "purchasely");
const opencodeRoot = (d: Deps) => join(home(d), ".config", "opencode");
/** The owned dirs of a copy agent. Without `src` (no payload yet) only the dirs known in advance. */
function layout(d: Deps, a: "cursor" | "opencode", src?: string): { src: string; dst: string }[] {
  if (a === "cursor") return [{ src: src ?? "", dst: cursorTarget(d) }];
  const refs = { src: join(src ?? "", "references"), dst: join(opencodeRoot(d), "references") };
  if (!src) return [refs];
  const skills = readdirSync(join(src, "skills"), { withFileTypes: true }).filter((e) => e.isDirectory()).map((e) => e.name);
  return [...skills.map((s) => ({ src: join(src, "skills", s), dst: join(opencodeRoot(d), "skills", s) })), refs];
}
const marker = (dir: string): string | null => { try { return readFileSync(join(dir, MARKER), "utf8").trim(); } catch { return null; } };

/** Copy to a temp dir, move the old dir aside, rename, delete the old dir, write the marker last. */
function copyTo(d: Deps, src: string, target: string, version: string): void {
  mkdirSync(dirname(target), { recursive: true });
  const tmp = `${target}.tmp-${d.pid}`;
  const old = `${target}.old-${d.pid}`;
  rmSync(tmp, { recursive: true, force: true });
  cpSync(src, tmp, { recursive: true });
  if (existsSync(target)) renameSync(target, old);
  renameSync(tmp, target);
  rmSync(old, { recursive: true, force: true });
  writeFileSync(join(target, MARKER), version + "\n");
}

const unreadable = () => new CliError(EXIT.GENERIC, "AI_RELEASE_UNREADABLE", ["could not read the latest plugin release. Check your network."]);
const ua = (d: Deps) => ({ "User-Agent": `purchasely-cli/${d.version}`, Accept: "application/vnd.github+json" });

/** Latest plugin release, checked before it enters a URL or a path. Never through the Purchasely limiter. */
export async function latestTag(d: Deps): Promise<string> {
  let v: unknown;
  try {
    const r = await d.fetch(API, { headers: ua(d), signal: AbortSignal.timeout(15000) });
    if (r.status !== 200) throw new Error("status");
    v = ((await r.json()) as { tag_name?: unknown }).tag_name;
  } catch {
    throw unreadable();
  }
  const tag = typeof v === "string" ? v : "";
  if (!/^v?\d+\.\d+\.\d+$/.test(tag)) throw unreadable();
  return tag;
}

/** Download and unpack the release tarball into a fresh dir. Returns the `purchasely/` payload dir. */
async function fetchPayload(d: Deps, tag: string): Promise<{ dir: string; src: string }> {
  const err = () => fail("could not download the plugin release. Check your network.");
  mkdirSync(join(d.cacheDir, "ai"), { recursive: true, mode: 0o700 });
  const dir = mkdtempSync(join(d.cacheDir, "ai", "release-"));
  try {
    const r = await d.fetch(tarballUrl(tag), { headers: ua(d), signal: AbortSignal.timeout(60000) }).catch(() => null);
    if (!r || r.status !== 200) throw err();
    writeFileSync(join(dir, "plugin.tgz"), Buffer.from(await r.arrayBuffer()));
    if (!good(await sh(d, "tar", ["-xzf", join(dir, "plugin.tgz"), "-C", dir]))) throw err();
    const src = join(dir, `Purchasely-AI-Plugin-${tag.replace(/^v/, "")}`, "purchasely");
    if (!existsSync(src)) throw err();
    return { dir, src };
  } catch (e) {
    rmSync(dir, { recursive: true, force: true });
    throw e;
  }
}

/** One run of the commands, or of the daily step. */
class Session {
  store: StateStore;
  tag?: string; // the version, without a leading v
  ref?: string; // the raw release tag, for the tarball URL
  private pl?: { dir: string; src: string };
  constructor(public d: Deps, public background: boolean, public dry: boolean) {
    this.store = new StateStore(d, "ai");
  }
  read(): Plugin {
    const r = this.store.read<Partial<Plugin> | null>("plugin", {});
    return { ...r, agents: r && typeof r.agents === "object" && r.agents ? r.agents : {} };
  }
  write(p: Plugin): void {
    this.store.write("plugin", p);
  }
  /** Call inside the lock: re-read, change the one agent, write. */
  put(a: Agent, reg: Reg | null): void {
    const p = this.read();
    if (reg) p.agents[a] = reg;
    else delete p.agents[a];
    if (this.tag) p.latest = this.tag;
    this.write(p);
  }
  async src(): Promise<string> {
    this.pl ??= await fetchPayload(this.d, this.ref!);
    return this.pl.src;
  }
  done(): void {
    if (this.pl) rmSync(this.pl.dir, { recursive: true, force: true });
  }

  /** One agent operation under the `ai` lock. A failure is recorded in `last_error` and becomes a row. */
  async guarded(a: Agent, fn: () => Promise<Row>): Promise<Row | null> {
    const body = async (): Promise<Row> => {
      try {
        return await fn();
      } catch (e) {
        if (!(e instanceof CliError)) throw e;
        const p = this.read();
        if (p.agents[a] && !this.dry) {
          p.agents[a]!.last_error = e.messages[0];
          this.write(p);
        }
        return row(a, "failed", p.agents[a]?.version ?? null, { message: e.messages[0] });
      }
    };
    if (this.dry) return body();
    try {
      return await this.store.withLock(body, this.background ? -1 : 300_000);
    } catch (e) {
      if ((e as Error).message !== "state lock timeout") throw e;
      return this.background ? null : row(a, "refused", this.read().agents[a]?.version ?? null, { message: refusal(a, "another ai command is running") });
    }
  }
}

type Own = { ok: true } | { ok: false; reason: string; drop?: boolean };
async function owns(s: Session, a: Agent, reg: Reg): Promise<Own> {
  if (isNative(a)) {
    const r = await sh(s.d, a, NATIVE[a].list);
    const e = good(r) ? NATIVE[a].find(r.stdout) : null;
    if (!e) return { ok: false, reason: "the list of installed plugins could not be read" };
    if (e.length === 0) return { ok: false, drop: true, reason: "the plugin is gone from the agent" };
    if (e.length > 1) return { ok: false, reason: "the plugin is installed more than once (for example in the user and the project scope)" };
    return JSON.stringify(e[0]) === JSON.stringify(reg.probe) ? { ok: true } : { ok: false, reason: "the plugin was replaced since ai install" };
  }
  const t = reg.targets ?? [];
  if (t.length && t.every((x) => !existsSync(x))) return { ok: false, drop: true, reason: "the plugin is gone from the agent" };
  return t.length && t.every((x) => marker(x) === reg.version) ? { ok: true } : { ok: false, reason: "the installed files changed since ai install" };
}

async function steps(d: Deps, a: NativeAgent, list: Step[]): Promise<void> {
  for (const [label, argv] of list) {
    const r = await sh(d, a, argv);
    if (!good(r)) throw stepFailed(a, label, argv, codeOf(r));
  }
}
/** Probe after a mutation: exactly one entry, its identity is what the next ownership check compares with. */
async function identity(d: Deps, a: NativeAgent, step: string): Promise<Rec> {
  const r = await sh(d, a, NATIVE[a].list);
  const e = good(r) ? NATIVE[a].find(r.stdout) : null;
  if (e?.length !== 1) throw stepFailed(a, `${step} check`, NATIVE[a].list, codeOf(r));
  return e[0];
}

const now = (s: Session) => s.d.now();
const lastTag = (s: Session) => s.tag ?? s.read().latest ?? "latest";

function planned(s: Session, a: Agent, verb: Exclude<Verb, "status">, reg?: Reg): Row {
  const ops: string[] = [];
  if (isNative(a)) {
    const n = NATIVE[a];
    const list = verb === "install" ? [...(n.mkt ? [["marketplace add", n.mkt.add] as Step] : []), ...n.install] : n[verb];
    ops.push(...list.map(([, argv]) => cmdline(a, argv)));
  } else if (verb === "uninstall") {
    ops.push(...(reg?.targets ?? []).map((t) => `delete ${t}`));
  } else {
    const dsts = a === "cursor" ? [cursorTarget(s.d)] : [join(opencodeRoot(s.d), "skills", "<each skill>"), join(opencodeRoot(s.d), "references")];
    ops.push(`fetch ${tarballUrl(lastTag(s))}`, ...dsts.map((t) => `copy to ${t} and write ${MARKER}`));
  }
  return row(a, "planned", reg?.version ?? lastTag(s), { ops });
}

const hand = (a: Agent) => row(a, "installed_by_hand", null, { message: refusal(a, "a plugin is already here that ai install did not install") });

/** A refusal. `drop`: the plugin is gone, so the record goes too. */
function refused(s: Session, a: Agent, reg: Reg, reason: string, drop = false): Row {
  const message = drop ? `${a}: the plugin is gone from the agent. The record was dropped.` : refusal(a, reason);
  if (!s.dry) {
    if (drop) s.put(a, null);
    else s.put(a, { ...reg, last_error: message });
  }
  return row(a, "refused", reg.version, { message });
}

async function install(s: Session, a: Agent): Promise<Row> {
  const { d } = s;
  const reg = s.read().agents[a];
  if (reg) return update(s, a, true);
  if (isNative(a)) {
    const n = NATIVE[a];
    const r = await sh(d, a, n.list);
    const found = good(r) ? n.find(r.stdout) : null;
    if (!found) return row(a, "refused", null, { message: refusal(a, "the list of installed plugins could not be read") });
    if (found.length) return hand(a);
    if (s.dry) return planned(s, a, "install");
    if (n.mkt) {
      const add = await sh(d, a, n.mkt.add);
      if (!good(add)) {
        // a marketplace that is already registered makes `add` fail: go on only when the list shows it
        const l = await sh(d, a, n.mkt.list);
        if (!good(l) || !n.mkt.has.test(l.stdout)) throw stepFailed(a, "marketplace add", n.mkt.add, codeOf(add));
      }
    }
    await steps(d, a, n.install);
    const probe = await identity(d, a, "install");
    s.put(a, { method: "native", id: n.id, scope: probe.scope, probe, version: s.tag!, updated_at: now(s) });
    return row(a, "installed", s.tag!);
  }
  if (s.dry) {
    if (layout(d, a).some((l) => existsSync(l.dst) && marker(l.dst) === null)) return hand(a);
    return planned(s, a, "install");
  }
  const src = await s.src();
  const dirs = layout(d, a, src);
  if (dirs.some((l) => existsSync(l.dst) && marker(l.dst) === null)) return hand(a);
  for (const l of dirs) copyTo(d, l.src, l.dst, s.tag!);
  s.put(a, { method: "copy", targets: dirs.map((l) => l.dst), version: s.tag!, updated_at: now(s) });
  return row(a, "installed", s.tag!);
}

/** `force`: an explicit command runs the native update even when the recorded tag is the latest. */
async function update(s: Session, a: Agent, force: boolean): Promise<Row> {
  const { d } = s;
  const reg = s.read().agents[a]!;
  if (isNative(a) && NATIVE[a].noIdentity) return refused(s, a, reg, `the ${a} list cannot show who owns the extension. Run "${cmdline(a, NATIVE[a].update[0][1])}" yourself`);
  const own = await owns(s, a, reg);
  if (!own.ok) return refused(s, a, reg, own.reason, own.drop);
  if (!isNative(a) && s.tag && !newer(s.tag, reg.version)) return row(a, "up_to_date", reg.version);
  if (isNative(a) && !force && s.tag && !newer(s.tag, reg.version)) return row(a, "up_to_date", reg.version);
  if (s.dry) return planned(s, a, "update", reg);
  if (isNative(a)) {
    await steps(d, a, NATIVE[a].update);
    const probe = await identity(d, a, "update");
    s.put(a, { ...reg, scope: probe.scope, probe, version: s.tag!, updated_at: now(s), last_error: undefined });
    return row(a, "updated", s.tag!);
  }
  const dirs = layout(d, a, await s.src());
  for (const l of dirs) copyTo(d, l.src, l.dst, s.tag!);
  // a dir of the last release that this one no longer has: ours (it held the marker), so it goes
  for (const t of reg.targets ?? []) if (!dirs.some((l) => l.dst === t)) rmSync(t, { recursive: true, force: true });
  s.put(a, { ...reg, targets: dirs.map((l) => l.dst), version: s.tag!, updated_at: now(s), last_error: undefined });
  return row(a, "updated", s.tag!);
}

async function uninstall(s: Session, a: Agent): Promise<Row> {
  const reg = s.read().agents[a]!;
  if (isNative(a) && NATIVE[a].noIdentity) return refused(s, a, reg, `the ${a} list cannot show who owns the extension. Run "${cmdline(a, NATIVE[a].uninstall[0][1])}" yourself`);
  const own = await owns(s, a, reg);
  if (!own.ok) return refused(s, a, reg, own.reason, own.drop);
  if (s.dry) return planned(s, a, "uninstall", reg);
  if (isNative(a)) await steps(s.d, a, NATIVE[a].uninstall);
  // the marketplace stays: other plugins may use it
  else for (const t of reg.targets ?? []) rmSync(t, { recursive: true, force: true });
  s.put(a, null);
  return row(a, "removed", reg.version);
}

/**
 * Read-only: a plugin that ai install did not register. The list probes are the ones of `install`; `"unknown"` = a probe failed.
 * shortcut: OpenCode has no probe that proves our plugin (the references folder is shared), so it is always "unknown", upgrade when its skill names are known without a download.
 */
async function byHand(d: Deps, a: Agent): Promise<boolean | "unknown"> {
  if (a === "opencode") return "unknown";
  if (a === "cursor") return layout(d, a).some((l) => existsSync(l.dst) && marker(l.dst) === null);
  const r = await sh(d, a, NATIVE[a].list);
  const found = good(r) ? NATIVE[a].find(r.stdout) : null;
  return found ? found.length > 0 : "unknown";
}

const chosen = (opt: string | undefined): Agent[] => {
  if (opt === undefined || opt === "all") return [...AGENTS];
  if (!(AGENTS as readonly string[]).includes(opt)) throw usage(`--agent must be one of ${AGENTS.join(", ")} or all.`);
  return [opt as Agent];
};
export const offline = (d: Deps) => d.env.PURCHASELY_OFFLINE === "1";

/** `ai install|update|uninstall|status`. */
export async function aiRun(rt: Runtime, g: Globals & { agent?: string }, verb: Verb): Promise<{ rows: object[]; errors: string[] }> {
  const { d } = rt;
  const s = new Session(d, false, !!g.dryRun);
  const agents = chosen(g.agent);
  if (verb === "status") {
    const p = s.read();
    const out = [];
    for (const a of agents) {
      const reg = p.agents[a];
      const detected = await detect(d, a);
      out.push({ agent: a, detected, managed: !!reg, installed_by_hand: !reg && detected ? await byHand(d, a) : false, version: reg?.version ?? null, follows: follows(a), last_error: reg?.last_error ?? null });
    }
    return { rows: out, errors: [] };
  }
  try {
    if ((verb === "install" || verb === "update") && !s.dry) {
      if (offline(d)) throw usage("PURCHASELY_OFFLINE=1 is set: the ai commands need the network.");
      s.ref = await latestTag(d);
      s.tag = s.ref.replace(/^v/, "");
    }
    let list = agents.filter((a) => verb === "install" || s.read().agents[a]);
    if (verb === "install") {
      const found: Agent[] = [];
      for (const a of list) if (await detect(d, a)) found.push(a);
      if (!found.length) throw usage(`No coding agent found. Supported agents: ${AGENTS.join(", ")}.`);
      list = found;
    }
    const rows: Row[] = [];
    for (const a of list) {
      const r = await s.guarded(a, () => (verb === "install" ? install(s, a) : verb === "update" ? update(s, a, true) : uninstall(s, a)));
      if (r) rows.push(r);
    }
    const bad = rows.filter((r) => r.action === "failed" || r.action === "refused");
    const done = (n: Agent) => rows.some((r) => r.agent === n && (r.action === "installed" || r.action === "updated"));
    if (!s.dry && done("claude")) d.stderr("Restart Claude Code to load the new plugin.\n");
    if (!s.dry && done("cursor")) d.stderr("Reload the Cursor window.\n");
    return { rows, errors: bad.map((r) => r.message!) };
  } finally {
    s.done();
  }
}

/** The daily step: update the agents that `ai install` registered, when a newer release exists. Silent in the background. */
export async function updatePlugins(rt: Runtime, o: { background: boolean }): Promise<void> {
  const { d } = rt;
  if (offline(d)) return;
  const s = new Session(d, o.background, false);
  if (!Object.keys(s.read().agents).length) return;
  try {
    if (o.background) {
      // reserve the day first, so two children never both check; on contention skip without reserving
      const go = await s.store.withLock(() => {
        const p = s.read();
        if (d.now() - (p.checked_at ?? 0) < DAY) return false;
        s.write({ ...p, checked_at: d.now() });
        return true;
      }, -1).catch(() => false);
      if (!go) return;
    }
    s.ref = await latestTag(d);
    s.tag = s.ref.replace(/^v/, "");
    await s.store.withLock(() => s.write({ ...s.read(), latest: s.tag, checked_at: d.now() }), o.background ? -1 : 300_000);
    for (const a of AGENTS) {
      const reg = s.read().agents[a];
      if (!reg || !newer(s.tag, reg.version) || !(await detect(d, a))) continue;
      const r = await s.guarded(a, () => update(s, a, false));
      if (r && !o.background) d.stderr(r.message ? r.message + "\n" : `${a}: plugin updated to ${r.version}\n`);
    }
  } finally {
    s.done();
  }
}
