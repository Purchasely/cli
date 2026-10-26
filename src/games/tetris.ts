import { usage } from "../core/errors.js";
import { BRAND, fg, type Game } from "./term.js";

const W = 10;
const H = 20;
const MIN_COLS = 40;
const MIN_ROWS = H + 1;

type Cell = [number, number];
// The cells sit in an n x n box: a rotation turns the box. The colours are the Purchasely colours.
const PIECES: { n: number; cells: Cell[]; color: string }[] = [
  { n: 4, cells: [[0, 1], [1, 1], [2, 1], [3, 1]], color: BRAND.lightGreen }, // I
  { n: 2, cells: [[0, 0], [1, 0], [0, 1], [1, 1]], color: BRAND.cream }, // O
  { n: 3, cells: [[1, 0], [0, 1], [1, 1], [2, 1]], color: BRAND.lavender }, // T
  { n: 3, cells: [[1, 0], [2, 0], [0, 1], [1, 1]], color: BRAND.sage }, // S
  { n: 3, cells: [[0, 0], [1, 0], [1, 1], [2, 1]], color: BRAND.coral }, // Z
  { n: 3, cells: [[0, 0], [0, 1], [1, 1], [2, 1]], color: BRAND.white }, // J
  { n: 3, cells: [[2, 0], [0, 1], [1, 1], [2, 1]], color: "#deb8d8" }, // L: a mix of coral and lavender
];
const POINTS = [0, 100, 300, 500, 800];
const KICKS = [0, -1, 1, -2, 2];

const paint = (t: number) => fg(PIECES[t].color, "██");

/** Tetris in a 10x20 well. `rand` picks the pieces. */
export default function tetris(cols: number, rows: number, rand = Math.random): Game {
  if (cols < MIN_COLS || rows < MIN_ROWS) {
    throw usage(`The terminal is too small for tetris: ${cols}x${rows}, it needs ${MIN_COLS}x${MIN_ROWS}.`);
  }
  const board: number[][] = Array.from({ length: H }, () => Array(W).fill(-1));
  const pick = () => Math.floor(rand() * PIECES.length);
  let next = pick();
  let t = 0;
  let cells: Cell[] = [];
  let x = 0;
  let y = 0;
  let score = 0;
  let lines = 0;
  let frames = 0;
  const level = () => 1 + Math.floor(lines / 10);

  // Out of the well, board[..]?.[..] is undefined: the piece does not fit.
  const fits = (cs: Cell[], dx: number, dy: number) => cs.every(([cx, cy]) => board[y + dy + cy]?.[x + dx + cx] === -1);
  const move = (dx: number, dy: number) => {
    if (!fits(cells, dx, dy)) return false;
    x += dx;
    y += dy;
    return true;
  };
  const spawn = () => {
    t = next;
    next = pick();
    cells = PIECES[t].cells.map(([a, b]) => [a, b]);
    x = Math.floor((W - PIECES[t].n) / 2);
    y = 0;
    if (!fits(cells, 0, 0)) g.over = `Game over. Score ${score}, ${lines} lines.`;
  };
  const lock = () => {
    for (const [cx, cy] of cells) board[y + cy][x + cx] = t;
    const kept = board.filter((r) => r.includes(-1));
    const n = H - kept.length;
    board.splice(0, H, ...Array.from({ length: n }, () => Array(W).fill(-1)), ...kept);
    score += POINTS[n];
    lines += n;
    frames = 0;
    spawn();
  };
  const rotate = () => {
    const n = PIECES[t].n;
    const turned: Cell[] = cells.map(([a, b]) => [n - 1 - b, a]);
    const dx = KICKS.find((k) => fits(turned, k, 0));
    if (dx === undefined) return;
    cells = turned;
    x += dx;
  };

  const g: Game = {
    // play() reads tick once: the gravity counts frames, so the level can change the speed.
    tick: 50,
    key(name) {
      if (name === "left") move(-1, 0);
      else if (name === "right") move(1, 0);
      else if (name === "up") rotate();
      else if (name === "down" && !move(0, 1)) lock();
      else if (name === "space") {
        while (move(0, 1));
        lock();
      }
    },
    step() {
      if (++frames < Math.max(1, 17 - level())) return;
      frames = 0;
      if (!move(0, 1)) lock();
    },
    draw() {
      const grid = board.map((r) => [...r]);
      for (const [cx, cy] of cells) grid[y + cy][x + cx] = t;
      const preview = [0, 1].map((py) =>
        [0, 1, 2, 3].map((px) => (PIECES[next].cells.some(([a, b]) => a === px && b === py) ? paint(next) : "  ")).join(""),
      );
      const side = [fg(BRAND.lavender, "PURCHASELY"), "Tetris", "", "Next", ...preview, "", `Score ${score}`, `Lines ${lines}`, `Level ${level()}`, "",
        "←→ move", "↑ rotate", "↓ soft drop", "space hard drop", "q to quit"];
      return [
        ...grid.map((r, i) => `│${r.map((c) => (c < 0 ? " ." : paint(c))).join("")}│  ${side[i] ?? ""}`),
        `└${"──".repeat(W)}┘`,
      ];
    },
  };
  spawn();
  return g;
}
