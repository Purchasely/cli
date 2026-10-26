import { EventEmitter } from "node:events";
import { describe, expect, it, vi } from "vitest";
import make2048 from "../src/games/2048.js";
import invaders from "../src/games/invaders.js";
import minesweeper from "../src/games/minesweeper.js";
import pong from "../src/games/pong.js";
import snake from "../src/games/snake.js";
import tetris from "../src/games/tetris.js";
import { play, type Game, type GameFactory } from "../src/games/term.js";

/** Fake streams: nothing touches the real terminal. */
function rig(cols = 80, rows = 24) {
  const stdin = Object.assign(new EventEmitter(), { setRawMode: vi.fn(), resume: vi.fn(), pause: vi.fn() });
  const written: string[] = [];
  const stdout = Object.assign(new EventEmitter(), { columns: cols, rows, write: (s: string) => written.push(s) > 0 });
  const proc = new EventEmitter();
  const exit = vi.fn();
  const io = { stdin: stdin as any, stdout: stdout as any, proc: proc as any, exit: exit as any };
  const key = (name: string, ctrl = false) => stdin.emit("keypress", undefined, { name, ctrl });
  return { stdin, stdout, proc, exit, io, written, key };
}
const idle = (over: Partial<Game> = {}): GameFactory => () => ({ tick: 0, key() {}, step() {}, draw: () => ["board"], ...over });
/** The terminal is back: raw mode on then off, the main screen, no listener left anywhere. */
const restored = (r: ReturnType<typeof rig>) => {
  expect(r.stdin.setRawMode.mock.calls).toEqual([[true], [false]]);
  expect(r.written.join("")).toContain("\x1b[?1049l");
  expect(r.written.at(-1)).toContain("\x1b[?1049l");
  expect(r.stdin.listenerCount("keypress")).toBe(0);
  expect(r.stdout.listenerCount("resize")).toBe(0);
  expect(r.proc.listenerCount("SIGTERM")).toBe(0);
  expect(r.proc.listenerCount("SIGHUP")).toBe(0);
  expect(r.proc.listenerCount("SIGINT")).toBe(0);
};

describe("play loop", () => {
  it("restores the terminal on q", async () => {
    const r = rig();
    const done = play(idle(), true, r.io);
    r.key("q");
    expect(await done).toBe("Bye.");
    restored(r);
  });

  it("restores the terminal on Ctrl-C", async () => {
    const r = rig();
    const done = play(idle(), true, r.io);
    r.key("c", true);
    expect(await done).toBe("Bye.");
    restored(r);
  });

  it("restores the terminal when a step throws", async () => {
    const r = rig();
    const done = play(idle({ tick: 1, step() { throw new Error("boom"); } }), true, r.io);
    await expect(done).rejects.toThrow("boom");
    restored(r);
  });

  it("restores the terminal when the first draw throws", async () => {
    const r = rig();
    await expect(play(idle({ draw() { throw new Error("first frame"); } }), true, r.io)).rejects.toThrow("first frame");
    restored(r);
  });

  it("restores the terminal and exits 128 + the signal number on SIGTERM, SIGHUP and SIGINT", async () => {
    for (const [sig, code] of [["SIGTERM", 143], ["SIGHUP", 129], ["SIGINT", 130]] as const) {
      const r = rig();
      const done = play(idle(), true, r.io);
      r.proc.emit(sig);
      expect(r.exit, sig).toHaveBeenCalledWith(code);
      await done;
      restored(r);
    }
  });

  it("ends with a message when the terminal is resized", async () => {
    const r = rig();
    const done = play(idle(), true, r.io);
    r.stdout.emit("resize");
    expect(await done).toBe("The terminal changed size.");
    restored(r);
  });
});

describe("play on a small terminal", () => {
  // The last row stays free, so the real need is one row more than the game asks for.
  const games: [string, GameFactory][] = [["2048", make2048], ["invaders", invaders], ["minesweeper", minesweeper], ["pong", pong], ["snake", snake], ["tetris", tetris]];
  for (const [name, make] of games) {
    it(`${name} reports the size the terminal needs, and that size works`, async () => {
      const err: Error = await play(make, true, rig(10, 5).io).then(() => new Error("no throw"), (e) => e);
      expect(err.message).toContain(`too small for ${name}`);
      expect(err.message).toContain("10x5"); // the real terminal size, not the game's
      const need = err.message.match(/needs (\d+)x(\d+)/)!;
      const [cols, rows] = [Number(need[1]), Number(need[2])];
      const ok = rig(cols, rows);
      const done = play(make, true, ok.io);
      ok.key("q");
      await done;
      restored(ok);
      const tight = rig(cols, rows - 1);
      await expect(play(make, true, tight.io)).rejects.toThrow(/too small/);
    });
  }
});
