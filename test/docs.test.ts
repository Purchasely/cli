import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import type { Command } from "commander";
import { describe, expect, it } from "vitest";
import { collect, markdown } from "../src/core/reference.js";
import { Runtime } from "../src/core/runtime.js";
import { build } from "../src/run.js";
import { managementCommands } from "../src/surfaces/management/index.js";
import { setup } from "./helpers.js";

const ROOT = resolve(import.meta.dirname, "..");
const read = (f: string) => readFileSync(join(ROOT, f), "utf8");
const DOCS = ["install", "commands", "guardrails", "exit-codes", "limits", "development", "release"].map((n) => `docs/${n}.md`);
const mdFiles = () => ["README.md", "AGENTS.md", ...readdirSync(join(ROOT, "docs")).filter((f) => f.endsWith(".md")).map((f) => `docs/${f}`)];

const subs = (c: Command) => c.createHelp().visibleCommands(c).filter((x) => x.name() !== "help");
const leaves = (c: Command, path: string[] = []): string[] =>
  subs(c).flatMap((x) => (subs(x).length === 0 ? [[...path, x.name()].join(" ")] : leaves(x, [...path, x.name()])));

describe("docs", () => {
  it("links resolve", () => {
    for (const f of DOCS) expect(existsSync(join(ROOT, f)), f).toBe(true);
    for (const f of mdFiles()) {
      for (const m of read(f).matchAll(/\]\(([^)\s]+)\)/g)) {
        const target = m[1].split("#")[0];
        if (!target || /^[a-z]+:/i.test(target)) continue;
        expect(existsSync(resolve(ROOT, dirname(f), target)), `${f} -> ${m[1]}`).toBe(true);
      }
    }
    // The README is the entry point: it links to every reference file.
    for (const f of [...DOCS, "AGENTS.md"]) expect(read("README.md"), f).toContain(`](${f})`);
  });

  it("every hand-written command is documented", () => {
    const generated = new Set(managementCommands().map((c) => `${c.resource} ${c.action}`));
    const program = build(new Runtime(setup().d));
    const doc = read("docs/commands.md");
    expect(leaves(program).filter((p) => !generated.has(p)).filter((p) => !doc.includes(`purchasely ${p}`))).toEqual([]);
  });

  // The file is generated: `npx vitest run test/docs.test.ts -t "commands.md is generated" -u` rewrites it.
  const generated = () => {
    const rt = new Runtime(setup().d);
    return "<!-- Generated from the command tree. Do not edit. Run: npx vitest run test/docs.test.ts -t \"commands.md is generated\" -u -->\n\n" + markdown(collect(build(rt), rt));
  };

  it("commands.md is generated", async () => {
    await expect(generated()).toMatchFileSnapshot("../docs/commands.md");
  });

  it("every documented row is in the tree", () => {
    const doc = generated();
    for (const w of ["keychain", "200 KB", "PURCHASELY_OFFLINE"]) expect(doc, w).toContain(w);
  });

  it("ai section: readmes and landing page", () => {
    for (const f of ["README.md", "site/index.html"]) {
      const text = read(f);
      expect(text, f).toContain(f.startsWith("site/") ? "The Purchasely AI plugin" : "Use with your AI agent");
      for (const w of ["purchasely ai install", "ai status", "ai update", "ai uninstall", "--agent", "purchasely-cli", "sdk-expert", "Claude Code", "Codex", "Copilot CLI", "Gemini CLI", "Cursor", "OpenCode"]) expect(text, `${f}: ${w}`).toContain(w);
      expect(text, f).toMatch(/SDK API key/);
    }
    expect(read("README.md").split("\n").length).toBeLessThan(100);
    const site = read("site/index.html");
    expect(site).toContain('href="#ai"');
    expect(site).toContain('id="ai"');
  });


  it("landing page: sharing head, reference links, real 404 and robots.txt", () => {
    const site = read("site/index.html");
    expect(site).toContain("<title>Purchasely CLI: your account in the terminal, the CI and your AI agent</title>");
    for (const tag of ['rel="canonical" href="https://cli.purchasely.io/"', 'property="og:image" content="https://cli.purchasely.io/assets/og.png"', 'name="twitter:card" content="summary_large_image"'])
      expect(site).toContain(tag);
    expect(existsSync(join(ROOT, "site/assets/og.png"))).toBe(true);
    expect(site).toContain('href="/reference"');
    expect(site).toContain('href="https://docs.purchasely.com/docs/management-api"');
    // The dry-run sample shows the real User-Agent form, and no install promise without a date.
    expect(site).toContain('"User-Agent": "purchasely-cli/&lt;version&gt; (-; -)"');
    expect(site).not.toMatch(/homebrew|@placement\.json/i);
    const notFound = read("site/404.html");
    expect(notFound).toContain('href="/"');
    // Served at any depth: assets need absolute paths.
    expect(notFound).not.toMatch(/(src|href)=["']?assets\//);
    for (const f of ["site/index.html", "site/404.html"]) expect(read(f), f).not.toMatch(/[\u2013\u2014]/);
    expect(read("site/robots.txt")).toBe("User-agent: *\nAllow: /\n");
  });


  it("ai section: commands, install, limits, release", () => {
    const doc = read("docs/commands.md");
    for (const c of ["ai install", "ai status", "ai update", "ai uninstall"]) expect(doc, c).toContain(`purchasely ${c}`);
    const install = read("docs/install.md");
    for (const w of ["ai.plugin.json", "daily", "never touch"]) expect(install, w).toContain(w);
    const limits = read("docs/limits.md");
    for (const w of ["gemini extensions update purchasely", "120 s", "restart"]) expect(limits, w).toContain(w);
    // The release doc names what the workflow pins.
    const wf = read(".github/workflows/release.yml");
    const rel = read("docs/release.md");
    expect(rel).toContain(/wrangler@[\d.]+/.exec(wf)![0]);
    // CI and the release never need more than read access.
    for (const f of [".github/workflows/ci.yml", ".github/workflows/release.yml"]) expect(read(f), f).toMatch(/^permissions:\n {2}contents: read$/m);
    for (const w of ["--branch=main", "SHA256SUMS", "install.sh", "_headers", "latest.json", "scripts/pack-release.sh"]) expect(rel, w).toContain(w);
  });

  it("uninstall and skill count match the code", () => {
    const readme = read("README.md");
    expect(readme).toContain("command -v purchasely");
    expect(readme).not.toContain("~/.local/bin/purchasely");
    expect(readme).toMatch(/6 skills from plugin 2\.3\.0 \(5 before\)/);
  });

  it("no em-dash or en-dash", () => {
    for (const f of mdFiles()) expect(read(f), f).not.toMatch(/[\u2014\u2013]/);
  });
});
