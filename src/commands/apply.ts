import type { Command } from "commander";
import { CliError, EXIT } from "../core/errors.js";
import { emit } from "../core/output.js";
import { addGlobals, refused, type Runtime } from "../core/runtime.js";
import { BASE_URLS, type Globals } from "../core/types.js";
import { perform, readPlan, writePlan, type PlanEntry } from "../surfaces/management/index.js";
import { withToken } from "../surfaces/management/request.js";

const METHODS = ["POST", "PUT", "PATCH", "DELETE"];

const HELP = `
A plan file holds writes that a command recorded with --plan <file>. An agent can record them
with PURCHASELY_READ_ONLY=1 or with no key: nothing is sent. A person reads the plan, then applies it.

The CLI lists the writes and asks once (or --yes). A risky change still asks on its own in a terminal;
without a terminal, apply stops there (exit 8). Each applied write is marked in the file, so a second
run sends only the rest. apply stops at the first error: exit 9 when some writes were applied.

Examples:
  purchasely placements update --app app_xxx --id plc_xxx --data @placement.json --env prod --plan plan.json
  purchasely apply plan.json --dry-run
  purchasely apply plan.json`;

/** An entry is checked again before the send: the key goes only to the Management API of its env. */
function check(r: PlanEntry, n: number): void {
  const bad = (why: string) => refused(`Request #${n} of the plan ${why}. The CLI sent nothing.`);
  if (r.env !== "prod" && r.env !== "staging") throw bad(`has env "${String(r.env)}", not prod or staging`);
  if (!METHODS.includes(r.method)) throw bad(`has method "${String(r.method)}", not a write`);
  if (typeof r.url !== "string" || !r.url.startsWith(`${BASE_URLS[r.env]}/management/v1/`)) throw bad(`goes to ${String(r.url)}, not to the Management API of ${r.env}`);
  if (r.body !== undefined) {
    try {
      JSON.parse(r.body);
    } catch {
      throw bad("has a body that is not a JSON string");
    }
  }
  if (r.confirm !== undefined && r.confirm !== "query" && r.confirm !== "body") throw bad("has an unknown confirm placement");
}

export function register(program: Command, rt: Runtime): void {
  addGlobals(program.command("apply").description("Send the writes of a plan file, after a person reads them").argument("<file>", "plan file from --plan"))
    .addHelpText("after", HELP.replaceAll("purchasely ", `${rt.d.name} `))
    .action(async (file: string, o: Globals) => {
      const plan = readPlan(file);
      const pending = plan.requests.map((r, i) => ({ r, n: i + 1 })).filter((x) => !x.r.applied_at);
      for (const { r, n } of pending) check(r, n);
      const out = { env: "-", key: null, name: "apply" };
      const list = pending.map(({ r, n }) => ({ number: n, command: r.command, env: r.env, method: r.method, url: r.url, body: r.body === undefined ? null : JSON.parse(r.body) }));
      if (o.dryRun || pending.length === 0) return emit(rt.d, o, { pending: list }, out);

      for (const p of list) rt.d.stderr(`#${p.number} ${p.env} ${p.method} ${p.url}\n`);
      const approved = o.yes || (rt.d.isTTY && /^y(es)?$/i.test((await rt.d.prompt(`Apply ${list.length} writes? [y/N] `)).trim()));
      if (!approved) throw new CliError(EXIT.CONFIRMATION, "CONFIRMATION_REQUIRED", [`The plan has ${list.length} writes to apply. Read them (apply --dry-run), then run again with --yes, or answer y in a terminal.`]);

      const applied: unknown[] = [];
      for (const { r, n } of pending) {
        // --yes approves the plan, not each risky change: those still ask on their own.
        const g: Globals = { ...o, env: r.env, yes: false, dryRun: false, plan: undefined };
        try {
          const { value } = await perform(rt, g, (_env, token) => ({ ...r, ...withToken(r.confirm, r.url, r.body, token) }), { name: r.command, refuseRedirect: true });
          r.applied_at = new Date(rt.d.now()).toISOString();
          writePlan(file, plan);
          applied.push({ number: n, command: r.command, response: value });
        } catch (e) {
          if (!(e instanceof CliError)) throw e;
          emit(rt.d, o, { applied, failed: { number: n, command: r.command, error: e.toJSON().error } }, out);
          throw new CliError(applied.length ? EXIT.PARTIAL : e.exit, e.code, [`Request #${n} of the plan failed. ${applied.length} applied, ${pending.length - applied.length} left.`, ...e.messages], e.extra, e.status);
        }
      }
      emit(rt.d, o, { applied }, out);
    });
}
