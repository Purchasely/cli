import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { defineSurface } from "../src/core/credentials.js";
import type { Deps, Extension, Keychain } from "../src/core/types.js";
import { run } from "../src/run.js";

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
  const exec = async (...argv: string[]) => {
    out.length = 0;
    err.length = 0;
    const code = await run(argv, d);
    return { code, out: out.join(""), err: err.join(""), json: () => JSON.parse(out.join("")), errJson: () => JSON.parse(err.join("").trim().split("\n").pop()!) };
  };
  return { d, exec, calls, keychain, root, prompts, tick: () => t };
}
