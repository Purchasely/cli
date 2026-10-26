import { describe, expect, it } from "vitest";
import snake from "../src/games/snake.js";

// 22x8 is the smallest terminal: a 10x5 board, the head at (5,2), the food at (0,0) with rand 0.
const game = () => snake(22, 8, () => 0);
const screen = (g: ReturnType<typeof game>) => g.draw().join("\n").replace(/\x1b\[[\d;]+m/g, "");
const steps = (g: ReturnType<typeof game>, n: number) => {
  for (let i = 0; i < n; i++) g.step();
};

describe("snake", () => {
  it("moves one cell per step and dies on the wall", () => {
    const g = game();
    steps(g, 4);
    expect(g.over).toBeUndefined();
    g.step();
    expect(g.over).toBe("Game over. Score: 0.");
  });

  it("eats, grows and scores", () => {
    const g = game();
    g.key("w");
    steps(g, 2);
    g.key("left");
    steps(g, 5);
    expect(g.over).toBeUndefined();
    expect(screen(g)).toContain("Score: 1");
    expect(screen(g).match(/██/g)).toHaveLength(4);
  });

  it("does not reverse onto itself", () => {
    const g = game();
    g.key("left");
    g.step();
    expect(g.over).toBeUndefined();
    g.key("up");
    g.key("left");
    g.step();
    expect(g.over).toBeUndefined();
  });

  it("draws a border, the board and a status line", () => {
    expect(screen(game()).split("\n")).toEqual([
      `┌${"─".repeat(20)}┐`,
      `│P ${" ".repeat(18)}│`,
      `│${" ".repeat(20)}│`,
      `│      ██████        │`,
      `│${" ".repeat(20)}│`,
      `│${" ".repeat(20)}│`,
      `└${"─".repeat(20)}┘`,
      "Purchasely Snake   Score: 0   q to quit",
    ]);
  });

  it("refuses a small terminal", () => {
    expect(() => snake(21, 8)).toThrow("too small for snake");
  });
});
