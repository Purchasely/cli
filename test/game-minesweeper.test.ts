import { describe, expect, it } from "vitest";
import minesweeper from "../src/games/minesweeper.js";
import type { Game } from "../src/games/term.js";

// A 20x9 terminal: a 10x8 board with 13 mines, the cursor at (5, 4).
const make = (rand: () => number, now?: () => number) => minesweeper(20, 9, rand, now);
const seeded = (s: number) => () => (s = (s * 16807) % 2147483647) / 2147483647;
const plain = (g: Game) => g.draw().map((l) => l.replace(/\x1b\[[\d;]+m/g, ""));
// One character per cell: the second character of each cell is a space.
const board = (g: Game) => plain(g).slice(0, -1).map((l) => l.replace(/(.)./g, "$1"));
const go = (g: Game, x: number, y: number) =>
  [...Array(10).fill("a"), ...Array(8).fill("w"), ...Array(x).fill("d"), ...Array(y).fill("s")].forEach((k) => g.key(k));

describe("minesweeper", () => {
  it("makes the first reveal safe and flood-fills its area", () => {
    for (let seed = 1; seed <= 20; seed++) {
      const g = make(seeded(seed));
      g.key("space");
      expect(g.over ?? "").not.toContain("Boom");
      for (const row of board(g).slice(3, 6)) expect(row.slice(4, 7)).not.toContain("·");
    }
  });

  it("blocks a reveal on a flag", () => {
    const g = make(seeded(1));
    g.key("f");
    g.key("return");
    expect(board(g)[4][5]).toBe("P");
    expect(board(g).join("").replace(/[^·]/g, "")).toHaveLength(79);
    expect(plain(g).at(-1)).toBe("Mines 12  q to quit");
    g.key("f");
    expect(plain(g).at(-1)).toBe("Mines 13  q to quit");
  });

  it("ends the game on a mine", () => {
    const g = make(seeded(1));
    g.key("space");
    for (let i = 0; i < 80 && !g.over; i++) {
      if (board(g)[i / 10 | 0][i % 10] !== "·") continue;
      go(g, i % 10, i / 10 | 0);
      g.key("space");
    }
    expect(g.over).toBe("Boom! You hit a mine.");
  });

  it("wins when all the safe cells are open", () => {
    // rand 0 puts the mines in the top rows: one reveal in the far corner opens the rest.
    const t = [1_000, 43_000];
    const g = make(() => 0, () => t.shift()!);
    go(g, 9, 7);
    g.key("return");
    expect(g.over).toBe("You cleared the board in 42 s.");
  });

  it("refuses a terminal that is too small", () => {
    expect(() => minesweeper(19, 30)).toThrow(/too small for minesweeper/);
  });
});
