import type { Command } from "commander";
import { usage } from "../core/errors.js";
import type { Runtime } from "../core/runtime.js";
import { menu, play, type GameFactory } from "../games/term.js";

/** One line per game, in alphabetical order. */
const GAMES: Record<string, () => Promise<{ default: GameFactory }>> = {
  "2048": () => import("../games/2048.js"),
  invaders: () => import("../games/invaders.js"),
  minesweeper: () => import("../games/minesweeper.js"),
  pong: () => import("../games/pong.js"),
  snake: () => import("../games/snake.js"),
  tetris: () => import("../games/tetris.js"),
};

export function register(program: Command, rt: Runtime): void {
  const names = Object.keys(GAMES);
  const action = (label: string) => async (name: string | undefined) => {
    if (name !== undefined && !GAMES[name]) throw usage(`Unknown game "${name}". Games: ${names.join(", ")}.`);
    if (!process.stdin.isTTY || !process.stdout.isTTY) throw usage(`${label} needs an interactive terminal.`);
    name ??= await play(menu(names), false);
    // Quit in the menu: `play` returns its goodbye, not a game name.
    const load = GAMES[name];
    rt.d.stdout((load ? await play((await load()).default) : name) + "\n");
  };
  const arg = "The name of the game. Without a name, a menu shows the games";
  program
    .command("games")
    .description("Play a game in the terminal")
    .argument("[game]", arg)
    .addHelpText("after", `\nGames: ${names.join(", ")}.\nKeys: arrows or WASD move, space acts, q, Esc or Ctrl-C quits.\nIt needs an interactive terminal. It uses no key and sends no request.\nExample:\n  ${rt.d.name} games snake\n`)
    .action(action("games"));
  // Hidden: the old name keeps working. A second command, not an alias, so the help shows one line.
  program.command("play", { hidden: true }).description("Play a game in the terminal").argument("[game]", arg).action(action("play"));
}
