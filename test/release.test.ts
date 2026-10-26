import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { chmodSync, cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { beforeAll, describe, expect, it, vi } from "vitest";
import type { Command } from "commander";
import { Runtime } from "../src/core/runtime.js";
import { build as buildProgram } from "../src/run.js";
import { setup } from "./helpers.js";

vi.setConfig({ testTimeout: 180000, hookTimeout: 180000 });

const REPO = join(__dirname, "..");
const pkg = JSON.parse(readFileSync(join(REPO, "package.json"), "utf8")) as { version: string; config: { host: string } };
const VER = pkg.version;

// Fake curl: maps https://cli.test/<file> to $FAKE_SITE/<file>, exits 22 when the file is missing.
const FAKE_CURL = `#!/bin/sh
out=""; url=""; p=""
for a in "$@"; do
  [ "$p" = "-o" ] && out=$a
  p=$a; url=$a
done
f=\${url#https://cli.test/}
[ -f "$FAKE_SITE/$f" ] || exit 22
cp "$FAKE_SITE/$f" "$out"
`;

/** Every node of the public command tree, as a path of names (same walk as parity.test.ts). */
const nodePaths = (c: Command, path: string[] = []): string[][] =>
  c.createHelp().visibleCommands(c).filter((x) => x.name() !== "help").flatMap((x) => [[...path, x.name()], ...nodePaths(x, [...path, x.name()])]);

let tmp: string;
let out: string;
let extracted: string | undefined;

function run(cmd: string, args: string[], opts: { cwd?: string; env?: NodeJS.ProcessEnv } = {}) {
  const r = spawnSync(cmd, args, { cwd: opts.cwd ?? REPO, env: opts.env ?? process.env, encoding: "utf8", input: "" });
  return { status: r.status, out: r.stdout, err: r.stderr };
}

const sha = (file: string) => createHash("sha256").update(readFileSync(file)).digest("hex");
const filesOf = (tgz: string) => run("tar", ["-tzf", tgz]).out.split("\n").filter((l) => l && !l.endsWith("/"));

function walk(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? walk(join(dir, e.name)) : [join(dir, e.name)]));
}

/** The public tarball extracted once; returns the top directory. */
function publicTree(): string {
  if (!extracted) {
    extracted = join(tmp, "extract");
    mkdirSync(extracted);
    const r = run("tar", ["-xzf", join(out, `purchasely-${VER}.tgz`), "-C", extracted]);
    if (r.status !== 0) throw new Error(`tar failed: ${r.err}`);
  }
  return join(extracted, `purchasely-${VER}`);
}

/** A PATH where `security` fails (no real keychain read) and node is found. */
function sandbox(name: string, extra: Record<string, string> = {}): NodeJS.ProcessEnv {
  const root = join(tmp, name);
  const fakebin = join(root, "fakebin");
  mkdirSync(fakebin, { recursive: true });
  for (const [file, body] of Object.entries({ security: "#!/bin/sh\nexit 44\n", curl: FAKE_CURL })) {
    writeFileSync(join(fakebin, file), body);
    chmodSync(join(fakebin, file), 0o755);
  }
  mkdirSync(join(root, "tmp"), { recursive: true });
  return {
    PATH: `${fakebin}:${dirname(process.execPath)}:/usr/bin:/bin`,
    HOME: join(root, "home"),
    XDG_CONFIG_HOME: join(root, "config"),
    XDG_STATE_HOME: join(root, "state"),
    XDG_CACHE_HOME: join(root, "cache"),
    XDG_DATA_HOME: join(root, "xdg"),
    PURCHASELY_BIN_DIR: join(root, "bin"),
    PURCHASELY_DISABLE_AUTOUPDATE: "1",
    TMPDIR: join(root, "tmp"),
    ...extra,
  };
}

beforeAll(() => {
  tmp = mkdtempSync(join(tmpdir(), "cli-release-"));
  out = join(tmp, "out");
  const build = run("npm", ["run", "build"]);
  if (build.status !== 0) throw new Error(`build failed: ${build.out}${build.err}`);
  const pack = run("sh", ["scripts/pack-release.sh", out]);
  if (pack.status !== 0) throw new Error(`pack failed: ${pack.out}${pack.err}`);
});

describe("pack-release.sh", () => {
  it("public tarball holds only the public files", () => {
    const top = `purchasely-${VER}`;
    expect(filesOf(join(out, `purchasely-${VER}.tgz`)).sort()).toEqual(
      ["LICENSE", "README.md", "dist/bin/purchasely.js", "install.sh", "man/purchasely.1", "package.json", "spec/management.json"].map((f) => `${top}/${f}`).sort(),
    );
    const minimal = JSON.parse(readFileSync(join(publicTree(), "package.json"), "utf8")) as Record<string, unknown>;
    expect(Object.keys(minimal).sort()).toEqual(["config", "name", "private", "type", "version"]);
    expect(minimal.version).toBe(VER);
    expect(minimal.config).toEqual({ host: pkg.config.host });
    expect(readFileSync(join(publicTree(), "README.md"), "utf8")).toBe(readFileSync(join(REPO, "README.md"), "utf8"));
  });


  it("public artifact runs", () => {
    const bin = join(publicTree(), "dist", "bin", "purchasely.js");
    const env = sandbox("artifact", { PURCHASELY_API_KEY: "dummy_key" });
    expect(run(process.execPath, [bin, "--version"], { env, cwd: tmp }).out.trim()).toBe(VER);
    const dry = run(process.execPath, [bin, "applications", "list", "--dry-run"], { env, cwd: tmp });
    expect(dry.status, dry.err).toBe(0);
  });


  it("site holds the landing page and the release files", () => {
    const site = out;
    expect(existsSync(join(site, "index.html"))).toBe(true);
    expect(statSync(join(site, "assets")).isDirectory()).toBe(true);
    expect(walk(site).filter((f) => f.endsWith(".md"))).toEqual([]);
    expect(readdirSync(site).sort()).toEqual(
      ["404.html", "SHA256SUMS", "_headers", "assets", "index.html", "install.sh", "latest.json", "recipes.html", "reference.html", "robots.txt", `purchasely-${VER}.tgz`].sort(),
    );
    const install = readFileSync(join(site, "install.sh"), "utf8");
    expect(install).toMatch(/^NAME=purchasely$/m);
    expect(install).toContain(`\nHOST=${pkg.config.host}\n`);
    const tgz = `purchasely-${VER}.tgz`;
    expect(readFileSync(join(site, "SHA256SUMS"), "utf8")).toBe(`${sha(join(site, tgz))}  ${tgz}\n`);
    expect(JSON.parse(readFileSync(join(site, "latest.json"), "utf8"))).toEqual({ version: VER, sha256: sha(join(site, tgz)) });
    const headers = readFileSync(join(site, "_headers"), "utf8");
    for (const f of ["/install.sh", "/latest.json", "/SHA256SUMS"]) expect(headers).toContain(`${f}\n  Cache-Control: no-cache`);
  });


  it("pack refuses a site _headers", () => {
    const copy = join(tmp, "repo-copy");
    mkdirSync(copy);
    for (const f of ["scripts", "site", "dist", "package.json", "install.sh"]) cpSync(join(REPO, f), join(copy, f), { recursive: true });
    writeFileSync(join(copy, "site", "_headers"), "/x\n  X: y\n");
    const r = run("sh", ["scripts/pack-release.sh", join(tmp, "refused")], { cwd: copy });
    expect(r.status).not.toBe(0);
    expect(r.err).toContain("site/_headers");
  });


  it("public install from the packed site", () => {
    const site = out;
    const file = join(tmp, "install-public.sh");
    writeFileSync(file, readFileSync(join(site, "install.sh"), "utf8").replace(/^HOST=.*$/m, "HOST=https://cli.test"));
    const env = sandbox("public-install", { FAKE_SITE: site });
    const r = run("sh", [file], { env });
    expect(r.status, r.out + r.err).toBe(0);
    expect(existsSync(join(env.XDG_DATA_HOME!, "purchasely", "install", "releases", VER, "dist", "bin", "purchasely.js"))).toBe(true);
    expect(run(join(env.PURCHASELY_BIN_DIR!, "purchasely"), ["--version"], { env }).out.trim()).toBe(VER);
  });
});

describe("reference", () => {
  const tarText = (tgz: string, path: string) => run("tar", ["-xzOf", tgz, path]).out;

  it("reference page and man page come from the command tree", () => {
    const page = readFileSync(join(out, "reference.html"), "utf8");
    for (const p of nodePaths(buildProgram(new Runtime(setup().d)))) expect(page, p.join(" ")).toContain(`id="${p.join("-")}"`);
    expect(page).not.toContain('id="extension-commands"');
    const pub = tarText(join(out, `purchasely-${VER}.tgz`), `purchasely-${VER}/man/purchasely.1`);
    expect(pub).toMatch(/^\.TH PURCHASELY 1 \d{4}-\d{2}-\d{2} "purchasely /);
  });
});
