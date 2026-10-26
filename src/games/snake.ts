import { usage } from "../core/errors.js";
import { BRAND, bg, fg, type Game } from "./term.js";

type P = [x: number, y: number];

const DIRS: Record<string, P> = {
  up: [0, -1], w: [0, -1],
  down: [0, 1], s: [0, 1],
  left: [-1, 0], a: [-1, 0],
  right: [1, 0], d: [1, 0],
};

/** Snake: arrows or WASD steer, food makes the snake grow, a wall or the snake itself ends the game. */
export default function snake(cols: number, rows: number, rand = Math.random): Game {
  // A board cell is 2 characters wide: a terminal cell is about twice as tall as wide.
  const w = Math.min(30, Math.floor((cols - 2) / 2));
  const h = Math.min(20, rows - 3);
  if (w < 10 || h < 5) throw usage(`The terminal is too small for snake: it needs 22x8, it has ${cols}x${rows}.`);
  const body: P[] = [[w >> 1, h >> 1], [(w >> 1) - 1, h >> 1], [(w >> 1) - 2, h >> 1]];
  const find = (x: number, y: number) => body.findIndex((p) => p[0] === x && p[1] === y);
  const place = (): P | undefined => {
    const free: P[] = [];
    for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) if (find(x, y) < 0) free.push([x, y]);
    return free[Math.floor(rand() * free.length)];
  };
  let dir: P = [1, 0];
  let next = dir;
  let score = 0;
  let food = place()!;

  const g: Game = {
    // shortcut: constant speed, because play() reads tick only once; skip frames in step() to speed up.
    tick: 120,
    key(name) {
      const d = DIRS[name];
      // Not the opposite of the last move (not `next`): two quick keys must not turn the snake onto itself.
      if (d && (d[0] + dir[0] || d[1] + dir[1])) next = d;
    },
    step() {
      dir = next;
      const x = body[0][0] + dir[0];
      const y = body[0][1] + dir[1];
      const eat = x === food[0] && y === food[1];
      // The tail moves first, so the head can take its cell.
      if (!eat) body.pop();
      if (x < 0 || y < 0 || x >= w || y >= h || find(x, y) >= 0) {
        g.over = `Game over. Score: ${score}.`;
        return;
      }
      body.unshift([x, y]);
      if (!eat) return;
      score++;
      const f = place();
      if (f) food = f;
      else g.over = `You win! Score: ${score}.`;
    },
    draw() {
      const line = "─".repeat(2 * w);
      const cell = (x: number, y: number) => {
        const i = find(x, y);
        if (i >= 0) return fg(i ? BRAND.lightGreen : BRAND.lavender, "██");
        // The food is the Purchasely "P".
        return x === food[0] && y === food[1] ? bg(BRAND.coral, fg(BRAND.darkGreen, "P ")) : "  ";
      };
      const grid = Array.from({ length: h }, (_, y) => Array.from({ length: w }, (_, x) => cell(x, y)).join(""));
      return [`┌${line}┐`, ...grid.map((r) => `│${r}│`), `└${line}┘`, `${fg(BRAND.lavender, "Purchasely Snake")}   Score: ${score}   q to quit`];
    },
  };
  return g;
}
