import { Command } from "commander";
import { CliError, EXIT, usage } from "../core/errors.js";
import type { Surface } from "../core/credentials.js";
import { send, userAgent, type Ctx } from "../core/http.js";
import { emit } from "../core/output.js";
import { addGlobals, group, type Runtime } from "../core/runtime.js";
import type { Globals } from "../core/types.js";
import { choose, clean } from "../games/term.js";
import { findManagementCommand, requiredFlags, runOp, safeJson, toError } from "../surfaces/management/index.js";
import { buildRequest, type Built } from "../surfaces/management/request.js";

type Rec = Record<string, unknown>;
type Point = { period: string; value: number };
type Panel = { name: string; value: number; series: Point[] };
type Mrr = { mrr_evolution_live?: { data?: { mrr?: { values?: { by_day?: unknown } } } } };

const BLOCKS = "▁▂▃▄▅▆▇█";
const DAY_MS = 86_400_000;

/** One block character per point, scaled from the lowest to the highest value. */
export function sparkline(values: number[]): string {
  const min = Math.min(...values);
  const span = Math.max(...values) - min;
  return values.map((v) => BLOCKS[span ? Math.round(((v - min) / span) * (BLOCKS.length - 1)) : 0]).join("");
}

const num = (v: unknown) => (typeof v === "number" && Number.isFinite(v) ? v : 0);
const points = (rows: unknown, key: string): Point[] =>
  (Array.isArray(rows) ? (rows as Rec[]) : []).map((r) => ({ period: String(r.period), value: num(r[key]) }));
const sum = (s: Point[]) => s.reduce((a, p) => a + p.value, 0);
const mrrPoints = (body: Rec) => points((body as Mrr).mrr_evolution_live?.data?.mrr?.values?.by_day, "value");

/** DAY up to 60 days, WEEK up to about 6 months, MONTH beyond: the periodicity the dashboard tools recommend. */
const periodicityFor = (days: number) => (days <= 60 ? "DAY" : days <= 183 ? "WEEK" : "MONTH");

function board(app: string, from: string, to: string, periodicity: string, panels: Panel[]): string {
  const fmt = (v: number) => v.toLocaleString("en-US", { maximumFractionDigits: 2 });
  const w = Math.max(...panels.map((p) => p.name.length));
  const vw = Math.max(...panels.map((p) => fmt(p.value).length));
  const rows = panels.map((p) => `${p.name.padEnd(w)}  ${fmt(p.value).padStart(vw)}  ${p.series.length ? sparkline(p.series.map((x) => x.value)) : "(no data)"}`);
  return [`${app}  ${from} to ${to}  (${periodicity})`, "", ...rows].join("\n");
}

const HELP = `
Shows the MRR, the active subscriptions, the paywall unique viewers and the conversions of one app.
The default output is JSON. --output table draws one line per metric with a sparkline.
MRR is today's value, in the dashboard currency of the app. Active subscriptions is the current base.
Viewers and conversions are the total of the range.
It sends 3 requests (4 when --days is above 60).

Example:
  PREFIX summary --app app_xxx --output table
  PREFIX summary --app app_xxx --days 90 --output table
`;

/** One dashboard read, ready to send: `preview` is what --dry-run prints. */
export interface Read { preview: unknown; read(): Promise<Rec> }

/** Where the dashboard data comes from: the Management API for `dashboard`, another surface for an extension. */
export interface Source {
  surface: Surface;
  prepare(ctx: Ctx, action: string, app: string, args: { date_range: string[]; periodicity: string }): Promise<Read>;
}

/** The `summary` leaf of a dashboard group: the options, the help and the run. Returns the run, for the menu. */
export function summaryLeaf(dashboard: Command, rt: Runtime, source: Source, prefix: string): (o: Globals & { app?: string; days: string }) => Promise<void> {
  addGlobals(dashboard.command("summary").description("Analytics dashboard of an app: MRR, subscriptions, viewers, conversions with trend charts"))
    .option("--app <app_id>", "application public id (required)")
    .option("--days <n>", "number of days up to today, from 2 to 730", "30")
    .addHelpText("after", HELP.replace(/PREFIX/g, prefix))
    .action(summary);

  /** `dashboard summary`: the options come from the command, or from the menu of `dashboard`. */
  async function summary(o: Globals & { app?: string; days: string }): Promise<void> {
    if (!o.app) throw usage("dashboard summary needs --app <app_id>.");
    const days = /^\d+$/.test(o.days) ? Number(o.days) : NaN;
    if (!(days >= 2 && days <= 730)) throw usage("--days must be a whole number from 2 to 730.");
    const ctx = await rt.ctx(o, source.surface);
    const now = rt.d.now();
    const to = new Date(now).toISOString().slice(0, 10);
    const from = new Date(now - (days - 1) * DAY_MS).toISOString().slice(0, 10);
    const periodicity = periodicityFor(days);

    const prepare = (action: string, dateRange = [from, to], p = periodicity) => source.prepare(ctx, action, o.app!, { date_range: dateRange, periodicity: p });
    const reqs = [await prepare("mrr-evolution-live"), await prepare("subscription-evolution"), await prepare("conversion")];
    // With WEEK or MONTH the last MRR bucket is the start of the period, so read today's MRR apart.
    if (periodicity !== "DAY") reqs.push(await prepare("mrr-evolution-live", [to, to], "DAY"));

    if (o.dryRun) return emit(rt.d, o, reqs.map((r) => r.preview), { env: ctx.env, key: ctx.key, name: "dashboard-summary" });
    const bodies: Rec[] = [];
    for (const r of reqs) {
      try {
        bodies.push(await r.read());
      } catch (e) {
        if (e instanceof CliError && e.exit === EXIT.NOT_FOUND) e.messages.unshift(`application "${o.app}" not found`);
        throw e;
      }
    }
    const [mrr, subs, convBody, today] = bodies;
    const conv = (convBody.conversion ?? {}) as Rec;

    const mrrSeries = mrrPoints(mrr);
    const subsSeries = points(subs.subscription_evolution, "count");
    const viewers = points(conv.primary_metric_by_period, "unique_viewers_count");
    const conversions = points(conv.secondary_metric_by_period, "total_conversions_count");
    const kpi = (Array.isArray(conv.kpi) ? conv.kpi[0] : undefined) as Rec | undefined;
    // MRR and the active base are stocks: read the last value, never a sum. Unique viewers do not add up across periods.
    const panels: Panel[] = [
      { name: "MRR", value: (today ? mrrPoints(today) : mrrSeries).at(-1)?.value ?? 0, series: mrrSeries },
      { name: "Active subscriptions", value: subsSeries.at(-1)?.value ?? 0, series: subsSeries },
      { name: "Paywall unique viewers", value: num(kpi?.unique_viewers_count), series: viewers },
      { name: "Conversions", value: sum(conversions), series: conversions },
    ];

    if (o.output === "table") return rt.d.stdout(board(o.app, from, to, periodicity, panels) + "\n");
    emit(rt.d, o, { app: o.app, from, to, periodicity, panels }, { env: ctx.env, key: ctx.key, name: "dashboard-summary" });
  }
  return summary;
}

/** The dashboards of the Management API: one request per read. */
const managementSource: Source = {
  surface: "management",
  async prepare(ctx, action, app, args) {
    const c = findManagementCommand("dashboard", action);
    if (!c) throw usage(`dashboard summary needs dashboard ${action}, which this version of the API spec does not have. Run ${ctx.d.name} update.`);
    const b = buildRequest(c, { application_id: app, ...args }, ctx.env, undefined);
    const headers = { Authorization: "Bearer <redacted>", Accept: "application/json", "User-Agent": userAgent(ctx) };
    return {
      preview: { method: "GET", url: b.url, headers },
      async read() {
        const res = await send(ctx, { method: "GET", url: b.url, pathTemplate: b.op.path });
        if (res.status < 200 || res.status >= 300) throw toError(res);
        const body = res.text.trim() ? safeJson(res.text) : {};
        return body !== null && typeof body === "object" ? (body as Rec) : {};
      },
    };
  },
};

/** `dashboard summary`: MRR, active subscriptions, paywall viewers and conversions of one app, in 3 or 4 requests. */
export function register(program: Command, rt: Runtime): void {
  rt.taken.add("dashboard summary");
  rt.searchIndex.push({
    command: `${rt.d.name} dashboard summary --app app_xxx --output table`, resource: "dashboard", action: "summary",
    summary: "Analytics dashboard of an app: key metrics with trend charts",
    description: "Dashboard of one app in the terminal: MRR, active subscriptions, paywall unique viewers and conversions over the last days, with a sparkline chart per metric (--output table). Overview of revenue, subscriptions, conversion and stats",
    method: "GET", path: "-", scope: "analytics.dashboards:view",
  });

  const dashboard = group(program, "dashboard", "Analytics dashboards. Start with: dashboard summary --app <app_id> --output table");
  const prefix = `${rt.d.name} dashboard`;
  const summary = summaryLeaf(dashboard, rt, managementSource, prefix);
  // Read when the command runs: the Management actions register after this module.
  const items = () =>
    dashboard.createHelp().visibleCommands(dashboard)
      .filter((leaf) => leaf.name() !== "help")
      .map((leaf): MenuItem => {
        const spec = findManagementCommand("dashboard", leaf.name());
        const required_flags = spec ? requiredFlags(spec) : ["--app"];
        return { name: leaf.name(), command: `${prefix} ${leaf.name()}`, description: leaf.description(), required_flags, dated: !!spec?.options.some((x) => x.name === "date_range"), daily: required_flags.includes("--periodicity") };
      });
  const apps = async (o: Globals): Promise<string | undefined> => {
    const ctx = await rt.ctx(o, "management");
    const b = buildRequest(findManagementCommand("applications", "list")!, { fields: ["id", "name"], per_page: 100 }, ctx.env, undefined);
    const res = await send(ctx, { method: "GET", url: b.url, pathTemplate: b.op.path });
    if (res.status < 200 || res.status >= 300) throw toError(res);
    const list = ((safeJson(res.text) as { mobile_applications?: { id: string; name: string }[] } | null)?.mobile_applications ?? []);
    return pickApp(rt, list.map((a) => ({ id: a.id, name: a.name })));
  };
  menuGroup(program, dashboard, prefix).action(menuAction(rt, {
    prefix, items, apps, summary,
    run: (o, item, range) => runOp(rt, findManagementCommand("dashboard", item.name)!, { ...o, ...(item.dated ? { dateRange: [range] } : {}), ...(item.daily ? { periodicity: "DAY" } : {}) }),
  }));
}

/** One line of the dashboard list: what the menu and the JSON list show. */
export interface MenuItem { name: string; command: string; description: string; required_flags: string[]; dated: boolean; daily: boolean }

/** What the menu of a dashboard group needs from its surface. */
export interface MenuSource {
  prefix: string;
  /** Read when the command runs. */
  items(o: Globals): MenuItem[] | Promise<MenuItem[]>;
  /** A menu of the apps of the key. The id of the chosen one, or undefined when the person quits. */
  apps(o: Globals): Promise<string | undefined>;
  summary(o: Globals & { app?: string; days: string }): Promise<void>;
  /** Run one dashboard other than `summary`; `o.app` is set, `range` is the last 30 days. */
  run(o: Globals & { app: string }, item: MenuItem, range: string): Promise<void>;
}

/** The menu of the apps. Each app is `name  id`, and `note` comes before the id when it is set. */
export async function pickApp(rt: Runtime, list: { id: string; name: string; note?: string }[]): Promise<string | undefined> {
  if (!list.length) throw new CliError(EXIT.NOT_FOUND, "NOT_FOUND", ["No application found for this key."]);
  const width = Math.max(...list.map((a) => clean(a.name).length));
  const note = Math.max(...list.map((a) => clean(a.note ?? "").length));
  const at = await choose(list.map((a) => `${clean(a.name).padEnd(width)}  ${note ? clean(a.note ?? "").padEnd(note) + "  " : ""}${a.id}`), { title: "Applications", verb: "choose" }, rt.d.io ?? {}, rt.d.stderr);
  return at === undefined ? undefined : list[at].id;
}

const ymd = (t: number) => new Date(t).toISOString().slice(0, 10);
const FILLED = ["--app", "--date-range", "--periodicity"];

/** The options and the help of a dashboard group that has a menu. The caller sets the action: `menuAction`. */
export function menuGroup(program: Command, dashboard: Command, prefix: string): Command {
  program.enablePositionalOptions();
  return addGlobals(dashboard)
    .helpCommand(true) // a group with an action loses the implicit `help summary`
    .option("--app <app_id>", "application public id. Without it, a menu shows the apps of the key")
    // The options of the group stop at the subcommand: `dashboard summary --app x` reaches summary.
    // Commander needs it on the root too: the root hands the raw arguments to the group.
    .enablePositionalOptions()
    .addHelpText("after", `\nWithout a subcommand, an interactive terminal shows a menu of the dashboards, then runs the chosen one for the last 30 days.\nThe menu leaves out the dashboards that need another flag (funnel, flow-paths): run them by name.\nWith --output json, or without a terminal, it prints the list of all the dashboards as JSON, with their required flags, and sends no request.\n\nExample:\n  ${prefix} --output json | jq -r '.[].command'\n`);
}

/**
 * `dashboard` with no subcommand. In a terminal: a menu of the apps of the key (without --app), then a menu of the
 * dashboards, then the chosen one for the last 30 days. Not in a terminal, or with --output json: the list as JSON.
 */
export function menuAction(rt: Runtime, src: MenuSource): (o: Globals & { app?: string }) => Promise<void> {
  const pick = (title: string, verb: string, lines: string[]) => choose(lines, { title, verb }, rt.d.io ?? {}, rt.d.stderr);

  return async (o) => {
    const all = await src.items(o);
    const io = { stdin: process.stdin, stdout: process.stdout, ...rt.d.io };
    if (o.output === "json" || !io.stdin.isTTY || !io.stdout.isTTY) {
      return rt.d.stdout(JSON.stringify(all.map(({ command, description, required_flags }) => ({ command, description, required_flags })), null, 2) + "\n");
    }
    if (o.dryRun && !o.app) throw usage("--dry-run needs --app: the app menu sends a request.");
    const app = o.app ?? (await src.apps(o));
    if (!app) return;
    // The menu fills the app, the date range and the periodicity. A dashboard with another required flag needs the command line.
    const list = all.filter((d) => d.required_flags.every((f) => FILLED.includes(f)));
    const width = Math.max(...list.map((d) => `dashboard ${d.name}`.length));
    const at = await pick("Dashboards", "run", list.map((d) => `${`dashboard ${d.name}`.padEnd(width)}  ${d.description}`));
    if (at === undefined) return;

    const item = list[at];
    const now = rt.d.now();
    const range = `${ymd(now - 29 * DAY_MS)},${ymd(now)}`;
    const tail = [...(o.env ? ["--env", o.env] : []), ...(o.reason ? ["--reason", JSON.stringify(o.reason)] : [])];
    if (item.name === "summary") {
      rt.d.stderr([item.command, "--app", app, "--days", "30", "--output", "table", ...tail].join(" ") + "\n");
      return src.summary({ ...o, app, days: "30", output: "table" });
    }
    rt.d.stderr([item.command, "--app", app, ...(item.dated ? ["--date-range", range] : []), ...(item.daily ? ["--periodicity", "DAY"] : []), ...(o.output ? ["--output", o.output] : []), ...tail].join(" ") + "\n");
    return src.run({ ...o, app }, item, range);
  };
}
