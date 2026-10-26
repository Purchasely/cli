import { spawnSync } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { Command } from "commander";
import { describe, expect, it, vi } from "vitest";
import { collect } from "../src/core/reference.js";
import { Runtime } from "../src/core/runtime.js";
import { build, type Register } from "../src/run.js";
import { EXT, setup } from "./helpers.js";

// Every node of both trees runs `--help` once.
vi.setConfig({ testTimeout: 180000 });

const subs = (c: Command) => c.createHelp().visibleCommands(c).filter((x) => x.name() !== "help");
/** The paths of the commands that have no subcommand. */
const leavesOf = (c: Command, path: string[] = []): string[][] => subs(c).flatMap((x) => (subs(x).length ? leavesOf(x, [...path, x.name()]) : [[...path, x.name()]]));
const extOf = (modules: Register[]) => (modules.length ? { ...EXT, modules } : undefined);
const pub = () => setup();
const acme = () => setup({ name: "acme", ext: EXT });
const programOf = (t: ReturnType<typeof setup>, modules: Register[] = []) => build(new Runtime({ ...t.d, ext: extOf(modules) }));
/** The tree and its reference come from one Runtime, as in `run()`: the Runtime knows which groups an extension added. */
const refOf = (t: ReturnType<typeof setup>, modules: Register[] = []) => {
  const rt = new Runtime({ ...t.d, ext: extOf(modules) });
  return collect(build(rt), rt);
};
const FORMATS = ["markdown", "man", "html"];

/** `--help` wraps at the terminal width. Pin it to the 100 columns of the reference. */
async function atWidth100<T>(f: () => Promise<T>): Promise<T> {
  const was = { isTTY: process.stdout.isTTY, columns: process.stdout.columns };
  Object.assign(process.stdout, { isTTY: true, columns: 100 });
  try {
    return await f();
  } finally {
    Object.assign(process.stdout, was);
  }
}

describe("reference", () => {
  it("leaves a hidden command out, as --help does", () => {
    for (const t of [pub(), acme()]) {
      const r = refOf(t, []);
      expect(programOf(t).commands.some((c) => c.name() === "play")).toBe(true);
      expect(r.groups.map((g) => g.name)).not.toContain("play");
      expect(r.groups.flatMap((g) => g.leaves.map((l) => l.path[0]))).not.toContain("play");
      // `games` is the visible name of the same action.
      expect(r.groups.flatMap((g) => g.leaves.map((l) => l.path[0]))).toContain("games");
    }
  });

  it("prints every public leaf of purchasely, offline", async () => {
    const t = pub();
    // The helpers give a key and switch the update check off. Offline means neither is needed.
    delete t.d.env.PURCHASELY_API_KEY;
    delete t.d.env.PURCHASELY_DISABLE_AUTOUPDATE;
    t.d.env.PURCHASELY_ENV = "not-an-env";
    const touched: string[] = [];
    const spy = (name: string) => async () => {
      touched.push(name);
      throw new Error(`${name} must not be called`);
    };
    t.d.exec = spy("exec");
    t.d.readStdin = spy("stdin") as typeof t.d.readStdin;
    t.d.prompt = spy("prompt") as typeof t.d.prompt;
    t.keychain.get = spy("keychain.get");
    t.keychain.set = spy("keychain.set");
    t.keychain.delete = spy("keychain.delete");

    const r = await t.exec("reference");
    expect(r.code, r.err).toBe(0);
    expect(r.err).toBe("");
    expect(touched).toEqual([]);
    expect(t.calls.length).toBe(0);
    for (const path of leavesOf(programOf(pub()))) expect(r.out, path.join(" ")).toContain(`### \`purchasely ${path.join(" ")}\``);
    expect(r.out).toContain("Required scope:");
    expect(r.out).toContain("PATCH /management/v1/applications/{application_id}/placements/{id}");
    expect(r.out).toContain("Body (");
    expect(r.out).toContain("Example:");
    expect(r.out).toContain("Confirmation flow");
  });

  it("help --all prints the reference", async () => {
    const t = pub();
    const reference = await t.exec("reference");
    expect(reference.out.length).toBeGreaterThan(1000);
    expect((await t.exec("help", "--all")).out).toBe(reference.out);
    expect((await t.exec("help", "--all", "--format", "man")).out).toBe((await t.exec("reference", "--format", "man")).out);
    const help = await t.exec("help");
    expect(help.out).toContain("Usage: purchasely");
    expect(help.out).not.toBe(reference.out);
  });

  it("an extension adds its section after the built-in part", async () => {
    const out = (await acme().exec("reference")).out;
    expect(out.split("\n").filter((l) => l === "## More commands")).toHaveLength(1);
    expect(out.indexOf("### `acme extra run`")).toBeGreaterThan(out.indexOf("\n## More commands\n"));
    expect(out.indexOf("### `acme auth login`")).toBeLessThan(out.indexOf("\n## More commands\n"));
    expect((await pub().exec("reference")).out).not.toContain("## More commands");
  });

  it("every block is the --help of its node, for both programs", async () => {
    for (const [make, modules] of [[pub, []], [acme, EXT.modules]] as const) {
      const t = make();
      const ref = refOf(t, [...modules]);
      const nodes = ref.groups.flatMap((g) => g.leaves);
      expect(nodes.length).toBeGreaterThan(150);
      const md = (await t.exec("reference")).out;
      await atWidth100(async () => {
        expect((await t.exec("--help")).out, "root").toBe(ref.root);
        for (const leaf of nodes) {
          expect((await t.exec(...leaf.path, "--help")).out, leaf.path.join(" ")).toBe(leaf.help);
          expect(md, leaf.path.join(" ")).toContain(leaf.help.replace(/\n+$/, ""));
        }
      });
      expect(md).toContain(ref.root.replace(/\n+$/, ""));
      expect(md).not.toContain("--background");
      expect(nodes.find((l) => l.path.join(" ") === "search")!.help).toContain("--limit <n>");
    }
  });

  it("markdown is stable", async () => {
    const a = setup();
    const b = setup();
    const out = (await a.exec("reference")).out;
    b.d.now = () => 1_800_000_000_000;
    expect((await b.exec("reference")).out).toBe(out);
    expect(out).not.toContain("9.9.9");
    expect(out).not.toContain("2023-11-14");
    expect(out.endsWith("\n")).toBe(true);
    expect(out.endsWith("\n\n")).toBe(false);
    expect(out.match(/```/g)!.length % 2).toBe(0);
    // A fenced block that holds a fence would end early.
    for (const block of out.split("```text\n").slice(1)) expect(block.split("```")[0]).not.toContain("`".repeat(3));
  });

  it("sorts groups and commands by path, and does not depend on the registration order", async () => {
    const leaf = (name: string) => (p: Command) => void p.command(name).description(`the ${name}`).option("--x", "an option").action(() => undefined);
    const t = setup({ name: "acme" });
    const a = refOf(t, [leaf("zeta"), leaf("alpha")]);
    const b = refOf(t, [leaf("alpha"), leaf("zeta")]);
    expect(a.groups).toEqual(b.groups);
    const names = (g: typeof a.groups) => g.filter((x) => x.extension).map((x) => x.name);
    expect(names(a.groups)).toEqual(["alpha", "zeta"]);
    const real = refOf(t, EXT.modules);
    for (const part of [real.groups.filter((g) => !g.extension), real.groups.filter((g) => g.extension)]) {
      const sorted = [...part].sort((x, y) => (x.name < y.name ? -1 : x.name > y.name ? 1 : 0));
      expect(part.map((g) => g.name)).toEqual(sorted.map((g) => g.name));
      for (const g of part) {
        const paths = g.leaves.map((l) => l.path.join(" "));
        expect(paths).toEqual([...paths].sort((x, y) => (x < y ? -1 : x > y ? 1 : 0)));
      }
    }
    // The built-in groups all come before the groups of the extension.
    expect(real.groups.findIndex((g) => g.extension)).toBe(real.groups.filter((g) => !g.extension).length);
    // The running tree keeps its order.
    const program = build(new Runtime({ ...t.d, ext: extOf([leaf("zeta"), leaf("alpha")]) }));
    const before = program.commands.map((c) => c.name());
    collect(program, new Runtime(t.d));
    expect(program.commands.map((c) => c.name())).toEqual(before);
  });

  it("man: header, escapes and lint", async () => {
    const out = (await pub().exec("reference", "--format", "man")).out;
    const lines = out.split("\n");
    expect(lines[0]).toMatch(/^\.TH PURCHASELY 1 \d{4}-\d{2}-\d{2} "purchasely /);
    expect(lines[0]).toContain("2023-11-14");
    expect(out.endsWith("\n")).toBe(true);
    expect(out.endsWith("\n\n")).toBe(false);
    for (const l of lines) expect(l.startsWith("'"), l).toBe(false);
    for (const l of lines.filter((x) => x.startsWith("."))) expect(l, l).toMatch(/^\.(TH|SH|SS|TP|PP|nf|fi|B|I)( |$)/);
    for (const l of lines.slice(1)) expect(l, l).not.toMatch(/(?<!\\)-/);
    for (const name of ["NAME", "SYNOPSIS", "DESCRIPTION", "COMMANDS", `"SEE ALSO"`]) expect(out).toContain(`.SH ${name}`);
    expect(out).not.toContain(".SH \"MORE COMMANDS\"");
    expect(out).toContain("https://cli.test/reference");
    const ext = (await acme().exec("reference", "--format", "man")).out;
    expect(ext.split("\n")[0]).toMatch(/^\.TH ACME 1 /);
    expect(ext).toContain(`.SH "MORE COMMANDS"`);

    // CI installs mandoc: a missing one there is a failure, not a skip.
    if (spawnSync("mandoc", ["-V"]).error) {
      expect(process.env.CI, "mandoc is missing in CI").toBeFalsy();
      return;
    }
    for (const text of [out, ext]) {
      const file = join(mkdtempSync(join(tmpdir(), "cli-man-")), "x.1");
      writeFileSync(file, text);
      const r = spawnSync("mandoc", ["-T", "lint", "-W", "warning", file], { encoding: "utf8" });
      expect({ status: r.status, out: r.stdout + r.stderr }).toEqual({ status: 0, out: "" });
    }
  });

  it("html: escaped, one unique id per leaf, no external script", async () => {
    const zz = [(p: Command) => void p.command("zz").description("a <b> & c").action(() => undefined)];
    const t = setup({ ext: { ...EXT, modules: zz } });
    const html = (await t.exec("reference", "--format", "html")).out;
    expect(html).toContain("a &lt;b&gt; &amp; c");
    expect(html).not.toContain("a <b> & c");
    expect(html.startsWith("<!doctype html>")).toBe(true);
    expect(html.endsWith("\n")).toBe(true);
    expect(html.endsWith("\n\n")).toBe(false);
    const ids = [...html.matchAll(/ id="([^"]+)"/g)].map((m) => m[1]);
    expect(new Set(ids).size).toBe(ids.length);
    const ref = refOf(t, zz);
    for (const g of ref.groups) {
      expect(ids).toContain(g.id);
      for (const l of g.leaves) expect(ids).toContain(l.id);
    }
    expect(html).not.toMatch(/<script[^>]* src=/);
    expect(html).not.toMatch(/<link[^>]*rel="stylesheet"/);
    expect(html).not.toContain("@import");
  });

  it("html: only a page with extension commands has their rules", async () => {
    expect((await pub().exec("reference", "--format", "html")).out).not.toContain("extension-commands");
    expect((await acme().exec("reference", "--format", "html")).out).toContain("#extension-commands{");
  });

  it("no format has a dash character", async () => {
    for (const make of [pub, acme]) {
      for (const f of FORMATS) expect((await make().exec("reference", "--format", f)).out, `${make().d.name} ${f}`).not.toMatch(/[–—]/);
    }
  });

  it("--format accepts markdown, man, html and json only", async () => {
    const t = pub();
    const bad = await t.exec("reference", "--format", "pdf");
    expect(bad.code).toBe(2);
    expect(bad.out).toBe("");
    expect(bad.errJson().error.code).toBe("USAGE");
    expect(bad.err).toContain("--format must be markdown, man, html or json.");
    expect((await t.exec("reference", "--out", "x")).code).toBe(2);
    expect((await t.exec("reference", "--output", "table")).code).toBe(2);
    expect((await t.exec("reference", "--env", "staging")).code).toBe(2);
    for (const f of FORMATS) expect((await t.exec("reference", "--format", f)).code).toBe(0);
  });
});
