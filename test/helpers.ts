import { lstatSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, readlinkSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { defineSurface } from "../src/core/credentials.js";
import type { Deps, Extension, Keychain } from "../src/core/types.js";
import { run } from "../src/run.js";
import { Command } from "commander";
import { afterEach, expect, vi } from "vitest";

const recordedSteps: Record<string, unknown>[] = [];
const contractRecordDir = process.env.CONTRACT_RECORD;
if (contractRecordDir) {
  afterEach(() => {
    if (!recordedSteps.length) return;
    const state = expect.getState();
    const testPath = state.testPath ?? "test/unknown.test.ts";
    const stem = testPath.split(/[\\/]/).pop()!.replace(/\.test\.ts$/, "");
    mkdirSync(contractRecordDir, { recursive: true });
    const destination = join(contractRecordDir, `${stem}.json`);
    const existing = exists(destination) ? JSON.parse(readFileSync(destination, "utf8")) as { cases?: unknown[] } : { cases: [] };
    const cases = existing.cases ?? [];
    const recorded = { test: `${stem}::${state.currentTestName ?? "unknown"}`, steps: recordedSteps.splice(0) };
    const old = cases.findIndex((item) => (item as { test?: string }).test === recorded.test);
    if (old === -1) cases.push(recorded);
    else cases[old] = recorded;
    writeFileSync(destination, `${JSON.stringify({ cases }, null, 2)}\n`);
  });
}

export interface Reply {
  status?: number;
  body?: unknown;
  text?: string;
  /** Binary body (a tarball). Wins over `text` and `body`. */
  bytes?: Uint8Array;
  headers?: Record<string, string>;
  throws?: Error;
}

export interface Call {
  at: number;
  url: string;
  init: RequestInit & { headers: Record<string, string> };
}

export class FakeKeychain implements Keychain {
  store = new Map<string, string>();
  fail = false;
  async get(s: string, a: string) {
    return this.store.get(`${s}/${a}`) ?? null;
  }
  async set(s: string, a: string, secret: string) {
    if (this.fail) throw new Error("no keychain");
    this.store.set(`${s}/${a}`, secret);
  }
  async delete(s: string, a: string) {
    this.store.delete(`${s}/${a}`);
  }
}

export const KEY = "pk_mgmt_live_SECRET123";
// A program built on `purchasely` for the tests of the seam: `acme`, with one group on its own surface.
export const EXTRA_TOKEN = "extra_tok_SECRET9012";
defineSurface("extra", { envVar: "ACME_EXTRA_TOKEN", what: "Extra token", hint: () => "Run `acme extra login` or set ACME_EXTRA_TOKEN.", rate: { default: 5, cap: 8 } });
export const EXT: Extension = {
  modules: [(p) => void p.command("extra").description("Added commands").command("run").description("Run it").action(() => undefined)],
  surface: "extra",
  identity: true,
  section: { title: "More commands", note: "Only in `acme`." },
};

const noExec: Deps["exec"] = async () => {
  throw Object.assign(new Error("spawn ENOENT"), { code: "ENOENT" });
};

export function setup(opts: { exec?: Deps["exec"]; replies?: Reply[] | ((c: Call, i: number) => Reply); env?: Record<string, string>; isTTY?: boolean; answers?: string[]; stdin?: string; name?: string; ext?: Extension } = {}) {
  const root = mkdtempSync(join(tmpdir(), "purchasely-cli-"));
  const out: string[] = [];
  const err: string[] = [];
  const calls: Call[] = [];
  const keychain = new FakeKeychain();
  let t = 1_700_000_000_000;
  const answers = [...(opts.answers ?? [])];
  const prompts: string[] = [];
  const reply = (c: Call, i: number): Reply =>
    typeof opts.replies === "function" ? opts.replies(c, i) : (opts.replies ?? [])[Math.min(i, (opts.replies ?? []).length - 1)] ?? { body: {} };
  const d: Deps = {
    fetch: (async (url: string, init: RequestInit) => {
      const c = { at: t, url: String(url), init: init as Call["init"] };
      calls.push(c);
      const r = reply(c, calls.length - 1);
      if (r.throws) throw r.throws;
      const text = r.text ?? JSON.stringify(r.body ?? {});
      return new Response(r.bytes ?? text, { status: r.status ?? 200, headers: r.headers });
    }) as typeof fetch,
    env: { PURCHASELY_API_KEY: KEY, PURCHASELY_DISABLE_AUTOUPDATE: "1", ...opts.env },
    stdout: (s) => void out.push(s),
    stderr: (s) => void err.push(s),
    isTTY: opts.isTTY ?? false,
    prompt: async (q) => {
      prompts.push(q);
      return answers.shift() ?? "";
    },
    readStdin: async () => opts.stdin ?? "",
    configDir: join(root, "config"),
    stateDir: join(root, "state"),
    cacheDir: join(root, "cache"),
    now: () => t,
    sleep: async (ms) => void (t += ms),
    keychain,
    exec: opts.exec ?? noExec,
    version: "9.9.9",
    pid: process.pid,
    name: opts.name ?? "purchasely",
    host: "https://cli.test",
    ext: opts.ext,
  };
  const recordDir = process.env.CONTRACT_RECORD;
  const exec = async (...argv: string[]) => {
    out.length = 0;
    err.length = 0;
    const code = recordDir ? await recordedStep(argv) : await run(argv, d);
    return { code, out: out.join(""), err: err.join(""), json: () => JSON.parse(out.join("")), errJson: () => JSON.parse(err.join("").trim().split("\n").pop()!) };
  };
  async function recordedStep(argv: string[]): Promise<number> {
    const before = new Map<string, unknown>();
    const roots = snapshotRoots(argv, d.env, root);
    for (const dir of roots) before.set(dir, snapshot(dir));
    const callsBefore = calls.length;
    const promptsBefore = prompts.length;
    const tBefore = d.now();
    const wall = (vi.getMockedSystemTime() ?? new Date("2026-01-15T12:00:00Z")).toISOString();
    const original = { fetch: d.fetch, exec: d.exec, prompt: d.prompt, now: d.now, sleep: d.sleep, keychain: d.keychain, ext: d.ext };
    const fetchCalls: unknown[] = [];
    const execCalls: unknown[] = [];
    const promptCalls: unknown[] = [];
    const keyOps: unknown[] = [];
    const pidProbes: unknown[] = [];
    let latestData: unknown = null;
    let fakeWall = Date.parse(wall);
    const hadFakeTimers = vi.isFakeTimers();
    const previousSystemTime = vi.getMockedSystemTime();
    if (!hadFakeTimers) vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(fakeWall);
    d.fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
      const call = { at: d.now(), url: String(input), method: init?.method ?? "GET", headers: init?.headers ?? {}, body: init?.body ?? null, refuse_redirect: init?.redirect === "manual" };
      try {
        const response = await original.fetch(input, init);
        const clone = response.clone();
        const binary = call.url.includes("/tar.gz/");
        if (binary) {
          const bytes = new Uint8Array(await clone.arrayBuffer());
          // The gzip header timestamp comes from tar(1)'s source file mtime and is not part of the fixture.
          if (bytes.length >= 8 && bytes[0] === 0x1f && bytes[1] === 0x8b) bytes.fill(0, 4, 8);
          const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
          fetchCalls.push({ ...call, reply: { status: response.status, headers: Object.fromEntries(response.headers), hex } });
        } else {
          const text = await clone.text();
          fetchCalls.push({ ...call, reply: { status: response.status, headers: Object.fromEntries(response.headers), text } });
        }
        return response;
      } catch (error) {
        const causeCode = error && typeof error === "object" && "cause" in error
          ? (error as { cause?: { code?: unknown } }).cause?.code
          : undefined;
        fetchCalls.push({
          ...call,
          reply: {
            error: typeof causeCode === "string" ? causeCode : error instanceof Error ? error.message : String(error),
            ...(typeof causeCode === "string" ? { code: causeCode } : {}),
          },
        });
        throw error;
      }
    }) as typeof d.fetch;
    d.exec = async (file, args, execOpts) => {
      const self = file === process.execPath && args[0] === process.argv[1];
      const rec: Record<string, unknown> = { file: self ? "<SELF>" : file, args: self ? args.slice(1) : args, opts: execOpts };
      execCalls.push(rec);
      // A test fake may run the detached child in process, or not: the child writes its status line, so a
      // change of the output or of the calls tells that it ran.
      const seen = () => out.length + err.length + fetchCalls.length + execCalls.length;
      const seenBefore = seen();
      try {
        const result = await original.exec(file, args, execOpts);
        rec.result = result;
        if (self && execOpts.detach && seen() !== seenBefore) rec.run_child = true;
        return result;
      } catch (error) {
        rec.error = error instanceof Error ? (error as NodeJS.ErrnoException).code ?? error.message : String(error);
        throw error;
      }
    };
    if (original.ext?.latest) {
      const latest = original.ext.latest;
      d.ext = { ...original.ext, latest: async (deps) => {
        const start = execCalls.length;
        try {
          const result = await latest(deps);
          latestData = { execs: execCalls.slice(start), result };
          return result;
        } catch (error) {
          // A failed channel is still a channel: record it, or the replay falls back to latest.json.
          latestData = { execs: execCalls.slice(start), error: error instanceof Error ? error.message : String(error) };
          throw error;
        }
      } };
    }
    d.prompt = async (question, promptOpts) => {
      const answer = await original.prompt(question, promptOpts);
      promptCalls.push({ question, hidden: promptOpts?.hidden ?? false, answer });
      return answer;
    };
    d.now = original.now;
    d.sleep = async (ms) => {
      await original.sleep(ms);
      fakeWall += ms;
      vi.setSystemTime(fakeWall);
    };
    const keychain = d.keychain;
    const initialKeychain = keychain && "store" in keychain ? Object.fromEntries((keychain as FakeKeychain).store) : {};
    if (keychain) {
      const proxy = new Proxy(keychain, { get(target, prop, receiver) {
        const value = Reflect.get(target, prop, receiver);
        if (!["get", "set", "delete"].includes(String(prop)) || typeof value !== "function") return value;
        return async (...args: unknown[]) => {
          try { const result = await value.apply(target, args); keyOps.push({ op: String(prop), args, result: result ?? null }); return result; }
          catch (error) { keyOps.push({ op: String(prop), args, error: error instanceof Error ? error.message : String(error) }); throw error; }
        };
      } });
      d.keychain = proxy;
    }
    // Keep the native function available to the spy's call-through implementation.
    const realKill = process.kill.bind(process);
    const killSpy = vi.spyOn(process, "kill").mockImplementation(((pid: number, signal?: number | NodeJS.Signals) => {
      try { const result = realKill(pid, signal as NodeJS.Signals); pidProbes.push({ pid: pid === process.pid ? "<PID>" : pid, alive: true }); return result; }
      catch (error) { pidProbes.push({ pid: pid === process.pid ? "<PID>" : pid, alive: false }); throw error; }
    }) as typeof process.kill);
    let stepCode = 0;
    try {
      stepCode = await run(argv, d);
      return stepCode;
    } finally {
      const after = new Map<string, unknown>();
      for (const dir of roots) after.set(dir, snapshot(dir));
      const filesBefore = Object.fromEntries([...before].map(([dir, value]) => [dirToken(dir, roots, root), value]));
      const filesAfter = Object.fromEntries([...after].map(([dir, value]) => [dirToken(dir, roots, root), value]));
      recordedSteps.push({ argv, env: { ...d.env }, is_tty: d.isTTY, stdin: opts.stdin ?? "", answers: opts.answers ?? [], name: d.name, pid: process.pid, version: d.version, host: d.host, platform: process.platform, stdout_tty: process.stdout.isTTY, columns: process.stdout.columns ?? null, ext: extensionData(opts.ext, latestData), t_before: tBefore, t_after: d.now(), wall, pid_probes: pidProbes, files_before: filesBefore, files_after: filesAfter, keychain: keychain ? { before: initialKeychain, after: keychain && "store" in keychain ? Object.fromEntries((keychain as FakeKeychain).store) : {}, ops: keyOps } : null, calls: fetchCalls, execs: execCalls, prompts: promptCalls, code: stepCode, out: out.join(""), err: err.join("") });
      Object.assign(d, original);
      killSpy.mockRestore();
      if (hadFakeTimers) {
        if (previousSystemTime) vi.setSystemTime(previousSystemTime);
      } else {
        vi.useRealTimers();
      }
    }
  }
  return { d, exec, calls, keychain, root, prompts, tick: () => t };
}

function extensionData(ext?: Extension, latest: unknown = null) {
  if (!ext) return null;
  return { surfaces: ext.surfaces ?? {}, identity: ext.identity ?? false, section: ext.section ?? null, npm_update: ext.npmUpdate ?? null, latest, commands: extensionCommands(ext) };
}

/** The commands that the extension modules add, so a replay can build the same tree. null when a module needs the runtime. */
function extensionCommands(ext: Extension): unknown {
  type Raw = Command & { _hidden?: boolean; _actionHandler?: unknown };
  const shape = (c: Raw): unknown => ({
    name: c.name(), description: c.description(), hidden: Boolean(c._hidden), action: Boolean(c._actionHandler),
    options: c.options.map((o) => ({ flags: o.flags, description: o.description, hidden: o.hidden })),
    commands: c.commands.map((s) => shape(s as Raw)),
  });
  const program = new Command();
  try {
    for (const m of ext.modules ?? []) m(program, {} as never);
  } catch {
    return null;
  }
  return program.commands.map((c) => shape(c as Raw));
}

function snapshotRoots(argv: string[], env: Record<string, string | undefined>, root: string): string[] {
  const roots = new Set<string>([root]);
  for (const raw of Object.values(env)) {
    if (!raw) continue;
    if (isAbsolute(raw)) {
      const candidate = resolve(raw);
      roots.add(exists(candidate) && statSync(candidate).isDirectory() ? candidate : dirname(candidate));
    }
  }
  for (const token of argv) {
    const value = token.startsWith("@") ? token.slice(1) : token;
    if (isAbsolute(value)) roots.add(dirname(resolve(value)));
  }
  // Only the temporary directories of the tests: an env path like /usr/bin/git must not snapshot /usr/bin.
  const tmp = resolve(tmpdir());
  return [...roots].filter((r) => r === root || r.startsWith(tmp + "/"));
}

function snapshot(dir: string): unknown {
  if (!exists(dir)) return [];
  const entries: unknown[] = [];
  const walk = (current: string) => {
    for (const name of readdirSync(current).sort()) {
      const path = join(current, name);
      const st = lstatSync(path);
      const item: Record<string, unknown> = { path: relative(dir, path), mode: st.mode & 0o777, directory: st.isDirectory() };
      if (st.isSymbolicLink()) item.link = readlinkSync(path);
      else if (st.isDirectory()) walk(path);
      else { const bytes = readFileSync(path); item.text = bytes.toString("utf8"); item.hex = bytes.toString("hex"); }
      entries.push(item);
    }
  };
  walk(dir);
  return entries;
}

function exists(path: string): boolean { try { statSync(path); return true; } catch { return false; } }
function dirToken(dir: string, roots: string[], root: string): string { return dir === root ? "<ROOT>" : `<TMP${roots.indexOf(dir) - 1}>`; }
