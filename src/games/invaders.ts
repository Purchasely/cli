import { usage } from "../core/errors.js";
import { BRAND, fg, type Game } from "./term.js";

const ROWS = 5;
// Each row of invaders spells the brand.
const NAME = "PURCHASELY";
const COLS = NAME.length;
/** The colour of each invader row, top first. The top rows score more. */
const COLOUR = [BRAND.coral, BRAND.lavender, BRAND.cream, BRAND.lightGreen, BRAND.sage];

type Spot = { x: number; y: number };

/** Space Invaders: arrows or a/d move the ship, space fires. `rand` drops the bombs, to make the tests deterministic. */
export default function invaders(cols: number, rows: number, rand = Math.random): Game {
  // Two columns for the side walls, one line for the status and one for the ground.
  const w = Math.min(cols - 2, 50);
  const h = Math.min(rows - 2, 22);
  if (w < 24 || h < 12) throw usage(`The terminal is too small for invaders: ${cols}x${rows}, it needs 26x14.`);
  const shipY = h - 1;
  let ship = w >> 1, score = 0, lives = 3, wave = 0, dir = 1, clock = 0, frame = 0;
  let army: (Spot & { row: number; c: string })[] = [], shots: Spot[] = [], bombs: Spot[] = [];

  const nextWave = () => {
    wave++;
    army = [];
    for (let row = 0; row < ROWS; row++) for (let i = 0; i < COLS; i++) army.push({ x: i * 2, y: row + 1, row, c: NAME[i] });
    shots = [];
    bombs = [];
    dir = 1;
  };
  const hit = () => {
    shots = shots.filter((s) => {
      const i = army.findIndex((a) => a.x === s.x && a.y === s.y);
      if (i < 0) return true;
      score += (ROWS - army[i].row) * 10;
      army.splice(i, 1);
      return false;
    });
  };
  nextWave();

  const g: Game = {
    // `play` reads `tick` once: the speed comes from the frames between two marches.
    tick: 50,
    key(name) {
      if (name === "left" || name === "a") ship = Math.max(0, ship - 1);
      else if (name === "right" || name === "d") ship = Math.min(w - 1, ship + 1);
      else if (name === "space" && shots.length < 2) shots.push({ x: ship, y: shipY - 1 });
    },
    step() {
      frame++;
      shots = shots.filter((s) => --s.y >= 0);
      hit();
      const period = Math.max(2, Math.round((Math.max(6, 20 - 2 * wave) * army.length) / (ROWS * COLS)));
      if (army.length && ++clock >= period) {
        clock = 0;
        if (army.some((a) => a.x + dir < 0 || a.x + dir >= w)) {
          dir = -dir;
          for (const a of army) a.y++;
        } else for (const a of army) a.x += dir;
        hit();
        if (army.some((a) => a.y >= shipY)) {
          g.over = `The invaders landed. Game over. Score ${score}, wave ${wave}.`;
          return;
        }
      }
      if (frame % 2 === 0) bombs = bombs.filter((b) => ++b.y <= shipY);
      if (army.length && rand() < 0.02 + 0.005 * wave) {
        const a = army[Math.floor(rand() * army.length)];
        bombs.push({ x: a.x, y: a.y + 1 });
      }
      if (bombs.some((b) => b.x === ship && b.y === shipY)) {
        bombs = [];
        if (--lives === 0) g.over = `Game over. Score ${score}, wave ${wave}.`;
      }
      if (!army.length) nextWave();
    },
    draw() {
      const field = Array.from({ length: h }, () => Array<string>(w).fill(" "));
      for (const a of army) field[a.y][a.x] = fg(COLOUR[a.row], a.c);
      for (const s of shots) field[s.y][s.x] = "|";
      for (const b of bombs) field[b.y][b.x] = fg(BRAND.coral, "!");
      field[shipY][ship] = fg(BRAND.lightGreen, "▲");
      return [
        `${fg(BRAND.lavender, "Purchasely Invaders")}   Score ${score}   Lives ${lives}   Wave ${wave}   q to quit`,
        ...field.map((r) => `│${r.join("")}│`),
        `└${"─".repeat(w)}┘`,
      ];
    },
  };
  return g;
}
