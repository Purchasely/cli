import { describe, expect, it } from "vitest";
import make2048, { slide } from "../src/games/2048.js";

// A fixed LCG: the same game on each run.
const seeded = (s: number) => () => (s = (s * 1103515245 + 12345) % 2 ** 31) / 2 ** 31;

describe("2048", () => {
  it("merges equal tiles once per move", () => {
    expect(slide([2, 2, 2, 2])).toEqual([[4, 4, 0, 0], 8]);
    expect(slide([2, 2, 4, 0])).toEqual([[4, 4, 0, 0], 4]);
    expect(slide([0, 4, 0, 4])).toEqual([[8, 0, 0, 0], 8]);
    expect(slide([2, 4, 8, 16])).toEqual([[2, 4, 8, 16], 0]);
  });

  it("spawns a tile only after a move that changes the grid", () => {
    // rand 0: each spawn is a 2 in the first empty cell, so the first row is [2, 2, 0, 0].
    const g = make2048(80, 24, () => 0);
    g.key("left"); // [4, 2, 0, 0] after the spawn
    const before = g.draw().join("\n");
    expect(before).toContain("Score 4");
    for (const k of ["left", "a", "up", "w", "x"]) g.key(k);
    expect(g.draw().join("\n")).toBe(before);
  });

  it("ends when no move is possible", () => {
    const g = make2048(80, 24, seeded(42));
    const keys = ["up", "left", "down", "right"];
    for (let n = 0; !g.over && n < 100_000; n++) g.key(keys[n % 4]);
    expect(g.over).toMatch(/^Game over\. Score \d+, best tile \d+\.$/);
  });

  it("refuses a terminal that is too small", () => {
    expect(() => make2048(20, 10)).toThrow(/too small for 2048/);
  });
});
