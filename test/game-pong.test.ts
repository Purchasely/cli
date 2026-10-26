import { describe, expect, it } from "vitest";
import pong from "../src/games/pong.js";

// pong(50, 13) gives a court of 40x10. draw(): line 0 is the score, line 1 the top border, lines 2-11 the court.
const court = (g: ReturnType<typeof pong>) => g.draw().slice(2, 12).map((l) => l.replace(/\x1b\[[\d;]+m/g, ""));
const paddle = (g: ReturnType<typeof pong>) => court(g).flatMap((l, r) => (l[2] === "█" ? [r] : []));
const ball = (g: ReturnType<typeof pong>) => court(g).findIndex((l) => l.includes("P"));

describe("pong", () => {
  it("refuses a small terminal", () => {
    expect(() => pong(20, 10)).toThrow(/too small for pong/);
  });

  it("keeps the paddle in the court", () => {
    const g = pong(50, 13, () => 0.5);
    for (let i = 0; i < 20; i++) g.key("down");
    expect(paddle(g)).toEqual([6, 7, 8, 9]);
    for (let i = 0; i < 20; i++) g.key("w");
    expect(paddle(g)).toEqual([0, 1, 2, 3]);
  });

  it("bounces the ball on the top wall", () => {
    const g = pong(50, 13, () => 0);
    const rows: number[] = [];
    for (let i = 0; i < 15; i++) {
      g.step();
      rows.push(ball(g));
    }
    expect(rows).toContain(0);
    expect(rows.at(-1)).toBeGreaterThan(0);
  });

  it("bounces the ball on the paddle", () => {
    const g = pong(50, 13, () => 0.5);
    for (let i = 0; i < 30; i++) g.step();
    expect(g.draw()[0]).toContain("You 0 : 0 CPU");
    expect(court(g)[ball(g)].indexOf("P")).toBeGreaterThan(3);
  });

  it("scores a miss and ends at 7", () => {
    const g = pong(50, 13, () => 0.5);
    for (let i = 0; i < 5; i++) g.key("up");
    for (let i = 0; i < 25; i++) g.step();
    expect(g.draw()[0]).toContain("You 0 : 1 CPU");
    for (let i = 0; i < 1000 && !g.over; i++) g.step();
    expect(g.over).toBe("You lose 0-7");
  });
});
