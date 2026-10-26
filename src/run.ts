import { Command, CommanderError } from "commander";
import { register as registerAi } from "./commands/ai.js";
import { register as registerApi } from "./commands/api.js";
import { register as registerAuth } from "./commands/auth.js";
import { register as registerCompletion } from "./commands/completion.js";
import { register as registerDashboard } from "./commands/dashboard.js";
import { register as registerGames } from "./commands/games.js";
import { register as registerReference } from "./commands/reference.js";
import { register as registerSearch } from "./commands/search.js";
import { register as registerUpdate } from "./commands/update.js";
import { register as registerUsage } from "./commands/usage.js";
import { credentialDisplay } from "./core/credentials.js";
import { CliError, EXIT, usage } from "./core/errors.js";
import { Runtime } from "./core/runtime.js";
import { autoUpdate } from "./core/update.js";
import type { Deps, Globals, Register } from "./core/types.js";
import { register as registerManagement } from "./surfaces/management/index.js";

export type { Register };

/**
 * Commander has no second long alias, so `--app-id` becomes `--app` before parsing.
 * `help --all` becomes `reference`: commander runs `help` before it reads options and would drop `--all`.
 */
const rewrite = (argv: string[]) => {
  const rest = argv[0] === "help" && argv[1] === "--all" ? ["reference", ...argv.slice(2)] : argv;
  return rest.map((a) => (a === "--app-id" ? "--app" : a.startsWith("--app-id=") ? `--app=${a.slice(9)}` : a));
};

const intro = (name: string) => `
Conventions:
  JSON goes to stdout. An error goes to stderr as one JSON object.
  A command is <resource> <action>. The resource is the lowercase tag of the spec (ab-tests, custom-events).
  The action is list, get, create, update, delete, or a verb such as start or make-default. apps is an alias of applications.
  --app <id> is the application id (--app-id is an alias). A command that exists with and without an application
  (placements list) calls the global endpoint when you omit --app.
  The other path parameters are flags: --id, --plan-id, --store.
  The query parameters are flags in kebab case. An array takes --fields id,name or a repeated flag.
  --data '<json>', --data @file.json or --data - (stdin) is the body.
  --dry-run prints { method, url, headers, body } and sends nothing. It needs no key.
  --env is mandatory on every write (POST, PUT, PATCH, DELETE), also with --dry-run. Exit 10 without it.
  A risky change returns CONFIRMATION_REQUIRED with its impact and a confirm_token (valid 5 minutes, tied to the exact request).
  A JSON output over 200 KB is not printed. It goes to a file in the cache directory and the CLI prints {"file": "...", "bytes": 123}.
  --out <file> chooses the file. --out - forces stdout.

Examples:
  ${name} placements list --app app_xxx --fields id,vendor_id | jq -r '.placements[].id'
  ${name} screens list --app app_xxx
  ${name} placements get --app app_xxx --id <placement>
  ${name} placements update --app app_xxx --id plc_xxx --data @placement.json --env prod --dry-run
  ${name} reference --format man`;

/**
 * The built-in commands first, then the modules of the extension. A hand-written command registers before the
 * Management commands and takes a generated name.
 */
export function build(rt: Runtime): Command {
  const program = new Command(rt.d.name)
    .description("Purchasely command line tool")
    .version(rt.d.version)
    .exitOverride()
    .configureOutput({ writeOut: rt.d.stdout, writeErr: rt.d.stderr, outputError: () => undefined })
    .addHelpText("after", intro(rt.d.name));
  for (const r of [registerAuth, registerUsage, registerSearch, registerReference, registerAi, registerUpdate, registerGames, registerCompletion, registerApi, registerDashboard, registerManagement]) r(program, rt);
  const before = new Set(program.commands);
  for (const r of rt.d.ext?.modules ?? []) r(program, rt);
  program.commands.filter((c) => !before.has(c)).forEach((c) => rt.extTop.add(c));
  // A group with no subcommand: commander prints its help on stderr and exits 2. Print it on stdout and exit 0.
  const stdoutHelp = (c: Command) => {
    const help = c.help.bind(c);
    c.help = ((o) => help({ ...o, error: false })) as Command["help"];
    c.commands.forEach(stdoutHelp);
  };
  stdoutHelp(program);

  program.hook("preAction", async (_cmd, action) => {
    // A group parses the options that come before its subcommand for itself, and the leaf never sees them: refuse them.
    const parent = action.parent;
    if (parent && parent !== program && parent.options.some((o) => parent.getOptionValueSource(o.attributeName()) === "cli")) {
      throw usage(`Put the options after the subcommand: ${rt.d.name} ${parent.name()} ${action.name()} --app ...`);
    }
    let top = action;
    while (top.parent?.parent) top = top.parent;
    // A game and `completion` need no key, no update check and no log line on the screen.
    if (["games", "play", "completion"].includes(top.name())) return;
    // `reference` is offline: no credential, no update check, no stderr line, no request, not even a valid --env.
    if (top.name() === "reference") return;
    // `grep` reads local files only. It still validates --env itself.
    if (top.name() === "grep") return;
    const g = action.opts() as Globals;
    const env = rt.env(g);
    if (g.output && g.output !== "json" && g.output !== "table") throw new CliError(EXIT.USAGE, "USAGE", ["--output must be json or table."]);
    await autoUpdate(rt, action);
    // A built-in command uses the Management key. A command added by an extension module uses the surface of the extension.
    const surface = rt.d.ext && rt.extTop.has(top) ? rt.d.ext.surface : "management";
    const cred = await rt.credential(env, surface);
    rt.d.stderr(`${rt.d.name} env=${env} surface=${surface} identity=${rt.d.ext?.identity ? rt.cfg.identity_email ?? "-" : "-"} key=${credentialDisplay(surface, cred?.key)}\n`);
  });
  return program;
}

export async function run(argv: string[], d: Deps): Promise<number> {
  const fail = (e: CliError) => {
    d.stderr(JSON.stringify(e.toJSON()) + "\n");
    return e.exit;
  };
  try {
    const rt = new Runtime(d);
    await build(rt).parseAsync(rewrite(argv), { from: "user" });
    return EXIT.OK;
  } catch (e) {
    if (e instanceof CliError) return fail(e);
    if (e instanceof CommanderError) {
      if (e.exitCode === 0) return EXIT.OK;
      return fail(new CliError(EXIT.USAGE, "USAGE", [e.message.replace(/^error: /, "")]));
    }
    return fail(new CliError(EXIT.GENERIC, "INTERNAL_ERROR", [(e as Error).message ?? "unknown error"]));
  }
}
