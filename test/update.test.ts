import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { installKind, newer, noticeLine, stable } from "../src/core/update.js";
import { CliError, EXIT } from "../src/core/errors.js";
import type { Deps, ExecResult, Extension } from "../src/core/types.js";
import { run } from "../src/run.js";
import { EXT, setup, type Reply } from "./helpers.js";

const OPT_IN = { PURCHASELY_DISABLE_AUTOUPDATE: "" };
const DAY = 24 * 3600_000;
const ok = (stdout = "", code = 0, stderr = ""): ExecResult => ({ code, stdout, stderr, timedOut: false });

type Call = { file: string; args: string[]; opts: Parameters<Deps["exec"]>[2] };

/** Fake exec. `answer` replies per binary. A detached call runs the child in process, as the real child would. */
function fakes(answer: (c: Call) => ExecResult | Promise<ExecResult>, runChild = true) {
  const calls: Call[] = [];
  let d!: Deps;
  const exec: Deps["exec"] = async (file, args, opts) => {
    const c = { file, args, opts };
    calls.push(c);
    if (opts.detach) {
      if (runChild) await run(args.slice(1), d);
      return ok();
    }
    return answer(c);
  };
  return { calls, exec, bind: (x: Deps) => void (d = x), detached: () => calls.filter((c) => c.opts.detach) };
}
const gh = (latest: string) => (c: Call) => (c.file === "gh" ? ok(`v${latest}\n`) : ok());
// A program on its own update channel: the latest tag comes from `gh`, as the tests fake it.
const ACME: Extension = {
  ...EXT,
  npmUpdate: "npm install -g acme",
  latest: async (d) => {
    const r = await d.exec("gh", ["release", "view"], { timeoutMs: 15000 });
    if (r.code !== 0) throw new CliError(EXIT.AUTH, "UPDATE_CHECK_FAILED", ["could not check the latest version"]);
    return stable(r.stdout.trim().replace(/^v/, ""));
  },
};
const stateFile = (d: Deps) => join(d.stateDir, `update.${d.name}.check.json`);

describe("install kind", () => {
  it("tells managed, brew and npm", () => {
    expect(installKind({ PURCHASELY_MANAGED_INSTALL_ROOT: "/r" }, "file:///x/dist/core/update.js")).toBe("managed");
    expect(installKind({}, "file:///opt/homebrew/Cellar/purchasely/0.2.0/libexec/dist/core/update.js")).toBe("brew");
    expect(installKind({}, "file:///Users/k/cli/dist/core/update.js")).toBe("npm");
  });
});

describe("newer", () => {
  it("compares x.y.z numerically", () => {
    expect(newer("v0.2.0", "0.1.9")).toBe(true);
    expect(newer("0.1.0", "0.1.0")).toBe(false);
    expect(newer("0.1.0", "0.2.0")).toBe(false);
    expect(newer("0.10.0", "0.9.0")).toBe(true);
  });
});

describe("daily background check", () => {
  it("spawns one detached child a day and treats a generated update action as a command", async () => {
    const f = fakes(gh("9.9.9"));
    const t = setup({ name: "acme", ext: ACME, exec: f.exec, env: OPT_IN });
    f.bind(t.d);
    expect((await t.exec("usage")).code).toBe(0);
    await t.d.sleep(3600_000);
    await t.exec("usage");
    expect(f.detached()).toHaveLength(1);
    const child = f.detached()[0];
    expect(child.file).toBe(process.execPath);
    expect(child.args.slice(-2)).toEqual(["update", "--background"]);
    expect(child.opts.timeoutMs).toBe(0);
    await t.d.sleep(24 * 3600_000);
    await t.exec("usage");
    expect(f.detached()).toHaveLength(2);
    expect(t.calls).toHaveLength(0);
    await t.d.sleep(25 * 3600_000);
    await t.exec("placements", "update", "--app", "a", "--id", "b", "--data", "{}", "--env", "staging", "--dry-run");
    expect(f.detached()).toHaveLength(3);
  });

  it("does not spawn for the top-level update command", async () => {
    const f = fakes(gh("9.9.9"));
    const t = setup({ name: "acme", ext: ACME, exec: f.exec, env: OPT_IN });
    f.bind(t.d);
    expect((await t.exec("update")).code).toBe(0);
    expect(f.detached()).toHaveLength(0);
  });

  it("never fails a command: a state file that is not JSON, a held lock", async () => {
    const f = fakes(gh("9.9.9"), false);
    const t = setup({ name: "acme", ext: ACME, exec: f.exec, env: OPT_IN });
    f.bind(t.d);
    mkdirSync(t.d.stateDir, { recursive: true });
    writeFileSync(stateFile(t.d), "not json");
    writeFileSync(join(t.d.stateDir, "update.acme.lock"), String(process.pid));
    expect((await t.exec("usage")).code).toBe(0);
  });
});

describe("opt-outs", () => {
  const cases: [string, Record<string, string>, boolean][] = [
    ["CI", { CI: "1" }, false],
    ["PURCHASELY_DISABLE_AUTOUPDATE", { PURCHASELY_DISABLE_AUTOUPDATE: "1" }, false],
    ["PURCHASELY_OFFLINE", { PURCHASELY_OFFLINE: "1" }, false],
    ["config auto_update false", {}, true],
  ];
  for (const [name, env, cfg] of cases) {
    it(`${name} records no exec and writes no state`, async () => {
      const f = fakes(gh("9.9.9"));
      const t = setup({ name: "acme", ext: ACME, exec: f.exec, env: { ...OPT_IN, ...env } });
      f.bind(t.d);
      if (cfg) {
        mkdirSync(t.d.configDir, { recursive: true });
        writeFileSync(join(t.d.configDir, "config.json"), JSON.stringify({ auto_update: false }));
      }
      expect((await t.exec("usage")).code).toBe(0);
      expect(f.calls).toHaveLength(0);
      expect(existsSync(stateFile(t.d))).toBe(false);
    });
  }
});

describe("notice once a day", () => {
  it("notice matrix per program and install kind", () => {
    const cases: [Deps["name"], "brew" | "npm", string][] = [
      ["purchasely", "brew", "purchasely 10.0.0 is available: brew upgrade purchasely"],
      ["acme", "brew", "acme 10.0.0 is available: brew upgrade acme"],
      ["purchasely", "npm", "purchasely 10.0.0 is available: curl -fsSL https://cli.example.test/install.sh | sh"],
      ["acme", "npm", "acme 10.0.0 is available: npm install -g acme"],
    ];
    for (const [name, kind, line] of cases) expect(noticeLine({ name, host: "https://cli.example.test", ext: name === "acme" ? ACME : undefined } as Deps, kind, "10.0.0")).toBe(line);
  });

  const seed = (t: ReturnType<typeof setup>) => {
    mkdirSync(t.d.stateDir, { recursive: true });
    writeFileSync(stateFile(t.d), JSON.stringify({ latest: "10.0.0", checked_at: t.d.now() }));
  };

  it("prints the npm line once a day, never for a managed install", async () => {
    const t = setup({ env: OPT_IN });
    seed(t);
    const line = noticeLine(t.d, "npm", "10.0.0");
    expect((await t.exec("usage")).err.split(line).length - 1).toBe(1);
    expect((await t.exec("usage")).err).not.toContain(line);
    await t.d.sleep(DAY + 1);
    expect((await t.exec("usage")).err).toContain(line);

    const m = setup({ env: { ...OPT_IN, PURCHASELY_MANAGED_INSTALL_ROOT: "/r" } });
    seed(m);
    expect((await m.exec("usage")).err).not.toContain("is available");
  });

  it("keeps both fields when the child result and the parent notice both write", async () => {
    const f = fakes(gh("11.0.0"));
    const t = setup({ name: "acme", ext: ACME, exec: f.exec, env: OPT_IN });
    f.bind(t.d);
    mkdirSync(t.d.stateDir, { recursive: true });
    writeFileSync(stateFile(t.d), JSON.stringify({ latest: "10.0.0", checked_at: t.d.now() - DAY - 1 }));
    expect((await t.exec("usage")).err).toContain("10.0.0 is available");
    const rec = JSON.parse(readFileSync(stateFile(t.d), "utf8"));
    expect(rec).toEqual({ latest: "11.0.0", checked_at: t.d.now(), notified_at: t.d.now() });
  });
});

describe("managed update runs the installer", () => {
  it("passes the version, keep, root and upgrade-only to install.sh", async () => {
    const f = fakes((c) => (c.file === "gh" ? ok("v10.0.0\n") : ok()));
    const t = setup({ name: "acme", ext: ACME, exec: f.exec, env: { PURCHASELY_MANAGED_INSTALL_ROOT: "/r" } });
    f.bind(t.d);
    const r = await t.exec("update");
    expect(r.code).toBe(0);
    const sh = f.calls.find((c) => c.file === "sh")!;
    expect(sh.args).toEqual(["/r/releases/9.9.9/install.sh", "--version", "10.0.0", "--keep", "9.9.9", "--root", "/r", "--upgrade-only"]);
    expect(r.json()).toEqual({ kind: "managed", current: "9.9.9", latest: "10.0.0", action: "updated" });
    expect(t.calls).toHaveLength(0);
    expect(JSON.parse(readFileSync(stateFile(t.d), "utf8"))).toMatchObject({ latest: "10.0.0", checked_at: t.d.now() });
  });

  it("exits 1 when the installer fails", async () => {
    const f = fakes((c) => (c.file === "gh" ? ok("v10.0.0\n") : ok("", 1, "install failed\n")));
    const t = setup({ name: "acme", ext: ACME, exec: f.exec, env: { PURCHASELY_MANAGED_INSTALL_ROOT: "/r" } });
    f.bind(t.d);
    const r = await t.exec("update");
    expect(r.code).toBe(1);
    expect(r.err).toContain("install failed");
  });
});

describe("up to date", () => {
  it("runs no installer when the latest is the current", async () => {
    const f = fakes(gh("9.9.9"));
    const t = setup({ name: "acme", ext: ACME, exec: f.exec, env: { PURCHASELY_MANAGED_INSTALL_ROOT: "/r" } });
    f.bind(t.d);
    const r = await t.exec("update");
    expect(f.calls.some((c) => c.file === "sh")).toBe(false);
    expect(r.json().action).toBe("up_to_date");
  });

  it("gives a notice, and no install, for an npm install", async () => {
    const f = fakes(gh("10.0.0"));
    const t = setup({ name: "acme", ext: ACME, exec: f.exec });
    f.bind(t.d);
    const r = await t.exec("update");
    expect(f.calls.some((c) => c.file === "sh")).toBe(false);
    expect(r.json()).toEqual({ kind: "npm", current: "9.9.9", latest: "10.0.0", action: "notice" });
    expect(r.err).toContain(noticeLine(t.d, "npm", "10.0.0"));
  });

  it("treats a prerelease tag as no update and writes nothing", async () => {
    const f = fakes((c) => (c.file === "gh" ? ok("v10.0.0-rc1\n") : ok()));
    const t = setup({ name: "acme", ext: ACME, exec: f.exec, env: { PURCHASELY_MANAGED_INSTALL_ROOT: "/r" } });
    f.bind(t.d);
    const r = await t.exec("update");
    expect(r.json().action).toBe("up_to_date");
    expect(existsSync(stateFile(t.d))).toBe(false);
  });
});

describe("background is silent", () => {
  it("exits 0 with empty stdout when gh fails", async () => {
    const f = fakes((c) => (c.file === "gh" ? ok("", 1, "boom\n") : ok()));
    const t = setup({ name: "acme", ext: ACME, exec: f.exec, env: { PURCHASELY_MANAGED_INSTALL_ROOT: "/r" } });
    f.bind(t.d);
    const r = await t.exec("update", "--background");
    expect(r.code).toBe(0);
    expect(r.out).toBe("");
  });

  it("exits 0 when the installer fails and gives it a log file", async () => {
    const f = fakes((c) => (c.file === "gh" ? ok("v10.0.0\n") : ok("", 1, "x\n")));
    const t = setup({ name: "acme", ext: ACME, exec: f.exec, env: { PURCHASELY_MANAGED_INSTALL_ROOT: "/r" } });
    f.bind(t.d);
    const r = await t.exec("update", "--background");
    expect(r.code).toBe(0);
    expect(r.out).toBe("");
    const sh = f.calls.find((c) => c.file === "sh")!;
    expect(sh.args).toContain("--log");
    expect(sh.args[sh.args.indexOf("--log") + 1]).toBe(join(t.d.stateDir, "update.acme.log"));
  });

  it("skips when another child already reserved the day", async () => {
    const f = fakes(gh("10.0.0"));
    const t = setup({ name: "acme", ext: ACME, exec: f.exec, env: { PURCHASELY_MANAGED_INSTALL_ROOT: "/r" } });
    f.bind(t.d);
    mkdirSync(t.d.stateDir, { recursive: true });
    writeFileSync(stateFile(t.d), JSON.stringify({ checked_at: t.d.now() }));
    await t.exec("update", "--background");
    expect(f.calls).toHaveLength(0);
  });
});

describe("detach", () => {
  it("a blocked child does not delay the parent's exit", () => {
    // Compile first: Node 20 cannot import TypeScript source (same as limiter.test.ts).
    const dist = mkdtempSync(join(tmpdir(), "cli-dist-"));
    const tsc = join(process.cwd(), "node_modules/typescript/bin/tsc");
    execFileSync(process.execPath, [tsc, "-p", join(process.cwd(), "tsconfig.json"), "--outDir", dist], { stdio: "pipe" });
    writeFileSync(join(dist, "package.json"), '{"type":"module"}');
    const script = `
      const { realExec } = await import(${JSON.stringify(join(dist, "core/exec.js"))});
      const r = await realExec(process.execPath, ["-e", "setTimeout(() => {}, 20000)"], { timeoutMs: 0, detach: true });
      console.log(JSON.stringify(r));`;
    const start = Date.now();
    const p = spawnSync(process.execPath, ["--input-type=module", "-e", script], { encoding: "utf8", timeout: 10_000 });
    expect(p.status, p.stderr).toBe(0);
    expect(JSON.parse(p.stdout)).toEqual({ code: 0, stdout: "", stderr: "", timedOut: false });
    expect(Date.now() - start).toBeLessThan(5000);
  }, 60_000);
});

describe("public channel", () => {
  const latestJson = (reply: Reply) => (c: Call2) => (c.url === "https://cli.test/latest.json" ? reply : { body: {} });
  type Call2 = { url: string; init: RequestInit };

  it("purchasely reads latest.json over HTTPS", async () => {
    const f = fakes(() => ok());
    const t = setup({ exec: f.exec, env: { PURCHASELY_MANAGED_INSTALL_ROOT: "/r" }, replies: latestJson({ body: { version: "9.9.10", sha256: "a".repeat(64) } }) });
    f.bind(t.d);
    const r = await t.exec("update");
    expect(r.code).toBe(0);
    expect(t.calls.map((c) => c.url)).toEqual(["https://cli.test/latest.json"]);
    expect(f.calls.some((c) => c.file === "gh")).toBe(false);
    expect(f.calls.find((c) => c.file === "sh")!.args).toContain("9.9.10");
    expect(r.json()).toEqual({ kind: "managed", current: "9.9.9", latest: "9.9.10", action: "updated" });
  });

  it("purchasely: a bad latest.json is a fixed error", async () => {
    const MSG = "could not check the latest version. Check your network, then try again.";
    const bad: Reply[] = [{ status: 500, text: "SECRET_BODY" }, { text: "<html>SECRET_BODY" }, { throws: new Error("SECRET_BODY") }, { body: { sha256: "x" } }];
    for (const reply of bad) {
      const f = fakes(() => ok());
      const t = setup({ exec: f.exec, replies: () => reply });
      f.bind(t.d);
      const r = await t.exec("update");
      expect(r.code).toBe(1);
      expect(r.errJson().error.code).toBe("UPDATE_CHECK_FAILED");
      expect(r.err).toContain(MSG);
      expect(r.out + r.err).not.toContain("SECRET_BODY");
      expect(f.calls).toHaveLength(0);
    }
  });

  it("purchasely: a version that is not x.y.z is no release", async () => {
    const t = setup({ replies: () => ({ body: { version: "10.0.0-rc1" } }) });
    const r = await t.exec("update");
    expect(r.code).toBe(0);
    expect(r.json().action).toBe("up_to_date");
  });

  it("each program has its own update record", async () => {
    const f = fakes(gh("9.9.9"), false);
    const acme = setup({ name: "acme", ext: ACME, exec: f.exec });
    f.bind(acme.d);
    await acme.exec("update", "--background");
    expect(existsSync(stateFile(acme.d))).toBe(true);
    // same stateDir, other program: its check is not reserved by the acme record
    const f2 = fakes(() => ok(), false);
    const pub = setup({ exec: f2.exec, env: { PURCHASELY_MANAGED_INSTALL_ROOT: "/r" }, replies: () => ({ body: { version: "10.0.0" } }) });
    Object.assign(pub.d, { stateDir: acme.d.stateDir, now: acme.d.now });
    f2.bind(pub.d);
    await pub.exec("update", "--background");
    expect(pub.calls).toHaveLength(1);
    expect(existsSync(stateFile(pub.d))).toBe(true);
    expect(stateFile(pub.d)).not.toBe(stateFile(acme.d));
  });

  it("spawns the running bundle", async () => {
    const f = fakes(() => ok(), false);
    const t = setup({ exec: f.exec, env: OPT_IN });
    f.bind(t.d);
    await t.exec("usage");
    expect(f.detached()[0].args).toEqual([process.argv[1], "update", "--background"]);
  });
});
