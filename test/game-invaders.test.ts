import { describe, expect, it } from "vitest";
import invaders from "../src/games/invaders.js";
import type { Game } from "../src/games/term.js";

const plain = (g: Game) => g.draw().map((l) => l.replace(/\x1b\[[\d;]+m/g, ""));
const score = (g: Game) => Number(/Score (\d+)/.exec(plain(g)[0])![1]);
const count = (g: Game) => plain(g).slice(1).join("").match(/[PURCHASELY]/g)?.length ?? 0;
const shipLine = (g: Game) => plain(g).find((l) => l.includes("▲"))!;
const noBomb = () => 0.99;

describe("invaders", () => {
  it("refuses a small terminal", () => {
    expect(() => invaders(20, 10)).toThrow(/too small for invaders/);
  });

  it("keeps the ship in the field", () => {
    const g = invaders(40, 24, noBomb);
    for (let i = 0; i < 60; i++) g.key("left");
    expect(shipLine(g)).toMatch(/^│▲/);
    for (let i = 0; i < 60; i++) g.key("d");
    expect(shipLine(g)).toMatch(/▲│$/);
  });

  it("kills an invader with a shot and scores", () => {
    const g = invaders(40, 24, noBomb);
    for (let i = 0; i < 300 && !score(g); i++) {
      g.key("space");
      g.step();
    }
    expect(score(g)).toBeGreaterThan(0);
    // Two shots can hit in the same step.
    expect(count(g)).toBeLessThan(50);
  });

  it("takes a life when a bomb hits the ship", () => {
    const g = invaders(40, 24, () => 0);
    for (let i = 0; i < 30; i++) g.key("left");
    for (let i = 0; i < 100 && !plain(g)[0].includes("Lives 2"); i++) g.step();
    expect(plain(g)[0]).toContain("Lives 2");
  });

  it("ends the game when the invaders land", () => {
    const g = invaders(40, 24, noBomb);
    for (let i = 0; i < 1e5 && !g.over; i++) g.step();
    expect(g.over).toBe("The invaders landed. Game over. Score 0, wave 1.");
  });

  it("starts a new wave when the last invader dies", () => {
    const g = invaders(40, 24, noBomb);
    for (let i = 0; i < 1e5 && !g.over && !plain(g)[0].includes("Wave 2"); i++) {
      // Aim the ship below the lowest invader.
      const lines = plain(g);
      const target = lines.slice(1).findLast((l) => /[PURCHASELY]/.test(l))!.search(/[PURCHASELY]/);
      const ship = shipLine(g).indexOf("▲");
      g.key(target < ship ? "left" : target > ship ? "right" : "space");
      g.step();
    }
    expect(g.over).toBeUndefined();
    expect(plain(g)[0]).toContain("Wave 2");
    expect(count(g)).toBe(50);
  });
});
