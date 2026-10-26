import { describe, expect, it } from "vitest";
import { menu } from "../src/games/term.js";
import { setup } from "./helpers.js";

describe("games", () => {
  it("is in the help, and play is not", async () => {
    const out = (await setup().exec("--help")).out;
    expect(out).toMatch(/^ {2}games \[game\] +Play a game in the terminal$/m);
    expect(out).not.toMatch(/^ {2}play\b/m);
    expect(out).not.toContain("games|play");
  });

  it("the help lists the games, the keys and the limits", async () => {
    const out = (await setup().exec("games", "--help")).out;
    for (const word of ["2048", "invaders", "minesweeper", "pong", "snake", "tetris", "arrows or WASD", "space", "Esc", "Ctrl-C", "needs an interactive terminal", "uses no key and sends no request"]) {
      expect(out, word).toContain(word);
    }
  });

  it("refuses an unknown game before any key or request, under both names and in both programs", async () => {
    for (const name of ["purchasely", "acme"] as const) {
      for (const cmd of ["games", "play"]) {
        const t = setup({ env: { PURCHASELY_API_KEY: "" }, name });
        const r = await t.exec(cmd, "nope");
        expect(r.code, `${name} ${cmd}`).toBe(2);
        expect(r.errJson().error.messages[0]).toContain('Unknown game "nope"');
        expect(r.err).not.toContain("env=");
        expect(t.calls).toEqual([]);
      }
    }
  });

  it("needs a terminal for the menu", async () => {
    for (const cmd of ["games", "play"]) {
      const t = setup({ env: { PURCHASELY_API_KEY: "" } });
      const r = await t.exec(cmd);
      expect(r.code, cmd).toBe(2);
      expect(r.errJson().error.messages[0]).toBe(`${cmd} needs an interactive terminal.`);
      expect(r.err).not.toContain("env=");
    }
  });

  it("the menu wraps and returns the chosen name", () => {
    const g = menu(["2048", "pong", "snake"])(80, 24);
    g.key("up");
    const plain = g.draw().map((l) => l.replace(/\x1b\[[\d;]+m/g, ""));
    expect(plain).toContain("  P snake ");
    expect(plain).toContain("    pong");
    expect(plain.slice(1, 6).join("\n")).toContain("███  █  █ ███");
    g.key("down");
    g.key("s");
    expect(g.over).toBeUndefined();
    g.key("return");
    expect(g.over).toBe("pong");
  });
});
