import { execFileSync } from "node:child_process";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { realExec } from "../src/core/exec.js";
import type { Deps, ExecResult } from "../src/core/types.js";
import { run } from "../src/run.js";
import { setup, type Call, type Reply } from "./helpers.js";

const DAY = 24 * 3600_000;
const API = "https://api.github.com/repos/Purchasely/Purchasely-AI-Plugin/releases/latest";
const TARBALL = (tag: string) => `https://codeload.github.com/Purchasely/Purchasely-AI-Plugin/tar.gz/refs/tags/${tag}`;
const REPO = "Purchasely/Purchasely-AI-Plugin";
const SENTINEL = "SECRET_SENTINEL_123";

/** One fake agent binary per contract of the local `--help` (docs of section 3.0). `step` sets S or exits 2. */
interface Fake { entry: string; empty: string; mkt: string; step: string[] }
const argvOnly = (cases: string[]) => ["case \"$1 $2 $3\" in", ...cases, " *) echo \"bad argv\" >&2; exit 2;;", "esac"];
const FAKES: Record<string, Fake> = {
  claude: {
    entry: '[{"id":"purchasely@Purchasely-AI-Plugin","scope":"user","installedAt":"2026-10-01T00:00:00Z","version":"1.0.0","installPath":"/x/1.0.0"}]',
    empty: "[]",
    mkt: '[{"name":"Purchasely-AI-Plugin"}]',
    step: [
      ...argvOnly([
        ' "plugin list --json") S=LIST;;',
        ' "plugin marketplace list") S=MKT_LIST;;',
        ' "plugin marketplace add") S=MKT_ADD;;',
        ' "plugin marketplace update") S=MKT_UPDATE;;',
        ' "plugin install purchasely@Purchasely-AI-Plugin") S=INSTALL;;',
        ' "plugin update purchasely@Purchasely-AI-Plugin") S=UPDATE;;',
        ' "plugin uninstall purchasely@Purchasely-AI-Plugin") S=UNINSTALL;;',
      ]),
      // every mutation of the plugin names the user scope
      'case "$S" in INSTALL|UPDATE|UNINSTALL) case " $* " in *" -s user "*) ;; *) exit 2;; esac;; esac',
    ],
  },
  codex: {
    entry: '{"installed":[{"pluginId":"purchasely@purchasely","name":"purchasely","marketplaceName":"purchasely","version":"1.0.0","installed":true}]}',
    empty: '{"installed":[]}',
    mkt: '{"marketplaces":[{"name":"purchasely"}]}',
    step: [
      ...argvOnly([
        ' "plugin list --json") S=LIST; case " $* " in *" -m purchasely "*) ;; *) exit 2;; esac;;',
        ' "plugin marketplace list") S=MKT_LIST;;',
        ' "plugin marketplace add") S=MKT_ADD;;',
        ' "plugin marketplace upgrade") S=MKT_UPGRADE;;',
        ' "plugin add purchasely@purchasely") S=INSTALL;;',
        ' "plugin remove purchasely@purchasely") S=UNINSTALL;;',
      ]),
    ],
  },
  copilot: {
    entry: '[{"name":"purchasely","marketplace":"Purchasely-AI-Plugin","version":"1.0.0","enabled":true,"source":"installed"}]',
    empty: "[]",
    mkt: "Registered marketplaces:\n  • Purchasely-AI-Plugin (GitHub: Purchasely/Purchasely-AI-Plugin)",
    step: argvOnly([
      ' "plugin list --json") S=LIST;;',
      ' "plugin marketplace list") S=MKT_LIST;;',
      ' "plugin marketplace add") S=MKT_ADD;;',
      ' "plugin install purchasely@Purchasely-AI-Plugin") S=INSTALL;;',
      ' "plugin update purchasely@Purchasely-AI-Plugin") S=UPDATE;;',
      ' "plugin uninstall purchasely@Purchasely-AI-Plugin") S=UNINSTALL;;',
    ]),
  },
  gemini: {
    entry: "✓ purchasely (1.0.0)\n Path: /x/purchasely",
    empty: "No extensions installed.",
    mkt: "",
    step: [
      'case "$1 $2" in',
      ' "extensions list") S=LIST;;',
      // stdin is closed: the install needs --consent
      ' "extensions install") case " $* " in *" --consent "*) ;; *) exit 1;; esac; [ "$3" = "https://github.com/Purchasely/Purchasely-AI-Plugin" ] || exit 2; S=INSTALL;;',
      ' "extensions update") [ "$3" = purchasely ] || exit 2; S=UPDATE;;',
      ' "extensions uninstall") [ "$3" = purchasely ] || exit 2; S=UNINSTALL;;',
      ' *) exit 2;;',
      "esac",
    ],
  },
};

function script(name: string, f: Fake): string {
  const U = name.toUpperCase();
  return [
    "#!/bin/sh",
    `echo "${name} $*" >> "$FAKE_DIR/${name}.log"`,
    'if [ "$1" = "--version" ]; then echo 1.0.0; exit 0; fi',
    ...f.step,
    `eval "R=\\$FAKE_${U}_EXIT_$S"`,
    `if [ -n "$R" ] && [ "$R" != 0 ]; then if [ -n "$FAKE_SENTINEL" ]; then echo "$FAKE_SENTINEL"; echo "$FAKE_SENTINEL" >&2; fi; exit "$R"; fi`,
    'case "$S" in',
    ` LIST) if [ -n "$FAKE_${U}_LIST" ]; then echo "$FAKE_${U}_LIST"; elif [ -f "$FAKE_DIR/${name}.installed" ]; then echo "$FAKE_${U}_ENTRY"; else echo '${f.empty}'; fi;;`,
    ` MKT_LIST) if [ -f "$FAKE_DIR/${name}.mkt" ] || [ -n "$FAKE_${U}_MKT" ]; then echo '${f.mkt}'; else echo '[]'; fi;;`,
    ` MKT_ADD) touch "$FAKE_DIR/${name}.mkt";;`,
    ` INSTALL) touch "$FAKE_DIR/${name}.installed";;`,
    ` UNINSTALL) rm -f "$FAKE_DIR/${name}.installed";;`,
    "esac",
    "exit 0",
  ].join("\n") + "\n";
}

/** The release tarball of GitHub, per version: top dir `Purchasely-AI-Plugin-<v>/`. */
const tgz: Record<string, Uint8Array> = {};
beforeAll(() => {
  const src = mkdtempSync(join(tmpdir(), "cli-ai-src-"));
  for (const v of ["2.2.0", "2.3.0"]) {
    const top = join(src, `Purchasely-AI-Plugin-${v}`, "purchasely");
    mkdirSync(join(top, "skills", "purchasely-integrate"), { recursive: true });
    mkdirSync(join(top, "references"), { recursive: true });
    writeFileSync(join(top, "skills", "purchasely-integrate", "SKILL.md"), "See [versions](../../references/sdk-versions.md)\n");
    writeFileSync(join(top, "references", "sdk-versions.md"), `${v}\n`);
    const file = join(src, `${v}.tgz`);
    execFileSync("tar", ["-czf", file, "-C", src, `Purchasely-AI-Plugin-${v}`]);
    tgz[v] = readFileSync(file);
  }
});
afterEach(() => vi.unstubAllEnvs());

interface WorldOpts { agents?: string[]; installed?: string[]; cursor?: boolean; name?: string; latest?: string; env?: Record<string, string>; replies?: (c: Call) => Reply | undefined }

/** Fake agents on the PATH, a HOME, fake fetch routed by URL. `installed` agents already hold the plugin. */
function world(o: WorldOpts = {}) {
  const dir = mkdtempSync(join(tmpdir(), "cli-ai-bin-"));
  const home = mkdtempSync(join(tmpdir(), "cli-ai-home-"));
  for (const a of o.agents ?? []) {
    writeFileSync(join(dir, a), a === "opencode" ? '#!/bin/sh\necho "opencode $*" >> "$FAKE_DIR/opencode.log"\necho 1.0.0\n' : script(a, FAKES[a]));
    chmodSync(join(dir, a), 0o755);
  }
  for (const a of o.installed ?? []) writeFileSync(join(dir, `${a}.installed`), "");
  if (o.cursor) mkdirSync(join(home, ".cursor"), { recursive: true });
  vi.stubEnv("PATH", dir + ":/usr/bin:/bin");
  vi.stubEnv("FAKE_DIR", dir);
  vi.stubEnv("FAKE_SENTINEL", "");
  for (const [k, f] of Object.entries(FAKES)) vi.stubEnv(`FAKE_${k.toUpperCase()}_ENTRY`, f.entry);
  const rel = { latest: o.latest ?? "2.2.0" };
  const t = setup({
    name: o.name,
    exec: realExec,
    env: { HOME: home, ...o.env },
    replies: (c) =>
      o.replies?.(c) ??
      (c.url === API ? { body: { tag_name: rel.latest } }
        : c.url.startsWith("https://codeload.github.com/") ? { bytes: tgz[c.url.split("/").pop()!] }
        : c.url === "https://cli.test/latest.json" ? { body: { version: "9.9.9" } }
        : { status: 404 }),
  });
  const log = (a: string): string[] => (existsSync(join(dir, `${a}.log`)) ? readFileSync(join(dir, `${a}.log`), "utf8").trim().split("\n") : []);
  /** The lines of an agent log that change something: not `--version`, not a list probe. */
  const muts = (a: string) => log(a).filter((l) => !/ --version$| list( --json)?( -m purchasely)?$/.test(l) && !/marketplace list/.test(l));
  const stateFile = join(t.d.stateDir, "ai.plugin.json");
  const state = () => JSON.parse(readFileSync(stateFile, "utf8")) as { checked_at?: number; latest?: string; agents: Record<string, any> };
  const gh = () => t.calls.filter((c) => /api\.github\.com|codeload\.github\.com/.test(c.url));
  return { ...t, rel, dir, home, log, muts, state, stateFile, gh };
}
type World = ReturnType<typeof world>;
const readdirNames = (d: string) => readdirSync(d).sort();
const rows = (r: { json: () => any }): any[] => r.json().agents;
const by = (rs: any[], a: string) => rs.find((x) => x.agent === a);

/** An `ai.plugin.json` that registers the given agents, written the way a past `ai install` left it. */
function register(w: World, agents: Record<string, object>, extra: object = {}) {
  mkdirSync(w.d.stateDir, { recursive: true });
  writeFileSync(w.stateFile, JSON.stringify({ latest: "2.1.1", agents, ...extra }));
}
const CLAUDE_REC = {
  method: "native", id: "purchasely@Purchasely-AI-Plugin", scope: "user", version: "2.1.1", updated_at: 1,
  probe: { id: "purchasely@Purchasely-AI-Plugin", scope: "user", installedAt: "2026-10-01T00:00:00Z" },
};

describe("ai", () => {
  it("install runs the native commands of each detected agent", async () => {
    const w = world({ agents: ["claude", "codex", "copilot", "gemini"] });
    const r = await w.exec("ai", "install");
    expect(r.code, r.err).toBe(0);
    expect(rows(r).map((x) => [x.agent, x.action, x.version])).toEqual([
      ["claude", "installed", "2.2.0"], ["codex", "installed", "2.2.0"], ["copilot", "installed", "2.2.0"], ["gemini", "installed", "2.2.0"],
    ]);
    expect(w.muts("claude")).toEqual([`claude plugin marketplace add ${REPO}`, "claude plugin install purchasely@Purchasely-AI-Plugin -s user"]);
    expect(w.muts("codex")).toEqual([`codex plugin marketplace add ${REPO}`, "codex plugin add purchasely@purchasely"]);
    expect(w.muts("copilot")).toEqual([`copilot plugin marketplace add ${REPO}`, "copilot plugin install purchasely@Purchasely-AI-Plugin"]);
    expect(w.muts("gemini")).toEqual([`gemini extensions install https://github.com/${REPO} --consent`]);
    const s = w.state();
    expect(s.agents.claude).toMatchObject({
      method: "native", id: "purchasely@Purchasely-AI-Plugin", scope: "user", version: "2.2.0",
      probe: { id: "purchasely@Purchasely-AI-Plugin", scope: "user", installedAt: "2026-10-01T00:00:00Z" },
    });
    expect(s.agents.codex).toMatchObject({ id: "purchasely@purchasely", probe: { pluginId: "purchasely@purchasely", marketplaceName: "purchasely" } });
    expect(s.agents.copilot.probe).toEqual({ name: "purchasely", marketplace: "Purchasely-AI-Plugin" });
    expect(s.agents.gemini).toMatchObject({ method: "native", probe: { name: "purchasely" } });
    expect(r.err).toContain("Restart Claude Code to load the new plugin.");
  });

  it("install with no detected agent is a usage error", async () => {
    const r = await world().exec("ai", "install");
    expect(r.code).toBe(2);
    for (const a of ["claude", "codex", "copilot", "gemini", "cursor", "opencode"]) expect(r.errJson().error.messages.join(" ")).toContain(a);
  });

  it("a failed marketplace add needs a registered marketplace", async () => {
    const withMkt = world({ agents: ["claude"], env: {} });
    vi.stubEnv("FAKE_CLAUDE_EXIT_MKT_ADD", "1");
    vi.stubEnv("FAKE_CLAUDE_MKT", "1");
    const ok = await withMkt.exec("ai", "install");
    expect(ok.code, ok.err).toBe(0);
    expect(by(rows(ok), "claude").action).toBe("installed");
    expect(withMkt.muts("claude").at(-1)).toBe("claude plugin install purchasely@Purchasely-AI-Plugin -s user");

    const none = world({ agents: ["claude"] });
    vi.stubEnv("FAKE_CLAUDE_MKT", "");
    const bad = await none.exec("ai", "install");
    expect(bad.code).toBe(1);
    expect(by(rows(bad), "claude").action).toBe("failed");
    expect(bad.errJson().error.messages).toEqual([`claude: the plugin marketplace add failed (exit 1). Run "claude plugin marketplace add ${REPO}" yourself to see why.`]);
    expect(none.muts("claude")).toEqual([`claude plugin marketplace add ${REPO}`]);
  });

  it("a v-prefixed release tag keeps the raw tag in the tarball URL and strips it for the version", async () => {
    const w = world({
      agents: ["opencode"],
      replies: (c) => (c.url === API ? { body: { tag_name: "v2.3.0" } } : c.url === TARBALL("v2.3.0") ? { bytes: tgz["2.3.0"] } : undefined),
    });
    const r = await w.exec("ai", "install");
    expect(r.code, r.err).toBe(0);
    expect(w.gh().map((c) => c.url)).toContain(TARBALL("v2.3.0"));
    expect(w.state().agents.opencode.version).toBe("2.3.0");
  });

  it("copy agents keep the skill references", async () => {
    const w = world({ agents: ["opencode"], cursor: true });
    const r = await w.exec("ai", "install");
    expect(r.code, r.err).toBe(0);
    expect(rows(r).map((x) => [x.agent, x.action])).toEqual([["cursor", "installed"], ["opencode", "installed"]]);
    const skills = [
      join(w.home, ".cursor/plugins/local/purchasely/skills/purchasely-integrate/SKILL.md"),
      join(w.home, ".config/opencode/skills/purchasely-integrate/SKILL.md"),
    ];
    for (const s of skills) {
      expect(existsSync(s), s).toBe(true);
      // lexical, no realpath: this is how an agent resolves the link
      expect(existsSync(resolve(dirname(s), "../../references/sdk-versions.md")), s).toBe(true);
    }
    for (const m of [".cursor/plugins/local/purchasely", ".config/opencode/skills/purchasely-integrate", ".config/opencode/references"]) {
      expect(readFileSync(join(w.home, m, ".purchasely-cli"), "utf8").trim()).toBe("2.2.0");
    }
    expect(w.calls.map((c) => c.url)).toEqual([API, TARBALL("2.2.0")]);
    expect(w.calls[0].init.headers["User-Agent"]).toBe("purchasely-cli/9.9.9");
    expect(w.state().agents.cursor).toMatchObject({ method: "copy", version: "2.2.0", targets: [join(w.home, ".cursor/plugins/local/purchasely")] });
    expect(r.err).toContain("Reload the Cursor window.");
  });

  it("a plugin installed by hand is never touched", async () => {
    const w = world({ agents: ["claude", "opencode"], cursor: true });
    vi.stubEnv("FAKE_CLAUDE_LIST", JSON.stringify([{ id: "purchasely@Purchasely-AI-Plugin", scope: "project", installedAt: "x" }]));
    const cursor = join(w.home, ".cursor/plugins/local/purchasely");
    const refs = join(w.home, ".config/opencode/references");
    for (const d of [cursor, refs]) {
      mkdirSync(d, { recursive: true });
      writeFileSync(join(d, "mine.txt"), "hand");
    }
    const r = await w.exec("ai", "install");
    expect(r.code, r.err).toBe(0);
    expect(rows(r).map((x) => [x.agent, x.action])).toEqual([["claude", "installed_by_hand"], ["cursor", "installed_by_hand"], ["opencode", "installed_by_hand"]]);
    expect(w.muts("claude")).toEqual([]);
    expect(readdirNames(cursor)).toEqual(["mine.txt"]);
    expect(readdirNames(refs)).toEqual(["mine.txt"]);
    expect(existsSync(join(w.home, ".config/opencode/skills"))).toBe(false);
    expect(existsSync(w.stateFile) ? w.state().agents : {}).toEqual({});
    const before = w.gh().length;
    const bg = await w.exec("update", "--background");
    expect(bg.code).toBe(0);
    expect(w.gh()).toHaveLength(before);
    expect(w.muts("claude")).toEqual([]);
    expect(readdirNames(cursor)).toEqual(["mine.txt"]);
  });

  it("update and uninstall re-check ownership", async () => {
    const two = JSON.stringify([
      { id: "purchasely@Purchasely-AI-Plugin", scope: "user", installedAt: "2026-10-01T00:00:00Z" },
      { id: "purchasely@Purchasely-AI-Plugin", scope: "project", installedAt: "2026-10-02T00:00:00Z" },
    ]);
    const cases: [string, () => void][] = [
      ["replaced", () => vi.stubEnv("FAKE_CLAUDE_ENTRY", FAKES.claude.entry.replace("2026-10-01", "2026-10-09"))],
      ["mixed scope", () => vi.stubEnv("FAKE_CLAUDE_LIST", two)],
      ["probe failure", () => vi.stubEnv("FAKE_CLAUDE_EXIT_LIST", "1")],
      ["gone", () => undefined],
    ];
    for (const verb of ["update", "uninstall"]) {
      for (const [name, tamper] of cases) {
        vi.unstubAllEnvs();
        const w = world({ agents: ["claude"] });
        expect((await w.exec("ai", "install")).code).toBe(0);
        const before = w.muts("claude").length;
        if (name === "gone") rmSync(join(w.dir, "claude.installed"));
        tamper();
        const r = await w.exec("ai", verb);
        const label = `${verb} ${name}`;
        expect(r.code, label).toBe(1);
        expect(by(rows(r), "claude").action, label).toBe("refused");
        expect(w.muts("claude").length, label).toBe(before);
        if (name === "gone") expect(w.state().agents.claude, label).toBeUndefined();
        else expect(w.state().agents.claude.last_error, label).toContain("claude:");
      }
    }
    for (const verb of ["update", "uninstall"]) {
      const w = world({ cursor: true });
      expect((await w.exec("ai", "install")).code).toBe(0);
      const target = join(w.home, ".cursor/plugins/local/purchasely");
      writeFileSync(join(target, ".purchasely-cli"), "1.0.0\n");
      const r = await w.exec("ai", verb);
      expect(r.code, verb).toBe(1);
      expect(by(rows(r), "cursor").action, verb).toBe("refused");
      expect(existsSync(join(target, "skills/purchasely-integrate/SKILL.md")), verb).toBe(true);
    }
  });

  it("copy agents update to a newer release and uninstall", async () => {
    const w = world({ cursor: true });
    expect((await w.exec("ai", "install")).code).toBe(0);
    const target = join(w.home, ".cursor/plugins/local/purchasely");
    w.rel.latest = "2.3.0";
    expect((await w.exec("update", "--background")).code).toBe(0);
    expect(w.state().agents.cursor.version).toBe("2.3.0");
    expect(readFileSync(join(target, ".purchasely-cli"), "utf8").trim()).toBe("2.3.0");
    expect(readFileSync(join(target, "references/sdk-versions.md"), "utf8").trim()).toBe("2.3.0");
    expect(existsSync(target + ".old-" + process.pid)).toBe(false);
    const r = await w.exec("ai", "uninstall");
    expect(r.code, r.err).toBe(0);
    expect(by(rows(r), "cursor").action).toBe("removed");
    expect(existsSync(target)).toBe(false);
    expect(w.state().agents.cursor).toBeUndefined();
  });

  it("status lists detected and managed agents", async () => {
    const w = world({ agents: ["claude", "codex"] });
    register(w, { claude: { ...CLAUDE_REC, last_error: "claude: the plugin update failed (exit 1). Run \"x\" yourself to see why." } });
    const r = await w.exec("ai", "status");
    expect(r.code, r.err).toBe(0);
    expect(w.gh()).toHaveLength(0);
    const rs = rows(r);
    expect(rs.map((x) => x.agent)).toEqual(["claude", "codex", "copilot", "gemini", "cursor", "opencode"]);
    expect(by(rs, "claude")).toMatchObject({ detected: true, managed: true, version: "2.1.1", follows: "default branch", last_error: expect.stringContaining("claude:") });
    expect(by(rs, "codex")).toMatchObject({ detected: true, managed: false, version: null });
    expect(by(rs, "gemini")).toMatchObject({ detected: false, managed: false });
    expect(by(rs, "cursor").follows).toBe("release tag");
    expect(by(rs, "opencode").follows).toBe("release tag");
  });

  it("status shows a plugin installed by hand, per agent, and changes nothing", async () => {
    const w = world({ agents: ["claude", "codex", "opencode"], installed: ["claude"] });
    mkdirSync(join(w.home, ".config/opencode/references"), { recursive: true }); // the shared folder proves nothing
    const r = await w.exec("ai", "status");
    expect(r.code, r.err).toBe(0);
    const rs = rows(r);
    expect(by(rs, "claude")).toMatchObject({ detected: true, managed: false, installed_by_hand: true });
    expect(by(rs, "codex")).toMatchObject({ detected: true, managed: false, installed_by_hand: false });
    expect(by(rs, "gemini")).toMatchObject({ detected: false, installed_by_hand: false });
    expect(by(rs, "opencode").installed_by_hand).toBe("unknown");
    expect(w.muts("claude")).toEqual([]);
    expect(existsSync(w.stateFile)).toBe(false);
    expect(w.gh()).toHaveLength(0);
  });

  it("status: an agent that ai install registered is not by hand, and a failed probe is unknown", async () => {
    const w = world({ agents: ["claude", "codex"], installed: ["claude", "codex"] });
    register(w, { claude: CLAUDE_REC });
    vi.stubEnv("FAKE_CODEX_EXIT_LIST", "1");
    const rs = rows(await w.exec("ai", "status"));
    expect(by(rs, "claude")).toMatchObject({ managed: true, installed_by_hand: false });
    expect(by(rs, "codex")).toMatchObject({ managed: false, installed_by_hand: "unknown" });
  });

  it("status finds a Cursor directory without the marker", async () => {
    const w = world({ cursor: true });
    expect(by(rows(await w.exec("ai", "status")), "cursor").installed_by_hand).toBe(false);
    const dir = join(w.home, ".cursor/plugins/local/purchasely");
    mkdirSync(dir, { recursive: true });
    expect(by(rows(await w.exec("ai", "status")), "cursor").installed_by_hand).toBe(true);
    writeFileSync(join(dir, ".purchasely-cli"), "2.2.0\n");
    expect(by(rows(await w.exec("ai", "status")), "cursor").installed_by_hand).toBe(false);
  });

  it("dry-run changes nothing", async () => {
    const outs: string[] = [];
    for (const name of ["purchasely", "acme"] as const) {
      const w = world({ name, agents: ["claude"], cursor: true });
      register(w, { claude: CLAUDE_REC });
      writeFileSync(join(w.dir, "claude.installed"), "");
      const stateBefore = readFileSync(w.stateFile, "utf8");
      const logBefore = w.log("claude");
      let out = "";
      for (const verb of ["install", "update", "uninstall"]) {
        const r = await w.exec("ai", verb, "--dry-run");
        expect(r.code, `${name} ${verb}: ${r.err}`).toBe(0);
        const rs = rows(r);
        expect(rs.length).toBeGreaterThan(0);
        for (const x of rs) expect(x.action).toBe("planned");
        for (const x of rs) expect(x.ops.length).toBeGreaterThan(0);
        out += r.out.split(w.home).join("<home>");
      }
      expect(w.calls).toHaveLength(0);
      expect(w.log("claude").slice(logBefore.length).every((l) => / --version$| list --json$/.test(l))).toBe(true);
      expect(readFileSync(w.stateFile, "utf8")).toBe(stateBefore);
      expect(existsSync(join(w.home, ".cursor/plugins"))).toBe(false);
      outs.push(out);
    }
    expect(outs[1]).toBe(outs[0]);
    expect(outs[0]).toContain("fetch https://codeload.github.com/Purchasely/Purchasely-AI-Plugin/tar.gz/refs/tags/2.1.1");
  });

  it("daily update runs only for registered agents", async () => {
    const w = world({ agents: ["claude", "gemini"], installed: ["claude"], replies: (c) => (c.url === "https://cli.test/latest.json" ? { status: 500 } : undefined) });
    register(w, { claude: CLAUDE_REC }, { checked_at: w.d.now() - 2 * DAY });
    const r = await w.exec("update", "--background");
    expect(r.code).toBe(0);
    expect(r.out).toBe("");
    expect(r.err).not.toContain("claude");
    expect(w.muts("claude")).toEqual([`claude plugin marketplace update Purchasely-AI-Plugin`, "claude plugin update purchasely@Purchasely-AI-Plugin -s user"]);
    expect(w.log("gemini")).toEqual([]);
    expect(w.state().agents.claude.version).toBe("2.2.0");
    const calls = w.calls.length;
    await w.exec("update", "--background");
    expect(w.calls).toHaveLength(calls);

    // the CLI day is reserved, the plugin day is not: the plugin step still runs
    const w2 = world({ agents: ["claude"], installed: ["claude"] });
    register(w2, { claude: CLAUDE_REC }, { checked_at: w2.d.now() - 2 * DAY });
    mkdirSync(w2.d.stateDir, { recursive: true });
    writeFileSync(join(w2.d.stateDir, `update.${w2.d.name}.check.json`), JSON.stringify({ checked_at: w2.d.now() }));
    await w2.exec("update", "--background");
    expect(w2.muts("claude")).toHaveLength(2);
    expect(w2.state().agents.claude.version).toBe("2.2.0");
  });

  it("the parent spawns when only the plugin is due", async () => {
    const spawned: string[][] = [];
    const exec: Deps["exec"] = async (file, args, opts): Promise<ExecResult> => {
      if (opts.detach) {
        spawned.push(args);
        return { code: 0, stdout: "", stderr: "", timedOut: false };
      }
      return realExec(file, args, opts);
    };
    const w = world({ env: { PURCHASELY_DISABLE_AUTOUPDATE: "" } });
    const d = { ...w.d, exec };
    const now = w.d.now();
    mkdirSync(w.d.stateDir, { recursive: true });
    writeFileSync(join(w.d.stateDir, "update.purchasely.check.json"), JSON.stringify({ checked_at: now }));
    const go = async (...argv: string[]) => {
      spawned.length = 0;
      expect(await run(argv, d)).toBe(0);
      return spawned.length;
    };
    register(w, { claude: CLAUDE_REC }, { checked_at: now - 2 * DAY });
    expect(await go("usage")).toBe(1);
    expect(spawned[0].slice(-2)).toEqual(["update", "--background"]);
    // no spawn for the ai group itself: it does the work in the foreground
    expect(await go("ai", "status")).toBe(0);
    register(w, { claude: CLAUDE_REC }, { checked_at: now });
    expect(await go("usage")).toBe(0);
    register(w, {}, { checked_at: now - 2 * DAY });
    expect(await go("usage")).toBe(0);
  });

  it("no network when PURCHASELY_OFFLINE=1", async () => {
    const w = world({ agents: ["claude"], installed: ["claude"], env: { PURCHASELY_OFFLINE: "1" } });
    for (const verb of ["install", "update"]) {
      const r = await w.exec("ai", verb);
      expect(r.code, verb).toBe(2);
      expect(r.errJson().error.messages).toEqual(["PURCHASELY_OFFLINE=1 is set: the ai commands need the network."]);
    }
    expect(w.calls).toHaveLength(0);
    expect(w.log("claude")).toEqual([]);
    register(w, { claude: CLAUDE_REC }, { checked_at: 0 });
    expect((await w.exec("ai", "status")).code).toBe(0);
    expect(w.calls).toHaveLength(0);
    const bg = await w.exec("update", "--background");
    const fg = await w.exec("update");
    expect([bg.code, fg.code]).toEqual([0, 0]);
    expect(w.gh()).toHaveLength(0);
    expect(w.muts("claude")).toEqual([]);
    // uninstall works offline
    const un = await w.exec("ai", "uninstall");
    expect(un.code, un.err).toBe(0);
    expect(w.muts("claude")).toEqual(["claude plugin uninstall purchasely@Purchasely-AI-Plugin -s user"]);
  });

  it("agent errors use fixed messages", async () => {
    const w = world({ agents: ["claude"] });
    vi.stubEnv("FAKE_SENTINEL", SENTINEL);
    vi.stubEnv("FAKE_CLAUDE_EXIT_INSTALL", "1");
    const r = await w.exec("ai", "install");
    expect(r.code).toBe(1);
    expect(r.errJson().error.messages).toEqual(['claude: the plugin install failed (exit 1). Run "claude plugin install purchasely@Purchasely-AI-Plugin -s user" yourself to see why.']);
    for (const s of [r.out, r.err, existsSync(w.stateFile) ? readFileSync(w.stateFile, "utf8") : ""]) expect(s).not.toContain(SENTINEL);

    // the daily step records the same fixed message and stays silent
    const d = world({ agents: ["claude"], installed: ["claude"] });
    register(d, { claude: CLAUDE_REC }, { checked_at: 0 });
    vi.stubEnv("FAKE_SENTINEL", SENTINEL);
    vi.stubEnv("FAKE_CLAUDE_EXIT_UPDATE", "1");
    const bg = await d.exec("update", "--background");
    expect(bg.out).toBe("");
    expect(bg.err).not.toContain("claude");
    expect(d.state().agents.claude.last_error).toBe('claude: the plugin update failed (exit 1). Run "claude plugin update purchasely@Purchasely-AI-Plugin -s user" yourself to see why.');
    expect(readFileSync(d.stateFile, "utf8")).not.toContain(SENTINEL);
    const log = join(d.d.stateDir, "update.purchasely.log");
    if (existsSync(log)) expect(readFileSync(log, "utf8")).not.toContain(SENTINEL);
  });

  it("a bad release tag is refused", async () => {
    for (const tag of ["../x", "2.2.0;rm"]) {
      const w = world({ agents: ["claude"], cursor: true, latest: tag });
      const r = await w.exec("ai", "install");
      expect(r.code, tag).toBe(1);
      expect(r.errJson().error.messages).toEqual(["could not read the latest plugin release. Check your network."]);
      expect(w.calls.map((c) => c.url)).toEqual([API]);
      expect(w.log("claude")).toEqual([]);
    }
  });

  it("gemini update and uninstall name the native command (B7)", async () => {
    const w = world({ agents: ["gemini"] });
    expect((await w.exec("ai", "install")).code).toBe(0);
    for (const [verb, cmd] of [["update", "gemini extensions update purchasely"], ["uninstall", "gemini extensions uninstall purchasely"]]) {
      const before = w.muts("gemini").length;
      const r = await w.exec("ai", verb);
      expect(r.code, verb).toBe(1);
      expect(by(rows(r), "gemini").action).toBe("refused");
      expect(r.errJson().error.messages.join(" ")).toContain(cmd);
      expect(w.muts("gemini")).toHaveLength(before);
    }
  });
});

