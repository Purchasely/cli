import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { EXT, setup } from "./helpers.js";

const have = (bin: string) => spawnSync(bin, ["--version"]).error === undefined;
const SHELLS = ["bash", "zsh", "fish"] as const;
/** How each shell writes a long option: fish takes `-l name`, without the dashes. */
const long = (sh: string, name: string) => (sh === "fish" ? `-l ${name}` : `--${name}`);
describe("completion", () => {
  it("prints a script for each shell that holds the visible commands and the long options", async () => {
    const t = setup({ env: { PURCHASELY_API_KEY: "" } });
    for (const sh of SHELLS) {
      const r = await t.exec("completion", sh);
      expect(r.code, r.err).toBe(0);
      expect(r.out, sh).toContain("placements");
      expect(r.out, sh).toContain(long(sh, "app"));
      expect(r.out, sh).toContain("ab-tests");
    }
    expect(t.calls).toEqual([]);
  });

  it("needs no key, makes no request, writes no stderr line and stores nothing", async () => {
    const t = setup({ env: { PURCHASELY_API_KEY: "" } });
    const r = await t.exec("completion", "zsh");
    expect(r.code).toBe(0);
    expect(r.err).toBe("");
    expect(existsSync(t.d.configDir)).toBe(false);
    expect(existsSync(t.d.stateDir)).toBe(false);
  });

  it("refuses an unknown shell and a missing shell as a usage error", async () => {
    const t = setup();
    for (const argv of [["completion", "powershell"], ["completion"]]) {
      const r = await t.exec(...argv);
      expect(r.code, argv.join(" ")).toBe(2);
      expect(r.out).toBe("");
      expect(r.errJson().error.code).toBe("USAGE");
    }
  });

  it("leaves out a hidden command and a hidden option", async () => {
    const t = setup({ ext: { ...EXT, modules: [(p) => {
      const c = p.command("secret-cmd", { hidden: true }).description("x");
      c.option("--secret-opt <x>", "x");
      const v = p.command("visible-cmd").description("A visible one");
      v.option("--visible-opt <x>", "x");
      v.addOption(Object.assign(v.createOption("--gone-opt <x>", "x"), { hidden: true }));
    }] }, name: "acme" });
    for (const sh of SHELLS) {
      const out = (await t.exec("completion", sh)).out;
      expect(out, sh).toContain("visible-cmd");
      expect(out, sh).toContain(long(sh, "visible-opt"));
      expect(out, sh).not.toContain("secret-cmd");
      expect(out, sh).not.toContain("secret-opt");
      expect(out, sh).not.toContain("gone-opt");
    }
  });

  it("names the program of the build", async () => {
    const pub = (await setup().exec("completion", "bash")).out;
    const acme = (await setup({ name: "acme", ext: EXT }).exec("completion", "bash")).out;
    expect(pub).toContain("complete -F _purchasely_complete purchasely");
    expect(acme).toContain("complete -F _acme_complete acme");
    expect(acme).toContain("extra");
    expect(pub).not.toContain("extra");
  });

  it.skipIf(!have("bash"))("bash: the script parses and completes a command, a subcommand and an option", async () => {
    const t = setup();
    const script = (await t.exec("completion", "bash")).out;
    const file = join(mkdtempSync(join(tmpdir(), "cli-compl-")), "c.bash");
    writeFileSync(file, script);
    expect(spawnSync("bash", ["-n", file]).status).toBe(0);
    const complete = (...words: string[]) => {
      const r = spawnSync("bash", ["-c", `source "${file}"; COMP_WORDS=(${words.map((w) => `'${w}'`).join(" ")}); COMP_CWORD=${words.length - 1}; _purchasely_complete; printf '%s\\n' "\${COMPREPLY[@]}"`], { encoding: "utf8" });
      expect(r.status, r.stderr).toBe(0);
      return r.stdout.split("\n").filter(Boolean);
    };
    expect(complete("purchasely", "plac")).toEqual(["placements"]);
    expect(complete("purchasely", "auth", "")).toEqual(expect.arrayContaining(["login", "status", "logout"]));
    expect(complete("purchasely", "auth", "")).not.toContain("placements");
    expect(complete("purchasely", "placements", "list", "--ap")).toContain("--app");
    expect(complete("purchasely", "completion", "")).toEqual(expect.arrayContaining(["--help"]));
  });

  it("zsh: no local variable named path, which zsh ties to PATH", async () => {
    const out = (await setup().exec("completion", "zsh")).out;
    expect(out).not.toMatch(/\bpath\b/);
  });

  it.skipIf(!have("zsh"))("zsh: the script parses", async () => {
    const file = join(mkdtempSync(join(tmpdir(), "cli-compl-")), "c.zsh");
    writeFileSync(file, (await setup().exec("completion", "zsh")).out);
    expect(spawnSync("zsh", ["-n", file]).status).toBe(0);
  });

  it.skipIf(!have("fish"))("fish: the script parses", async () => {
    const file = join(mkdtempSync(join(tmpdir(), "cli-compl-")), "c.fish");
    writeFileSync(file, (await setup().exec("completion", "fish")).out);
    expect(spawnSync("fish", ["-n", file]).status).toBe(0);
  });
});
