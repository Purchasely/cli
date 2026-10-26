import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  chmodSync,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  readlinkSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { afterAll, beforeEach, describe, expect, it, vi } from "vitest";

vi.setConfig({ testTimeout: 60000 });

const INSTALL_SH = join(__dirname, "..", "install.sh");
const SECRET = "SECRET_SENTINEL_123";

// Fake curl: maps https://cli.test/<file> to $FAKE_SITE/<file>, exits 22 when the file is missing.
const FAKE_CURL = `#!/bin/sh
echo "$*" >> "$CURL_LOG"
out=""; url=""; p=""
for a in "$@"; do
  [ "$p" = "-o" ] && out=$a
  p=$a; url=$a
done
if [ -n "\${FAKE_CURL_STDERR:-}" ]; then
  printf '%s\\n' "\${FAKE_CURL_STDERR}" >&2
  exit 22
fi
f=\${url#https://cli.test/}
[ -f "$FAKE_SITE/$f" ] || exit 22
cp "$FAKE_SITE/$f" "$out"
`;

interface Env {
  tmp: string;
  home: string;
  xdg: string;
  bindir: string;
  root: string;
  fakebin: string;
  assets: string;
  curlLog: string;
  /** install.sh with the test host, as pack-release.sh writes it. */
  script: string;
  env: Record<string, string>;
}

const cleanups: string[] = [];
let t: Env;

function sha256(file: string) {
  return createHash("sha256").update(readFileSync(file)).digest("hex");
}

/** Build the release assets of one version under <assets>/<ver>/ with real tar. */
function makeRelease(assets: string, ver: string, opts: { reported?: string; badSums?: boolean; name?: string; man?: string } = {}) {
  const name = opts.name ?? "purchasely";
  const work = mkdtempSync(join(tmpdir(), "cli-rel-"));
  cleanups.push(work);
  const top = join(work, `${name}-${ver}`);
  mkdirSync(join(top, "dist", "bin"), { recursive: true });
  writeFileSync(join(top, "package.json"), JSON.stringify({ name: "x", version: ver }));
  const reported = opts.reported ?? ver;
  writeFileSync(
    join(top, "dist", "bin", `${name}.js`),
    `if (process.argv[2] === "--version") console.log(${JSON.stringify(reported)});\n` +
      `else console.log("MANAGED=" + process.env.PURCHASELY_MANAGED_INSTALL_ROOT + " AUTOUPDATE=" + (process.env.PURCHASELY_DISABLE_AUTOUPDATE ?? ""));\n`,
  );
  if (opts.man !== undefined) {
    mkdirSync(join(top, "man"));
    writeFileSync(join(top, "man", `${name}.1`), opts.man);
  }
  const dir = join(assets, ver);
  mkdirSync(dir, { recursive: true });
  const tgz = join(dir, `${name}-${ver}.tgz`);
  const r = spawnSync("tar", ["-czf", tgz, "-C", work, `${name}-${ver}`]);
  if (r.status !== 0) throw new Error("tar failed");
  const sum = opts.badSums ? "0".repeat(64) : sha256(tgz);
  writeFileSync(join(dir, "SHA256SUMS"), `${sum}  ${name}-${ver}.tgz\n`);
  // The host holds one version: the site of this version says it is the latest.
  writeFileSync(join(dir, "latest.json"), JSON.stringify({ version: ver, sha256: "0".repeat(64) }));
  return dir;
}

/** The site serves the version that `--version` asks for as the latest one, so a test can install any version. */
const siteFor = (args: string[], e: Env = t) => {
  const i = args.indexOf("--version");
  return i >= 0 && /^[0-9]+\.[0-9]+\.[0-9]+$/.test(args[i + 1] ?? "") ? join(e.assets, args[i + 1]) : e.env.FAKE_SITE;
};

function install(args: string[] = [], extra: Record<string, string> = {}, e: Env = t) {
  const r = spawnSync("sh", [e.script, ...args], { env: { ...e.env, FAKE_SITE: siteFor(args, e), ...extra }, encoding: "utf8", input: "" });
  return { status: r.status, out: r.stdout, err: r.stderr, all: `${r.stdout}${r.stderr}` };
}

const curlLines = (e: Env = t) => (existsSync(e.curlLog) ? readFileSync(e.curlLog, "utf8").trim().split("\n") : []);
const downloads = (e: Env = t) => curlLines(e).filter((l) => l.includes(".tgz"));
const releases = (e: Env = t) => (existsSync(join(e.root, "releases")) ? readdirSync(join(e.root, "releases")).sort() : []);
const current = (e: Env = t) => readFileSync(join(e.root, "current-version"), "utf8").trim();

function launcher(args: string[], e: Env = t) {
  const r = spawnSync(join(e.bindir, "purchasely"), args, { env: e.env, encoding: "utf8" });
  return { status: r.status, out: r.stdout.trim(), err: r.stderr };
}

beforeEach(() => {
  const tmp = mkdtempSync(join(tmpdir(), "cli-install-"));
  cleanups.push(tmp);
  const home = join(tmp, "home");
  const xdg = join(tmp, "xdg");
  const bindir = join(tmp, "bin");
  const fakebin = join(tmp, "fakebin");
  const assets = join(tmp, "assets");
  mkdirSync(join(tmp, "tmp"), { recursive: true });
  mkdirSync(home, { recursive: true });
  mkdirSync(fakebin);
  mkdirSync(assets);
  writeFileSync(join(fakebin, "curl"), FAKE_CURL);
  chmodSync(join(fakebin, "curl"), 0o755);
  writeDarwinSha256sum(fakebin);
  const curlLog = join(tmp, "curl.log");
  const script = join(tmp, "install.sh");
  writeFileSync(script, readFileSync(INSTALL_SH, "utf8").replace(/^HOST=$/m, "HOST=https://cli.test"));
  t = {
    tmp,
    home,
    xdg,
    bindir,
    root: join(xdg, "purchasely", "install"),
    fakebin,
    assets,
    curlLog,
    script,
    env: {
      PATH: `${fakebin}:${dirname(process.execPath)}:/usr/bin:/bin`,
      HOME: home,
      XDG_DATA_HOME: xdg,
      PURCHASELY_BIN_DIR: bindir,
      TMPDIR: join(tmp, "tmp"),
      CURL_LOG: curlLog,
      FAKE_SITE: join(assets, "1.2.3"),
    },
  };
});

afterAll(() => {
  // tmp dirs live in the OS temp dir; nothing else to release
});

describe("install.sh", () => {
  it("fresh install: stages, switches, links and runs", () => {
    makeRelease(t.assets, "1.2.3");
    const r = install();
    expect(r.status, r.all).toBe(0);
    expect(current()).toBe("1.2.3");
    expect(existsSync(join(t.root, "releases", "1.2.3", "dist", "bin", "purchasely.js"))).toBe(true);
    const link = join(t.bindir, "purchasely");
    expect(lstatSync(link).isSymbolicLink()).toBe(true);
    expect(readlinkSync(link)).toBe(join(t.root, "bin", "purchasely"));
    expect(launcher(["--version"]).out).toBe("1.2.3");
    expect(launcher([]).out).toContain(`MANAGED=${t.root}`);
    expect(downloads()[0]).toContain("https://cli.test/purchasely-1.2.3.tgz");
    expect(curlLines()[0]).toContain("https://cli.test/latest.json");
    expect(r.out).toContain("PATH"); // bindir is not on the test PATH: the hint is printed
  });

  it("bad checksum: nothing is switched and nothing is left behind", () => {
    makeRelease(t.assets, "1.2.3", { badSums: true });
    const r = install();
    expect(r.status).toBe(1);
    expect(existsSync(join(t.root, "current-version"))).toBe(false);
    expect(releases()).toEqual([]);
    expect(readdirSync(join(t.root, "staging"))).toEqual([]);
  });

  it("smoke test fails: exit 1 and current-version is unchanged", () => {
    makeRelease(t.assets, "1.0.0");
    expect(install(["--version", "1.0.0"]).status).toBe(0);
    makeRelease(t.assets, "1.2.3", { reported: "0.0.0" });
    const r = install(["--version", "1.2.3"]);
    expect(r.status).toBe(1);
    expect(r.all).toContain("smoke test failed");
    expect(current()).toBe("1.0.0");
    expect(releases()).toEqual(["1.0.0"]);
  });

  it("already current: no download", () => {
    makeRelease(t.assets, "1.2.3");
    expect(install().status).toBe(0);
    const before = downloads().length;
    const r = install();
    expect(r.status).toBe(0);
    expect(r.out).toContain("up to date");
    expect(downloads().length).toBe(before);
  });

  it("update keeps running and previous: prunes to the 3 newest plus the current and --keep", () => {
    for (const v of ["1.0.0", "1.1.0", "1.2.0", "1.3.0"]) makeRelease(t.assets, v);
    for (const v of ["1.0.0", "1.1.0", "1.2.0"]) expect(install(["--version", v]).status).toBe(0);
    expect(releases()).toEqual(["1.0.0", "1.1.0", "1.2.0"]);
    expect(install(["--version", "1.3.0"]).status).toBe(0);
    expect(releases()).toEqual(["1.1.0", "1.2.0", "1.3.0"]);
    // Rollback to a present release: no download, the newer one stays.
    const n = downloads().length;
    expect(install(["--version", "1.2.0"]).status).toBe(0);
    expect(downloads().length).toBe(n);
    expect(current()).toBe("1.2.0");
    expect(releases()).toEqual(["1.1.0", "1.2.0", "1.3.0"]);
    // --keep protects a release that is not among the 3 newest.
    makeRelease(t.assets, "1.4.0");
    makeRelease(t.assets, "1.5.0");
    expect(install(["--version", "1.4.0", "--keep", "1.1.0"]).status).toBe(0);
    expect(releases()).toEqual(["1.1.0", "1.2.0", "1.3.0", "1.4.0"]);
    expect(install(["--version", "1.5.0", "--keep", "1.1.0"]).status).toBe(0);
    expect(releases()).toEqual(["1.1.0", "1.3.0", "1.4.0", "1.5.0"]);
  });

  it("update keeps running and previous: never prunes the current release, even when it is old", () => {
    for (const v of ["1.1.0", "1.2.0", "1.3.0", "1.4.0"]) makeRelease(t.assets, v);
    for (const v of ["1.2.0", "1.3.0", "1.4.0"]) expect(install(["--version", v]).status).toBe(0);
    expect(install(["--version", "1.1.0"]).status).toBe(0); // explicit rollback to an older release
    expect(current()).toBe("1.1.0");
    expect(releases()).toEqual(["1.1.0", "1.2.0", "1.3.0", "1.4.0"]);
  });

  it("lock held: a live pid blocks, a dead pid is taken over", () => {
    makeRelease(t.assets, "1.2.3");
    mkdirSync(join(t.root, "staging", "lock"), { recursive: true });
    writeFileSync(join(t.root, "staging", "lock", "pid"), `${process.pid}\n`);
    const r = install();
    expect(r.status).toBe(3);
    expect(downloads()).toEqual([]);
    expect(existsSync(join(t.root, "staging", "lock"))).toBe(true); // the holder's lock is untouched
    writeFileSync(join(t.root, "staging", "lock", "pid"), "999999\n");
    const r2 = install();
    expect(r2.status, r2.all).toBe(0);
    expect(existsSync(join(t.root, "staging", "lock"))).toBe(false);
  });

  it("upgrade-only: runs under the lock and never goes backwards", () => {
    for (const v of ["1.1.0", "1.2.0", "1.3.0"]) makeRelease(t.assets, v);
    expect(install(["--version", "1.2.0"]).status).toBe(0);
    const n = downloads().length;
    // B installed 1.2.0 first; A then runs upgrade-only with 1.1.0: nothing happens.
    const older = install(["--version", "1.1.0", "--upgrade-only"]);
    expect(older.status).toBe(0);
    expect(current()).toBe("1.2.0");
    expect(downloads().length).toBe(n);
    expect(releases()).toEqual(["1.2.0"]);
    // Same version: nothing happens either.
    expect(install(["--version", "1.2.0", "--upgrade-only"]).status).toBe(0);
    expect(downloads().length).toBe(n);
    // Explicit install may go back to an older version.
    expect(install(["--version", "1.1.0"]).status).toBe(0);
    expect(current()).toBe("1.1.0");
    // A newer version is installed.
    expect(install(["--version", "1.3.0", "--upgrade-only"]).status).toBe(0);
    expect(current()).toBe("1.3.0");
  });

  it("custom root: an update through the launcher keeps the custom root and does not move the symlink", () => {
    const root = join(t.tmp, "custom-root");
    const bindir = join(t.tmp, "custom-bin");
    makeRelease(t.assets, "1.0.0");
    makeRelease(t.assets, "1.1.0");
    const custom = { PURCHASELY_INSTALL_ROOT: root, PURCHASELY_BIN_DIR: bindir };
    expect(install(["--version", "1.0.0"], custom).status).toBe(0);
    const viaLauncher = spawnSync(join(bindir, "purchasely"), [], { env: t.env, encoding: "utf8" });
    const managed = /MANAGED=(\S+)/.exec(viaLauncher.stdout)?.[1];
    expect(managed).toBe(root);
    // The updater drops the overrides and passes the managed root it learned from the launcher.
    const dropped = { ...t.env };
    delete (dropped as Record<string, string | undefined>).PURCHASELY_INSTALL_ROOT;
    delete (dropped as Record<string, string | undefined>).PURCHASELY_BIN_DIR;
    const r = spawnSync("sh", [t.script, "--root", managed as string, "--version", "1.1.0", "--upgrade-only"], {
      env: { ...dropped, FAKE_SITE: join(t.assets, "1.1.0") },
      encoding: "utf8",
    });
    expect(r.status, r.stdout + r.stderr).toBe(0);
    expect(readFileSync(join(root, "current-version"), "utf8").trim()).toBe("1.1.0");
    expect(existsSync(join(t.root, "current-version"))).toBe(false); // the default root is untouched
    expect(existsSync(join(t.home, ".local", "bin", "purchasely"))).toBe(false);
    expect(readlinkSync(join(bindir, "purchasely"))).toBe(join(root, "bin", "purchasely"));
    expect(spawnSync(join(bindir, "purchasely"), ["--version"], { env: t.env, encoding: "utf8" }).stdout.trim()).toBe("1.1.0");
  });

  it("prerequisites: Node 20 and curl", () => {
    makeRelease(t.assets, "1.2.3");
    // A fake node that fails the version check.
    const fakeNode = join(t.tmp, "fakenode");
    mkdirSync(fakeNode);
    writeFileSync(join(fakeNode, "node"), "#!/bin/sh\nexit 1\n");
    chmodSync(join(fakeNode, "node"), 0o755);
    const old = install([], { PATH: `${fakeNode}:${t.env.PATH}` });
    expect(old.status).toBe(2);
    expect(old.all).toContain("Node 20");
    // No curl: PATH has only a dir of symlinks to the tools the script uses.
    const bare = bareBin(join(t.tmp, "barebin"));
    const nocurl = install([], { PATH: `${bare}:${dirname(process.execPath)}` });
    expect(nocurl.status).toBe(2);
    expect(nocurl.all).toContain("curl is required");
    expect(downloads()).toEqual([]);
  });

  it("log file: all messages go to the file", () => {
    makeRelease(t.assets, "1.2.3");
    const f = join(t.tmp, "update.log");
    const r = install(["--log", f]);
    expect(r.status, r.all).toBe(0);
    expect(r.out).toBe("");
    expect(r.err).toBe("");
    expect(readFileSync(f, "utf8")).toContain("purchasely 1.2.3 installed");
  });

  it("rejects a version that could escape the releases dir", () => {
    makeRelease(t.assets, "1.2.3");
    for (const bad of ["..", "../x", "1.0/../..", "v", "-1"]) {
      const r = install(["--version", bad]);
      expect(r.status, bad).toBe(2);
    }
    expect(downloads()).toEqual([]);
  });
});


const publicCopy = () => t.script;
const makeSite = (ver: string, opts: { badSums?: boolean } = {}) => makeRelease(t.assets, ver, opts);
const withCurl = () => t.curlLog;

/** A sha256sum like macOS's: `-c` with no file argument prints usage and fails instead of reading stdin. */
function writeDarwinSha256sum(dir: string) {
  const file = join(dir, "sha256sum");
  writeFileSync(file, `#!/bin/sh\n[ "$#" -eq 1 ] && [ "$1" = "-c" ] && { echo "usage: sha256sum [-bctwz] [files ...]" >&2; exit 1; }\nexec shasum -a 256 "$@"\n`);
  chmodSync(file, 0o755);
}

const TOOLS = ["sh", "tar", "gzip", "mkdir", "mv", "ln", "rm", "cp", "cat", "grep", "awk", "mktemp", "chmod", "sleep", "shasum", "sha256sum", "dirname"];

/** A dir of symlinks to the tools the script uses (tar -z needs gzip on Linux), minus `omit`. */
function bareBin(dir: string, omit: string[] = []) {
  mkdirSync(dir);
  for (const tool of TOOLS.filter((x) => !omit.includes(x))) {
    for (const d of ["/usr/bin", "/bin"]) {
      if (existsSync(join(d, tool))) {
        symlinkSync(join(d, tool), join(dir, tool));
        break;
      }
    }
  }
  return dir;
}

/** A PATH with the tools the script uses, node and the fake curl. */
function noGhPath(omit: string[] = []) {
  const bare = bareBin(join(t.tmp, "barebin"), omit);
  symlinkSync(join(t.fakebin, "curl"), join(bare, "curl"));
  if (!omit.includes("sha256sum")) {
    rmSync(join(bare, "sha256sum"), { force: true });
    writeDarwinSha256sum(bare);
  }
  return `${bare}:${dirname(process.execPath)}`;
}

function runScript(file: string, args: string[], env: Record<string, string>) {
  const r = spawnSync("sh", [file, ...args], { env, encoding: "utf8", input: "" });
  return { status: r.status, out: r.stdout, err: r.stderr, all: `${r.stdout}${r.stderr}` };
}

describe("install.sh channels", () => {
  it("public channel installs from latest.json with curl", () => {
    const site = makeSite("1.2.3");
    const curlLog = withCurl();
    const env = { ...t.env, PATH: noGhPath(), FAKE_SITE: site, CURL_LOG: curlLog };
    const r = runScript(publicCopy(), [], env);
    expect(r.status, r.all).toBe(0);
    expect(existsSync(join(t.xdg, "purchasely", "install", "releases", "1.2.3", "dist", "bin", "purchasely.js"))).toBe(true);
    const out = spawnSync(join(t.bindir, "purchasely"), ["--version"], { env: t.env, encoding: "utf8" });
    expect(out.stdout.trim()).toBe("1.2.3");
    expect(readFileSync(curlLog, "utf8")).toContain("--proto =https --tlsv1.2 -fsSL");
    // Only the latest version is published on the host.
    const old = runScript(publicCopy(), ["--version", "1.0.0"], env);
    expect(old.status).toBe(1);
    expect(old.all).toContain("only the latest version (1.2.3) is published on https://cli.test");
  });

  it("public channel: a missing gzip is a prerequisite error", () => {
    const curlLog = withCurl();
    const r = runScript(publicCopy(), [], { ...t.env, PATH: noGhPath(["gzip"]), FAKE_SITE: makeSite("1.2.3"), CURL_LOG: curlLog });
    expect(r.status).toBe(2);
    expect(r.all).toContain("tar and gzip are required");
  });

  it("public channel refuses a bad checksum", () => {
    const site = makeSite("1.2.3", { badSums: true });
    const curlLog = withCurl();
    const root = join(t.xdg, "purchasely", "install");
    const r = runScript(publicCopy(), [], { ...t.env, PATH: noGhPath(), FAKE_SITE: site, CURL_LOG: curlLog });
    expect(r.status).toBe(1);
    expect(existsSync(join(root, "current-version"))).toBe(false);
    expect(readdirSync(join(root, "releases"))).toEqual([]);
    expect(readdirSync(join(root, "staging"))).toEqual([]);
  });

  it("public channel: curl errors use a fixed message", () => {
    const site = makeSite("1.2.3");
    const curlLog = withCurl();
    const r = runScript(publicCopy(), [], {
      ...t.env,
      PATH: noGhPath(),
      FAKE_SITE: site,
      CURL_LOG: curlLog,
      FAKE_CURL_STDERR: SECRET,
    });
    expect(r.status).toBe(1);
    expect(r.err).toContain("download failed from https://cli.test. Check your network, then run this again.");
    expect(r.all).not.toContain(SECRET);
  });

  it("a truncated script runs nothing", () => {
    const half = readFileSync(INSTALL_SH, "utf8").split("\n");
    const cut = half.slice(0, Math.floor(half.length / 2)).join("\n");
    const r = spawnSync("sh", [], { env: t.env, encoding: "utf8", input: cut });
    expect(r.status).not.toBe(0);
    expect(curlLines()).toEqual([]);
    expect(existsSync(t.root)).toBe(false);
  });

});

describe("install.sh PATH and profile", () => {
  const MARKER = "# added by the purchasely installer";
  const LINE = `export PATH="$HOME/.local/bin:$PATH" ${MARKER}`;
  const count = (file: string) => (existsSync(file) ? readFileSync(file, "utf8").split("\n").filter((l) => l.includes(MARKER)).length : 0);

  /** Own HOME, SHELL and PATH, and no PURCHASELY_BIN_DIR. */
  function pathEnv(home: string, shell: string, path = t.env.PATH as string) {
    const env = { ...t.env, HOME: home, SHELL: shell, PATH: path };
    delete (env as Record<string, string | undefined>).PURCHASELY_BIN_DIR;
    return env;
  }

  it("bin dir already on PATH: no profile edit", () => {
    makeRelease(t.assets, "1.2.3");
    mkdirSync(join(t.home, "bin"));
    const r = runScript(t.script, [], pathEnv(t.home, "/bin/zsh", `${join(t.home, "bin")}:${t.env.PATH}`));
    expect(r.status, r.all).toBe(0);
    expect(readlinkSync(join(t.home, "bin", "purchasely"))).toBe(join(t.root, "bin", "purchasely"));
    expect(existsSync(join(t.home, ".zshrc"))).toBe(false);
    expect(r.out).not.toContain("open a new terminal");
  });

  it("no bin dir on PATH: one profile line, once", () => {
    makeRelease(t.assets, "1.2.3");
    const site = makeSite("1.2.3");
    const curlLog = withCurl();
    const env = { ...pathEnv(t.home, "/bin/bash"), FAKE_SITE: site, CURL_LOG: curlLog };
    const first = runScript(t.script, [], env);
    expect(first.status, first.all).toBe(0);
    expect(first.out).toContain("open a new terminal");
    const second = runScript(publicCopy(), [], env);
    expect(second.status, second.all).toBe(0);
    expect(lstatSync(join(t.home, ".local", "bin", "purchasely")).isSymbolicLink()).toBe(true);
    expect(lstatSync(join(t.home, ".local", "bin", "purchasely")).isSymbolicLink()).toBe(true);
    for (const f of [".bashrc", ".bash_profile"]) {
      expect(count(join(t.home, f)), f).toBe(1);
      expect(readFileSync(join(t.home, f), "utf8")).toContain(LINE);
    }
    // The same install again appends nothing.
    runScript(t.script, [], env);
    expect(count(join(t.home, ".bashrc"))).toBe(1);
    // ~/.local/bin exists but is not on PATH: the line is still appended.
    const home2 = join(t.tmp, "home2");
    mkdirSync(join(home2, ".local", "bin"), { recursive: true });
    const r = runScript(t.script, [], { ...pathEnv(home2, "/bin/zsh"), XDG_DATA_HOME: join(t.tmp, "xdg2") });
    expect(r.status, r.all).toBe(0);
    expect(count(join(home2, ".zshrc"))).toBe(1);
    expect(lstatSync(join(home2, ".local", "bin", "purchasely")).isSymbolicLink()).toBe(true);
  });

  it("fish or unknown shell uses .profile", () => {
    makeRelease(t.assets, "1.2.3");
    const r = runScript(t.script, [], pathEnv(t.home, "/usr/local/bin/fish"));
    expect(r.status, r.all).toBe(0);
    expect(count(join(t.home, ".profile"))).toBe(1);
    for (const f of [".zshrc", ".bashrc", ".bash_profile"]) expect(existsSync(join(t.home, f)), f).toBe(false);
  });

  it("upgrade-only changes no PATH and no profile", () => {
    makeRelease(t.assets, "1.0.0");
    makeRelease(t.assets, "1.1.0");
    expect(install(["--version", "1.0.0"]).status).toBe(0);
    const before = readlinkSync(join(t.bindir, "purchasely"));
    const r = runScript(t.script, ["--root", t.root, "--version", "1.1.0", "--upgrade-only"], { ...pathEnv(t.home, "/bin/zsh"), FAKE_SITE: join(t.assets, "1.1.0") });
    expect(r.status, r.all).toBe(0);
    expect(current()).toBe("1.1.0");
    expect(readlinkSync(join(t.bindir, "purchasely"))).toBe(before);
    for (const f of [".zshrc", ".bashrc", ".bash_profile", ".profile", ".local"]) expect(existsSync(join(t.home, f)), f).toBe(false);
  });
});

describe("install.sh man page", () => {
  const page = (e: Env = t) => join(e.xdg, "man", "man1", "purchasely.1");
  const HINT = /To read "man purchasely", add to your shell profile: export MANPATH="[^"\n]*man:\$MANPATH"/;

  /** A fake man: logs the PATH and MANPATH it runs with; `man -w` prints $FAKE_MAN_FOUND, or fails as when no page exists. */
  function withFakeMan(e: Env = t) {
    const log = join(e.tmp, "man.log");
    writeFileSync(join(e.fakebin, "man"), `#!/bin/sh\necho "$* | PATH=$PATH | MANPATH=\${MANPATH-unset}" >> "$MAN_LOG"\n[ -n "\${FAKE_MAN_FOUND:-}" ] && { echo "$FAKE_MAN_FOUND"; exit 0; }\nexit 16\n`);
    chmodSync(join(e.fakebin, "man"), 0o755);
    return log;
  }

  it("man page: installed in the XDG man dir and replaced by an update", () => {
    makeRelease(t.assets, "1.0.0", { man: ".TH PURCHASELY 1\nv1\n" });
    makeRelease(t.assets, "1.1.0", { man: ".TH PURCHASELY 1\nv2\n" });
    makeRelease(t.assets, "1.2.0", { man: ".TH PURCHASELY 1\nv3\n" });
    expect(install(["--version", "1.0.0"]).status).toBe(0);
    expect(readFileSync(page(), "utf8")).toBe(".TH PURCHASELY 1\nv1\n");
    expect(install(["--version", "1.1.0"]).status).toBe(0);
    expect(readFileSync(page(), "utf8")).toBe(".TH PURCHASELY 1\nv2\n");
    // The background updater runs with --upgrade-only and the managed root.
    expect(install(["--root", t.root, "--version", "1.2.0", "--upgrade-only"]).status).toBe(0);
    expect(readFileSync(page(), "utf8")).toBe(".TH PURCHASELY 1\nv3\n");
    expect(readdirSync(join(t.xdg, "man", "man1"))).toEqual(["purchasely.1"]); // no temporary file is left
  });

  it("man page: a release without one installs and leaves no file", () => {
    makeRelease(t.assets, "1.2.3");
    const r = install();
    expect(r.status, r.all).toBe(0);
    expect(existsSync(join(t.xdg, "man"))).toBe(false);
    expect(r.all).not.toContain("man page");
  });

  it("man page: the same version installed again repairs a missing or changed page", () => {
    makeRelease(t.assets, "1.2.3", { man: "good\n" });
    expect(install().status).toBe(0);
    rmSync(page());
    const again = install();
    expect(again.out).toContain("up to date");
    expect(readFileSync(page(), "utf8")).toBe("good\n");
    writeFileSync(page(), "edited\n");
    expect(install().status).toBe(0);
    expect(readFileSync(page(), "utf8")).toBe("good\n");
  });

  it("man page: --upgrade-only on the same version restores a deleted page, without a hint", () => {
    makeRelease(t.assets, "1.2.3", { man: "good\n" });
    expect(install().status).toBe(0);
    rmSync(page());
    const r = install(["--version", "1.2.3", "--upgrade-only"]);
    expect(r.out).toContain("up to date");
    expect(readFileSync(page(), "utf8")).toBe("good\n");
    expect(r.all).not.toContain("MANPATH");
  });

  it("man page: a failed copy warns and does not fail the install, the next run installs it", () => {
    makeRelease(t.assets, "1.2.3", { man: "good\n" });
    mkdirSync(t.xdg, { recursive: true });
    writeFileSync(join(t.xdg, "man"), "a file where the man dir should be"); // mkdir -p cannot pass
    const r = install();
    expect(r.status, r.all).toBe(0);
    expect(current()).toBe("1.2.3");
    expect(r.all).toContain("warning: could not install the man page");
    expect(r.all).toContain(`${t.xdg}/man`);
    rmSync(join(t.xdg, "man"));
    const retry = install();
    expect(retry.status, retry.all).toBe(0);
    expect(retry.all).not.toContain("warning");
    expect(readFileSync(page(), "utf8")).toBe("good\n");
  });

  it("man page: man -w runs with the bin dir on PATH, and the hint shows only when man does not find the page", () => {
    makeRelease(t.assets, "1.2.3", { man: "good\n" });
    const log = withFakeMan();
    const env = { MAN_LOG: log };
    // man finds the installed page: no hint. The bin dir is the first PATH entry, and the page is searched by name.
    const found = install([], { ...env, FAKE_MAN_FOUND: page() });
    expect(found.status, found.all).toBe(0);
    expect(found.out).not.toContain("MANPATH");
    const ran = readFileSync(log, "utf8").trim().split("\n");
    expect(ran).toHaveLength(1);
    expect(ran[0]!.startsWith(`-w purchasely | PATH=${t.bindir}:`)).toBe(true);
    // man finds nothing, or another page: the hint.
    rmSync(t.root, { recursive: true });
    const nothing = install([], env);
    expect(nothing.out).toContain(`export MANPATH="${t.xdg}/man:$MANPATH"`);
    const other = join(t.tmp, "other.1");
    writeFileSync(other, "stale\n");
    expect(install([], { ...env, FAKE_MAN_FOUND: other }).out).toMatch(HINT);
  });

  it("man page: no man check when the release has no page, nor on an upgrade-only run", () => {
    makeRelease(t.assets, "1.0.0");
    makeRelease(t.assets, "1.1.0", { man: "good\n" });
    const log = withFakeMan();
    expect(install(["--version", "1.0.0"], { MAN_LOG: log }).status).toBe(0);
    expect(existsSync(log)).toBe(false);
    // upgrade-only never chooses a bin dir, so it prints no hint, but it installs the page.
    const r = install(["--root", t.root, "--version", "1.1.0", "--upgrade-only"], { MAN_LOG: log });
    expect(r.status, r.all).toBe(0);
    expect(r.out).not.toContain("MANPATH");
    expect(existsSync(page())).toBe(true);
  });

  describe("without man (best effort: the rule man uses)", () => {
    // Derived layout: bin dir <x>/bin and data dir <x>/share, so man derives <x>/share/man from the PATH entry.
    const derived = () => ({ XDG_DATA_HOME: join(t.tmp, "x", "share"), PURCHASELY_BIN_DIR: join(t.tmp, "x", "bin") });
    const hint = (manpath: string | undefined, layout: Record<string, string>) => {
      const env: Record<string, string> = { ...layout, PATH: `${t.fakebin}:${bareBin(join(t.tmp, `bare${Math.random()}`))}:${dirname(process.execPath)}` };
      const e = { ...t.env, ...env } as Record<string, string | undefined>;
      if (manpath === undefined) delete e.MANPATH;
      else e.MANPATH = manpath;
      const r = spawnSync("sh", [t.script], { env: e as NodeJS.ProcessEnv, encoding: "utf8", input: "" });
      expect(r.status, r.stdout + r.stderr).toBe(0);
      return /MANPATH/.test(r.stdout);
    };
    const mandir = () => join(t.tmp, "x", "share", "man");
    const custom = () => ({ XDG_DATA_HOME: t.xdg, PURCHASELY_BIN_DIR: join(t.tmp, "custom", "bin") });

    it("man page: MANPATH unset or empty or with an empty component: the bin dir decides", () => {
      makeRelease(t.assets, "1.2.3", { man: "good\n" });
      for (const m of [undefined, "", ":/usr/share/man", "/usr/share/man:", "/usr/share/man::/opt/man"]) {
        expect(hint(m, derived()), `derived, MANPATH=${m}`).toBe(false);
        expect(hint(m, custom()), `custom, MANPATH=${m}`).toBe(true);
      }
    });

    it("man page: a MANPATH with no empty component is the whole search path, the bin dir does not count", () => {
      makeRelease(t.assets, "1.2.3", { man: "good\n" });
      expect(hint("/usr/share/man", derived())).toBe(true);
      expect(hint("/usr/share/man:/opt/man", derived())).toBe(true);
    });

    it("man page: an explicit man dir in MANPATH counts", () => {
      makeRelease(t.assets, "1.2.3", { man: "good\n" });
      expect(hint(mandir(), derived())).toBe(false);
      expect(hint(`/usr/share/man:${mandir()}`, derived())).toBe(false);
      expect(hint(`${mandir()}:/usr/share/man`, custom())).toBe(true); // custom layout: another dir than the one installed
      expect(hint(`${join(t.xdg, "man")}:/usr/share/man`, custom())).toBe(false);
      expect(hint(`${join(t.xdg, "man")}::/usr/share/man`, custom())).toBe(false);
    });
  });

  it("man page: man finds it from PATH", () => {
    if (spawnSync("sh", ["-c", "command -v man"]).status !== 0) return; // no man here: nothing to check
    // Derived layout and the real man: the page the installer copied is the page man -w prints.
    const x = join(t.tmp, "x");
    const layout = { XDG_DATA_HOME: join(x, "share"), PURCHASELY_BIN_DIR: join(x, "bin") };
    makeRelease(t.assets, "1.2.3", { man: ".TH PURCHASELY 1\n.SH NAME\npurchasely \\- test\n" });
    const r = install([], layout);
    expect(r.status, r.all).toBe(0);
    expect(r.out).not.toContain("MANPATH");
    const env = { PATH: `${layout.PURCHASELY_BIN_DIR}:/usr/bin:/bin`, HOME: t.home };
    const w = spawnSync("man", ["-w", "purchasely"], { env, encoding: "utf8" });
    expect(w.status, w.stderr).toBe(0);
    expect(readFileSync(w.stdout.trim(), "utf8")).toBe(readFileSync(join(layout.XDG_DATA_HOME, "man", "man1", "purchasely.1"), "utf8"));
  });
});
