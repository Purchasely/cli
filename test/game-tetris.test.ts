import { describe, expect, it } from "vitest";
import tetris from "../src/games/tetris.js";

const I = 0;
const O = 1;
// A rand that gives these pieces in turn.
const seq = (...ts: number[]) => {
  let i = 0;
  return () => (ts[i++ % ts.length] + 0.5) / 7;
};
// The well rows as text: "#" for a block, "." for an empty cell.
const well = (lines: string[]) =>
  lines.slice(0, 20).map((l) => l.replace(/\x1b\[[\d;]*m/g, "").slice(1, 21).replaceAll("██", "#").replaceAll(" .", "."));
const keys = (g: ReturnType<typeof tetris>, ...ks: string[]) => ks.forEach((k) => g.key(k));

describe("tetris", () => {
  it("refuses a small terminal", () => {
    expect(() => tetris(30, 10)).toThrow(/too small for tetris/);
  });

  it("hard-drops a piece to the floor", () => {
    const g = tetris(80, 24, seq(I));
    g.key("space");
    expect(well(g.draw())[19]).toBe("...####...");
    expect(g.draw().join("\n")).toContain("q to quit");
  });

  it("clears a full line and scores it", () => {
    const g = tetris(80, 24, seq(I, I, O));
    keys(g, "left", "left", "left", "space", "right", "right", "right", "space", "space");
    const rows = well(g.draw());
    expect(rows[19]).toBe("....##....");
    expect(g.draw().join("\n")).toMatch(/Score 100[\s\S]*Lines 1/);
  });

  it("kicks a rotation away from the wall", () => {
    const g = tetris(80, 24, seq(I));
    keys(g, "up", "left", "left", "left", "left", "left", "up", "space");
    expect(well(g.draw())[19]).toBe("####......");
  });

  it("ends when a new piece cannot spawn", () => {
    const g = tetris(80, 24, seq(O));
    for (let i = 0; i < 20 && !g.over; i++) g.key("space");
    expect(g.over).toBe("Game over. Score 0, 0 lines.");
  });
});
