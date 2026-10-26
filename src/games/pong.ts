import { usage } from "../core/errors.js";
import { BRAND, fg, type Game } from "./term.js";

const P = 4; // Paddle height.
const STEP = 2; // Rows per key press: the terminal sends key repeats, never a key-up.
const CPU = 0.5; // Rows per frame for the computer, slower than a steep ball: it can be beaten.
const WIN = 7;

/** Pong against the computer: up/down or w/s move the left paddle. First to 7 points wins. */
export default function pong(cols: number, rows: number, rand = Math.random): Game {
  // The court, inside the border, under the score line.
  const w = Math.min(cols - 2, 78);
  const h = Math.min(rows - 3, 22);
  // 44: the score line must fit in the top border.
  if (w < 44 || h < 10) throw usage(`The terminal is too small for pong: ${cols}x${rows}, it needs 46x13.`);
  const L = 1;
  const R = w - 2;
  let me = 0;
  let cpu = 0;
  let py = (h - P) >> 1;
  let cy = py;
  let x = 0;
  let y = 0;
  let vx = 0;
  let vy = 0;
  const serve = (dir: number) => {
    x = w >> 1;
    y = h >> 1;
    vx = dir;
    vy = rand() - 0.5;
  };
  serve(rand() < 0.5 ? 1 : -1);

  const game: Game = {
    tick: 40,
    key(k) {
      if (k === "up" || k === "w") py = Math.max(0, py - STEP);
      else if (k === "down" || k === "s") py = Math.min(h - P, py + STEP);
    },
    step() {
      cy = Math.max(0, Math.min(h - P, cy + Math.max(-CPU, Math.min(CPU, y - (P - 1) / 2 - cy))));
      y += vy;
      if (y < 0 || y > h - 1) {
        vy = -vy;
        y = y < 0 ? -y : 2 * (h - 1) - y;
      }
      const nx = x + vx;
      const off = Math.round(y) - (nx === L ? py : nx === R ? Math.round(cy) : NaN);
      // The bounce angle comes from where the ball hits the paddle: the edges send it steepest.
      if (off >= 0 && off < P) {
        vx = -vx;
        vy = (off - (P - 1) / 2) / 2;
      } else x = nx;
      if (x >= 0 && x < w) return;
      if (x < 0) cpu++;
      else me++;
      if (me === WIN || cpu === WIN) game.over = `You ${me > cpu ? "win" : "lose"} ${me}-${cpu}`;
      else serve(x < 0 ? -1 : 1);
    },
    draw() {
      const g = Array.from({ length: h }, (_, r) => Array.from({ length: w }, (_, c): string => (c === w >> 1 && r % 2 === 0 ? "│" : " ")));
      for (let i = 0; i < P; i++) {
        g[py + i][L] = fg(BRAND.lightGreen, "█");
        g[Math.round(cy) + i][R] = fg(BRAND.lavender, "█");
      }
      if (x >= 0 && x < w) g[Math.round(y)][x] = fg(BRAND.coral, "P");
      const score = ` Purchasely Pong   You ${me} : ${cpu} CPU`;
      return [
        score + "q to quit ".padStart(w + 2 - score.length),
        `┌${"─".repeat(w)}┐`,
        ...g.map((r) => `│${r.join("")}│`),
        `└${"─".repeat(w)}┘`,
      ];
    },
  };
  return game;
}
