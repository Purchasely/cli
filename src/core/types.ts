import type { Command } from "commander";
import type { Io } from "../games/term.js";
import type { Runtime } from "./runtime.js";

export interface Keychain {
  get(service: string, account: string): Promise<string | null>;
  set(service: string, account: string, secret: string): Promise<void>;
  delete(service: string, account: string): Promise<void>;
}

export interface Deps {
  fetch: typeof fetch;
  env: Record<string, string | undefined>;
  stdout: (s: string) => void;
  stderr: (s: string) => void;
  isTTY: boolean;
  /** Ask a question. `hidden` must not echo the answer. */
  prompt: (question: string, opts?: { hidden?: boolean }) => Promise<string>;
  readStdin: () => Promise<string>;
  configDir: string;
  stateDir: string;
  cacheDir: string;
  now: () => number;
  sleep: (ms: number) => Promise<void>;
  keychain: Keychain | null;
  /**
   * Run a program without a shell. Rejects when the binary is missing.
   * `detach`: resolves at once with code 0, stdio ignored, the child is unref'd.
   */
  exec: (file: string, args: string[], opts: { cwd?: string; input?: string; timeoutMs: number; detach?: boolean }) => Promise<ExecResult>;
  version: string;
  pid: number;
  /** The program name: `purchasely`, or the name of a program built on it. */
  name: string;
  /** Public host of the release files (package.json config.host). */
  host: string;
  /** The terminal of an interactive menu. Absent: the streams of the process. The tests give fakes. */
  io?: Partial<Io>;
  /** What a program built on `purchasely` adds. Absent in `purchasely`. */
  ext?: Extension;
}

/** A group of commands. The built-in modules and the modules of an extension all have this shape. */
export type Register = (program: Command, rt: Runtime) => void;

/** The commands that a program built on `purchasely` adds, and how they differ from the built-in ones. */
export interface Extension {
  /** Registered after the built-in modules. */
  modules: Register[];
  /** The surface (credential and rate) of the top-level commands that the modules add. */
  surface: string;
  /** A top-level command whose surface is not `surface`, by its name. */
  surfaces?: Record<string, string>;
  /** Send `identity_email` of the config in the User-Agent, and ask for it at `auth login`. */
  identity: boolean;
  /** The heading of the added commands in `reference`, and the line under it. */
  section: { title: string; note: string };
  /** The latest stable version, instead of `<host>/latest.json`. `null`: no stable release. */
  latest?: (d: Deps) => Promise<string | null>;
  /** The update command of the daily notice of an npm install, instead of the install script. */
  npmUpdate?: string;
}

export interface ExecResult {
  code: number;
  stdout: string;
  stderr: string;
  timedOut: boolean;
}

export type Env = "prod" | "staging";
export const BASE_URLS: Record<Env, string> = {
  prod: "https://api.purchasely.io",
  staging: "https://api-staging.purchasely.io",
};

/** Global options present on every leaf command. */
export interface Globals {
  output?: string;
  env?: string;
  dryRun?: boolean;
  yes?: boolean;
  reason?: string;
  rate?: string;
  iAskedBackend?: boolean;
  out?: string;
  /** Record the write in this plan file instead of sending it. */
  plan?: string;
}
