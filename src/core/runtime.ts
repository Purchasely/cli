import { Command } from "commander";
import { readConfig, type Config } from "./config.js";
import { resolveCredential, surfaceDef, type Credential, type Surface } from "./credentials.js";
import { CliError, EXIT, usage } from "./errors.js";
import type { Ctx } from "./http.js";
import type { Deps, Env, Globals } from "./types.js";

export interface SearchEntry {
  command: string;
  resource: string;
  action: string;
  summary: string;
  description: string;
  method: string;
  path: string;
  scope: string;
}

/** Get or create a command group. A hand-written command and the generated ones share one group. */
export function group(program: Command, name: string, description = `Management API: ${name}`): Command {
  const existing = program.commands.find((c) => c.name() === name);
  if (existing) return existing;
  const created = program.command(name).description(description);
  if (name === "applications") created.alias("apps");
  return created;
}

/** Per-run state shared by all commands. */
export class Runtime {
  cfg: Config;
  /** `resource action` names already taken by a hand-written command. */
  taken = new Set<string>();
  searchIndex: SearchEntry[] = [];
  /** The top-level commands that an extension module added. Filled by `build()`. */
  extTop = new Set<Command>();
  /** Entries from extension modules, computed at search time. Each one reads a cache and sends nothing. */
  searchExtra: ((env: Env) => Promise<SearchEntry[]>)[] = [];
  private creds = new Map<string, Promise<Credential | null>>();

  constructor(public d: Deps) {
    this.cfg = readConfig(d.configDir);
  }

  env(g: Globals): Env {
    const v = g.env ?? this.d.env.PURCHASELY_ENV ?? "prod";
    if (v !== "prod" && v !== "staging") throw usage(`--env must be prod or staging, got "${v}".`);
    return v;
  }

  credential(env: Env, surface: Surface): Promise<Credential | null> {
    const k = `${env}:${surface}`;
    if (!this.creds.has(k)) this.creds.set(k, resolveCredential(this.d, env, surface));
    return this.creds.get(k)!;
  }

  /** Requests per second. `--rate` and the config may only lower the default, unless --i-asked-backend. */
  rate(surface: Surface, g: Globals): number {
    const { default: def, cap } = surfaceDef(surface).rate;
    let rate: number = this.cfg.rate?.[surface] ?? def;
    if (g.rate !== undefined) rate = Number(g.rate);
    if (!(rate > 0)) throw usage("--rate must be a number above 0.");
    if (rate > cap && !g.iAskedBackend) {
      throw usage(`The rate ${rate} req/s is above the cap of ${cap} req/s for ${surface}. Lower it, or ask the backend team and pass --i-asked-backend.`);
    }
    return rate;
  }

  async ctx(g: Globals, surface: Surface = "management"): Promise<Ctx> {
    const env = this.env(g);
    const cred = await this.credential(env, surface);
    return {
      d: this.d,
      env,
      surface,
      key: cred?.key ?? null,
      identity: this.cfg.identity_email,
      reason: g.reason,
      rate: this.rate(surface, g),
    };
  }
}

export function addGlobals(cmd: Command): Command {
  return cmd
    .option("--output <format>", "json (default) or table")
    .option("--env <env>", "prod or staging. Mandatory for writes")
    .option("--dry-run", "print the request without sending it")
    .option("--yes", "accept the confirmation prompt of a risky change")
    .option("--reason <text>", "why you run this command (sent in the User-Agent)")
    .option("--rate <n>", "requests per second, lower than the default")
    .option("--i-asked-backend", "allow a rate above the hard cap")
    .option("--out <file>", "write the JSON output to a file, or - for stdout");
}

export const refused = (msg: string) => new CliError(EXIT.REFUSED, "REFUSED", [msg]);
