import { usage } from "../core/errors.js";
import { BRAND, bg, fg, type Game } from "./term.js";

type Dir = "left" | "right" | "up" | "down";
const DIRS: Dir[] = ["left", "right", "up", "down"];
const KEYS: Record<string, Dir> = { left: "left", right: "right", up: "up", down: "down", a: "left", d: "right", w: "up", s: "down" };
// Maps (line i, position j along the move) to (row, col), so every direction slides its lines to the left.
const AT: Record<Dir, (i: number, j: number) => [number, number]> = {
  left: (i, j) => [i, j],
  right: (i, j) => [i, 3 - j],
  up: (i, j) => [j, i],
  down: (i, j) => [3 - j, i],
};
const CW = 8; // Fits 131072, the largest possible tile.
const WIDTH = 4 * CW + 3;
const HEIGHT = 1 + 4 * 3 + 3 + 1;
// The Purchasely colours, in turn by log2 of the value. The empty cell is dark sage.
const TILES = [BRAND.lightGreen, BRAND.cream, BRAND.sage, BRAND.lavender, BRAND.coral, "#deb8d8", BRAND.white];
const tile = (v: number) => (v ? TILES[(Math.log2(v) - 1) % TILES.length] : "#3c5852");

/** Slides one line to the left and merges equal tiles once. Returns the new line and the points it scores. */
export function slide(line: number[]): [number[], number] {
  const t = line.filter(Boolean);
  const out: number[] = [];
  let score = 0;
  for (let i = 0; i < t.length; i++) {
    if (t[i] === t[i + 1]) {
      out.push(t[i] * 2);
      score += t[i] * 2;
      i++;
    } else out.push(t[i]);
  }
  return [[...out, ...Array<number>(line.length - out.length).fill(0)], score];
}

function move(grid: number[][], dir: Dir): [number[][], number] {
  const at = AT[dir];
  const out = grid.map((r) => [...r]);
  let score = 0;
  for (let i = 0; i < 4; i++) {
    const [line, s] = slide([0, 1, 2, 3].map((j) => grid[at(i, j)[0]][at(i, j)[1]]));
    score += s;
    line.forEach((v, j) => {
      const [r, c] = at(i, j);
      out[r][c] = v;
    });
  }
  return [out, score];
}

const cell = (v: number, k: number) => {
  const s = k === 1 && v ? String(v) : "";
  const l = (CW - s.length) >> 1;
  return bg(tile(v), fg(BRAND.darkGreen, `\x1b[1m${" ".repeat(l)}${s}${" ".repeat(CW - l - s.length)}\x1b[22m`));
};

/** 2048: slide the tiles with the arrows or WASD and merge equal tiles. */
export default function make2048(cols: number, rows: number, rand = Math.random): Game {
  if (cols < WIDTH || rows < HEIGHT) throw usage(`The terminal is too small for 2048: it needs ${WIDTH}x${HEIGHT}, it has ${cols}x${rows}.`);
  let grid = [0, 1, 2, 3].map(() => [0, 0, 0, 0]);
  let score = 0;
  // A real move always leaves an empty cell: a slide needs one and a merge frees one.
  const spawn = () => {
    const empty = grid.flatMap((r, i) => r.flatMap((v, j) => (v ? [] : [[i, j]])));
    const [i, j] = empty[Math.floor(rand() * empty.length)];
    grid[i][j] = rand() < 0.9 ? 2 : 4;
  };
  spawn();
  spawn();
  const best = () => Math.max(...grid.flat());
  const g: Game = {
    tick: 0,
    key(name) {
      const dir = KEYS[name];
      if (!dir) return;
      const [next, s] = move(grid, dir);
      if (next.join() === grid.join()) return;
      grid = next;
      score += s;
      spawn();
      if (DIRS.every((d) => move(grid, d)[0].join() === grid.join())) g.over = `Game over. Score ${score}, best tile ${best()}.`;
    },
    step() {},
    draw() {
      const pad = " ".repeat((cols - WIDTH) >> 1);
      const out = [pad + fg(BRAND.lavender, "PURCHASELY 2048")];
      out.push(...grid.flatMap((row, i) => [...(i ? [""] : []), ...[0, 1, 2].map((k) => pad + row.map((v) => cell(v, k)).join(" "))]));
      out.push(`${pad}Score ${score}${best() >= 2048 ? "  You win!" : ""}  q to quit`);
      return out;
    },
  };
  return g;
}
