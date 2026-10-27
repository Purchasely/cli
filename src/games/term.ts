import { emitKeypressEvents, type Key } from "node:readline";
import { CliError, EXIT, usage } from "../core/errors.js";

/** One game. `play` calls `key` on each key press and `step` each `tick` ms, then draws the screen. */
export interface Game {
  /** Milliseconds per frame. 0: a turn-based game, it changes only on a key. */
  tick: number;
  /** A readline key name: "up", "down", "left", "right", "space", "return", "a", "1"... */
  key(name: string): void;
  step(): void;
  /** The screen, one string per line. ANSI colours are allowed. */
  draw(): string[];
  /** The end message, with the score. When it is set, `play` stops the game and ends at the next key. */
  over?: string;
  /** The game takes text: `key` gets a printable character as typed (" ", "Q", "_"), and only Esc or Ctrl+C quits. */
  typing?: boolean;
}

/** Makes a game for a terminal of `cols` x `rows` characters. */
export type GameFactory = (cols: number, rows: number) => Game;

const CSI = "\x1b[";

/** The Purchasely colours, from site/BRAND.md. The light ones read well on a dark terminal. */
export const BRAND = {
  darkGreen: "#002420",
  lightGreen: "#ceecd0",
  lavender: "#bdbbff",
  coral: "#ffb6b2",
  cream: "#fff3d6",
  sage: "#90b09b",
  white: "#ffffff",
};
const rgb = (hex: string) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16)).join(";");
/** `s` in a 24-bit text colour. */
export const fg = (hex: string, s: string) => `${CSI}38;2;${rgb(hex)}m${s}${CSI}39m`;
/** `s` on a 24-bit background colour. */
export const bg = (hex: string, s: string) => `${CSI}48;2;${rgb(hex)}m${s}${CSI}49m`;

// Block letters, 4 columns x 5 rows.
const FONT: Record<string, string[]> = {
  P: ["███ ", "█  █", "███ ", "█   ", "█   "],
  U: ["█  █", "█  █", "█  █", "█  █", " ██ "],
  R: ["███ ", "█  █", "███ ", "█ █ ", "█  █"],
  C: [" ███", "█   ", "█   ", "█   ", " ███"],
  H: ["█  █", "█  █", "████", "█  █", "█  █"],
  A: [" ██ ", "█  █", "████", "█  █", "█  █"],
  S: [" ███", "█   ", " ██ ", "   █", "███ "],
  E: ["████", "█   ", "███ ", "█   ", "████"],
  L: ["█   ", "█   ", "█   ", "█   ", "████"],
  Y: ["█  █", "█  █", " ██ ", " █  ", " █  "],
};
// The hero gradient of purchasely.com, one colour per letter.
const GRADIENT = [BRAND.lightGreen, BRAND.lightGreen, BRAND.lightGreen, BRAND.lavender, BRAND.lavender, BRAND.lavender, BRAND.lavender, BRAND.coral, BRAND.coral, BRAND.coral];

/** "PURCHASELY" in coloured block letters: 5 lines of 49 columns. */
export const banner = (): string[] =>
  [0, 1, 2, 3, 4].map((r) => [..."PURCHASELY"].map((c, i) => fg(GRADIENT[i], FONT[c][r])).join(" "));

/**
 * A list of names: the typed letters keep the names that contain them (any case), the arrows choose, Enter sets `over` to the
 * chosen name and `index` to its place in `names`. Run it with `play(menu(names), false)`.
 * A list longer than the screen scrolls and keeps the choice in view.
 */
export const menu =
  (names: string[], { title = "Arcade", verb = "play" } = {}): ((cols: number, rows: number) => Game & { index?: number }) =>
  (cols, rows) => {
    let i = 0;
    let query = "";
    // The places in `names` of the names that contain the query.
    let shown = names.map((_, j) => j);
    const search = (q: string) => {
      query = q;
      shown = names.flatMap((n, j) => (n.toLowerCase().includes(q.toLowerCase()) ? [j] : []));
      i = 0;
    };
    const g: Game & { index?: number } = {
      tick: 0,
      typing: true,
      key(k) {
        const n = shown.length;
        if (k === "up") i = n ? (i + n - 1) % n : 0;
        else if (k === "down") i = n ? (i + 1) % n : 0;
        else if (k === "return" || k === "enter") {
          if (n) [g.over, g.index] = [names[shown[i]], shown[i]];
        } else if (k === "backspace") search(query.slice(0, -1));
        else if (k.length === 1) search(query + k);
      },
      step() {},
      draw: () => {
        const head = ["", ...(cols >= 51 && rows >= 16 ? banner().map((l) => ` ${l}`) : [fg(BRAND.lightGreen, " PURCHASELY")]), fg(BRAND.sage, ` ${title}`), ` ${fg(BRAND.sage, "Search:")} ${query}_`, ""];
        const room = Math.max(3, rows - head.length - 2);
        const top = Math.min(Math.max(0, i - (room >> 1)), Math.max(0, shown.length - room));
        const lines = shown.slice(top, top + room).map((at, j) => (top + j === i ? ` ${bg(BRAND.lavender, fg(BRAND.darkGreen, ` P ${names[at]} `))}` : `    ${names[at]}`));
        return [...head, ...(lines.length ? lines : ["    No match."]), "", fg(BRAND.sage, ` ↑↓ choose   Enter ${verb}   type to search   Esc quit`)];
      },
    };
    return g;
  };

type In = Pick<NodeJS.ReadStream, "on" | "off" | "resume" | "pause" | "isTTY"> & { setRawMode(raw: boolean): unknown };
type Out = Pick<NodeJS.WriteStream, "write" | "on" | "off" | "columns" | "rows" | "isTTY">;
/** The streams and the process that `play` uses. The tests give fakes. */
export interface Io {
  stdin: In;
  stdout: Out;
  proc: Pick<NodeJS.Process, "on" | "off">;
  exit: (code: number) => unknown;
}

const SIGNALS = { SIGHUP: 1, SIGINT: 2, SIGTERM: 15 } as const;

/**
 * Runs a game in the alternate screen until it ends or the player quits (q, Esc, Ctrl+C). Returns the end message.
 * `wait`: when the game ends, keep the final board on the screen until a key.
 * A signal (SIGTERM, SIGHUP, SIGINT from another process) restores the terminal and exits 128 + the signal number. A resize ends the game.
 */
export function play(make: GameFactory, wait = true, io: Partial<Io> = {}): Promise<string> {
  const { stdin = process.stdin, stdout = process.stdout, proc = process, exit = (c: number) => process.exit(c) } = io;
  return new Promise((resolve, reject) => {
    // The last row stays empty: a full last line would scroll the screen. The message counts that row.
    let g: Game;
    try {
      g = make(stdout.columns, stdout.rows - 1);
    } catch (e) {
      if (e instanceof CliError && e.exit === EXIT.USAGE) throw usage(e.message.replace(/(\d+)x(\d+)/g, (_, c, r) => `${c}x${+r + 1}`));
      throw e;
    }
    let timer: NodeJS.Timeout | undefined;
    const draw = () => stdout.write(`${CSI}H${g.draw().join(`${CSI}K\r\n`)}${CSI}K${CSI}J`);
    const signals = Object.entries(SIGNALS).map(([name, n]) => [name, () => (end(), exit(128 + n))] as const);
    const onResize = () => {
      g.over ??= "The terminal changed size.";
      end();
    };
    const end = (e?: unknown) => {
      clearInterval(timer);
      stdin.off("keypress", onKey);
      stdout.off("resize", onResize);
      signals.forEach(([name, f]) => proc.off(name as NodeJS.Signals, f));
      stdin.setRawMode(false);
      stdin.pause();
      stdout.write(`${CSI}0m${CSI}?25h${CSI}?1049l`);
      if (e) reject(e);
      else resolve(g.over ?? "Bye.");
    };
    // An exception must not leave the terminal in raw mode.
    const guard = (f: () => void) => {
      try {
        f();
        if (g.over && !wait) return end();
        draw();
        // The final board stays on the screen, with the message on the free last row, until a key.
        if (g.over) {
          clearInterval(timer);
          stdout.write(`${CSI}${stdout.rows};1H${g.over} Press a key.`);
        }
      } catch (e) {
        end(e);
      }
    };
    const onKey = (s: string | undefined, k: Key | undefined) => {
      // In a game that takes text, a printable character goes to it as typed, so q is a letter there: Esc quits.
      const typed = g.typing && s?.length === 1 && !/[\x00-\x1f\x7f-\x9f]/.test(s) && !k?.ctrl && !k?.meta;
      if (g.over || (k?.ctrl && k.name === "c") || (k?.name === "q" && !typed) || k?.name === "escape") return end();
      const name = typed ? s : (k?.name ?? s);
      if (name) guard(() => g.key(name));
    };
    emitKeypressEvents(stdin as NodeJS.ReadStream);
    stdin.setRawMode(true);
    stdin.resume();
    stdin.on("keypress", onKey);
    stdout.on("resize", onResize);
    signals.forEach(([name, f]) => proc.on(name as NodeJS.Signals, f));
    stdout.write(`${CSI}?1049h${CSI}?25l`);
    guard(() => {});
    if (g.tick) timer = setInterval(() => guard(() => g.step()), g.tick);
  });
}

/** A line that comes from a server: a control code in it would move the cursor or change the terminal. */
export const clean = (s: string) => s.replace(/[\x00-\x1f\x7f-\x9f]+/g, " ");
export const cut = (s: string, n: number) => (s.length > n ? `${s.slice(0, n - 3)}...` : s);

/**
 * A menu of lines in the alternate screen: the index of the chosen line, or undefined when the user quits.
 * A line is cleaned and cut to the width of the terminal. A resize closes the menu: `say` gets the message.
 */
export async function choose(lines: string[], opts: { title: string; verb: string }, io: Partial<Io>, say: (s: string) => void): Promise<number | undefined> {
  let chosen: ReturnType<ReturnType<typeof menu>> | undefined;
  const result = await play((cols, rows) => (chosen = menu(lines.map((l) => cut(clean(l), cols - 8)), opts)(cols, rows)), false, io);
  if (chosen?.index === undefined && result !== "Bye.") say(`${result}\n`);
  return chosen?.index;
}
