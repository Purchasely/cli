import { readFileSync } from "node:fs";
import { createInterface } from "node:readline";
import { homedir } from "node:os";
import { join } from "node:path";
import { defaultKeychain } from "./core/credentials.js";
import { realExec } from "./core/exec.js";
import { ROOT } from "./core/root.js";
import type { Deps, Extension } from "./core/types.js";
import { run } from "./run.js";

function prompt(question: string, opts?: { hidden?: boolean }): Promise<string> {
  const rl = createInterface({ input: process.stdin, output: process.stderr, terminal: true });
  const w = rl as unknown as { _writeToOutput: (s: string) => void };
  const write = w._writeToOutput.bind(rl);
  if (opts?.hidden) w._writeToOutput = (s) => { if (s.includes(question)) write(s); };
  return new Promise((resolve) => rl.question(question, (a) => { rl.close(); if (opts?.hidden) process.stderr.write("\n"); resolve(a); }));
}

async function readStdin(): Promise<string> {
  let s = "";
  for await (const chunk of process.stdin) s += chunk;
  return s;
}

/** Entry of the program. `purchasely` has no extension; a program built on it passes its own. */
export async function main(name: string, ext?: Extension): Promise<void> {
  // `purchasely reference | head` closes the pipe: stop writing, print no stack.
  process.stdout.on("error", (e: NodeJS.ErrnoException) => { if (e.code !== "EPIPE") throw e; });
  const env = process.env;
  const home = homedir();
  const pkg = JSON.parse(readFileSync(new URL("package.json", ROOT), "utf8")) as { version: string; config?: { host?: string } };

  const deps: Deps = {
    fetch: globalThis.fetch,
    env,
    stdout: (s) => void process.stdout.write(s),
    stderr: (s) => void process.stderr.write(s),
    isTTY: !!process.stdin.isTTY && !!process.stderr.isTTY,
    prompt,
    readStdin,
    configDir: join(env.XDG_CONFIG_HOME || join(home, ".config"), "purchasely"),
    stateDir: join(env.XDG_STATE_HOME || join(home, ".local", "state"), "purchasely"),
    cacheDir: join(env.XDG_CACHE_HOME || join(home, ".cache"), "purchasely"),
    now: Date.now,
    sleep: (ms) => new Promise((r) => setTimeout(r, ms)),
    keychain: defaultKeychain(),
    exec: realExec,
    version: pkg.version,
    pid: process.pid,
    name,
    host: pkg.config?.host ?? "",
    ext,
  };

  process.exitCode = await run(process.argv.slice(2), deps);
}
