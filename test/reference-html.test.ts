import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it, vi } from "vitest";
import { collect } from "../src/core/reference.js";
import { html } from "../src/core/reference-html.js";
import { Runtime } from "../src/core/runtime.js";
import { build } from "../src/run.js";
import { EXT, setup } from "./helpers.js";

// Every node of the tree runs `--help` once.
vi.setConfig({ testTimeout: 180000 });

const site = readFileSync(join(__dirname, "..", "site", "index.html"), "utf8");
const pageOf = (name: "purchasely" | "acme") => {
  const t = name === "acme" ? setup({ name, ext: EXT }) : setup();
  const rt = new Runtime(t.d);
  const ref = collect(build(rt), rt);
  return { ref, page: html(ref) };
};

describe("reference page", () => {
  it("uses the tokens and fonts of the landing page", () => {
    // The 3 @font-face lines, the :root block and the phone gutter line that follows it.
    const brand = site.match(/@font-face[\s\S]*?\n\}\n[^\n]*\n/)?.[0] ?? "";
    expect(brand.match(/@font-face/g)).toHaveLength(3);
    expect(brand).toContain("--dark-green:#002420");
    expect(brand).toContain("--gutter:1rem");
    const logo = site.match(/<symbol id="logo-mark"[\s\S]*?<\/symbol>/)?.[0] ?? "";
    expect(logo).toContain("<path");
    const { page } = pageOf("purchasely");
    expect(page).toContain(brand);
    expect(page).toContain(logo);
  });

  it("has the brand nav, a group index and an anchor per command", () => {
    const { ref, page } = pageOf("acme");
    expect(page).toContain('class="brand"');
    expect(page).toContain('<span class="tag">CLI</span>');
    for (const a of ["/#who", "/#use-cases", "/#mcp", "/#agents", "/#safety", "/#install", "https://docs.purchasely.com/docs/management-api"]) {
      expect(page).toContain(`href="${a}"`);
    }
    expect(page).toContain('href="assets/favicon.png"');
    const index = page.match(/<nav class="index"[\s\S]*?<\/nav>/)?.[0] ?? "";
    expect(index).toContain('href="#extension-commands"');
    expect(ref.groups.some((g) => g.extension)).toBe(true);
    for (const g of ref.groups) {
      expect(index).toContain(`<a href="#${g.id}">${g.name}</a> <span class="soft">${g.leaves.length}</span>`);
      for (const l of g.leaves) expect(page).toContain(`<article id="${l.id}">\n<h3><a href="#${l.id}">`);
    }
  });

  it("has no script and no filter input", () => {
    for (const name of ["purchasely", "acme"] as const) {
      const { page } = pageOf(name);
      expect(page).not.toMatch(/<script/);
      expect(page).not.toMatch(/<input/);
    }
  });
});
