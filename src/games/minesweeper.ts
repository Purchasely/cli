import { usage } from "../core/errors.js";
import { BRAND, fg, type Game } from "./term.js";

const MOVES: Record<string, [number, number]> = {
  up: [0, -1], w: [0, -1], down: [0, 1], s: [0, 1], left: [-1, 0], a: [-1, 0], right: [1, 0], d: [1, 0],
};
// The classic colours of 1 to 8.
const COLOURS = ["", "94", "32", "91", "34", "31", "36", "35", "90"];

/** Minesweeper: up to 16x16 with 40 mines, the same density on a smaller terminal. The first reveal is never a mine. */
export default function minesweeper(cols: number, rows: number, rand = Math.random, now = Date.now): Game {
  // Each cell is 2 characters wide; the status line needs 20 characters and 1 row.
  if (cols < 20 || rows < 9) throw usage(`The terminal is too small for minesweeper: it needs 20x9 characters, it has ${cols}x${rows}.`);
  const w = Math.min(16, Math.floor(cols / 2));
  const h = Math.min(16, rows - 1);
  const n = Math.round((w * h * 40) / 256);
  const mine = new Array<boolean>(w * h).fill(false);
  const open = new Array<boolean>(w * h).fill(false);
  const flag = new Array<boolean>(w * h).fill(false);
  let cx = w >> 1;
  let cy = h >> 1;
  let start = 0;
  let opened = 0;

  const around = (i: number) => {
    const x = i % w;
    const y = (i - x) / w;
    const r: number[] = [];
    for (let dy = -1; dy <= 1; dy++)
      for (let dx = -1; dx <= 1; dx++)
        if ((dx || dy) && x + dx >= 0 && x + dx < w && y + dy >= 0 && y + dy < h) r.push(i + dy * w + dx);
    return r;
  };
  const count = (i: number) => around(i).filter((j) => mine[j]).length;

  const place = (first: number) => {
    const safe = new Set([first, ...around(first)]);
    const cells = [...mine.keys()].filter((i) => !safe.has(i));
    for (let k = 0; k < n; k++) {
      const j = k + Math.floor(rand() * (cells.length - k));
      [cells[k], cells[j]] = [cells[j], cells[k]];
      mine[cells[k]] = true;
    }
    start = now();
  };

  const reveal = (i: number) => {
    if (open[i] || flag[i]) return;
    if (!opened) place(i);
    if (mine[i]) {
      g.over = "Boom! You hit a mine.";
      return;
    }
    const todo = [i];
    while (todo.length) {
      const j = todo.pop()!;
      if (open[j] || flag[j]) continue;
      open[j] = true;
      opened++;
      if (!count(j)) todo.push(...around(j));
    }
    if (opened === w * h - n) g.over = `You cleared the board in ${Math.round((now() - start) / 1000)} s.`;
  };

  const cell = (i: number) => {
    // The flag is the Purchasely "P".
    if (flag[i]) return fg(BRAND.coral, "P");
    if (!open[i]) return "\x1b[2m·\x1b[22m";
    const c = count(i);
    return c ? `\x1b[${COLOURS[c]}m${c}\x1b[39m` : " ";
  };

  const g: Game = {
    tick: 0,
    key(name) {
      const m = MOVES[name];
      const i = cy * w + cx;
      if (m) {
        cx = Math.max(0, Math.min(w - 1, cx + m[0]));
        cy = Math.max(0, Math.min(h - 1, cy + m[1]));
      } else if (name === "space" || name === "return") reveal(i);
      else if (name === "f" && !open[i]) flag[i] = !flag[i];
    },
    step() {},
    draw() {
      const lines: string[] = [];
      for (let y = 0; y < h; y++) {
        let line = "";
        for (let x = 0; x < w; x++) {
          const c = `${cell(y * w + x)} `;
          line += x === cx && y === cy ? `\x1b[7m${c}\x1b[27m` : c;
        }
        lines.push(line);
      }
      lines.push(`${cols >= 32 ? `${fg(BRAND.lavender, "Purchasely")}  ` : ""}Mines ${n - flag.filter(Boolean).length}  q to quit`);
      return lines;
    },
  };
  return g;
}
