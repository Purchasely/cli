import { Command as Cli, Option } from "commander";
import { CliError, EXIT, exitFor } from "../../core/errors.js";
import { send, userAgent, type HttpResponse } from "../../core/http.js";
import { emit } from "../../core/output.js";
import { addGlobals, group, Runtime, refused } from "../../core/runtime.js";
import type { Env, Globals } from "../../core/types.js";
import { helpText, plain, typeLabel, usageString } from "./help.js";
import { singular } from "./naming.js";
import { attr, buildRequest, parseValue, readData, type Values } from "./request.js";
import { deriveCommands, loadSpec, parseOps, type Command, type Op, type Param } from "./spec.js";

let derived: Command[] | undefined;
/** The generated commands. The spec is parsed once per process. */
export function managementCommands(): Command[] {
  return (derived ??= deriveCommands(parseOps(loadSpec())));
}

const paramOf = (c: Command, name: string): Param =>
  [c.appOp, c.globalOp].flatMap((o) => o?.params ?? []).find((p) => p.name === name)!;

/** The flags a command cannot run without: `--app` when it has no global form, and each required option. */
export const requiredFlags = (c: Command): string[] => [
  ...(c.globalOp ? [] : ["--app"]),
  ...c.options.filter((o) => paramOf(c, o.name).required).map((o) => `--${o.flag}`),
];

export const findManagementCommand = (resource: string, action: string): Command | undefined =>
  managementCommands().find((c) => c.resource === resource && c.action === action);

/** The `--app`, parameter, `--data` and confirm options of a generated command. */
export function addOperationOptions(leaf: Cli, c: Command): void {
  const hasApp = !!c.appOp;
  leaf.option("--app <app_id>", `application public id${hasApp && !c.globalOp ? " (required)" : c.globalOp && hasApp ? " (optional: omit for all apps)" : ""}`);
  for (const o of c.options) {
    const p = paramOf(c, o.name);
    const where = c.appOp && c.globalOp && !(c.appOp.params.some((x) => x.name === o.name) && c.globalOp.params.some((x) => x.name === o.name))
      ? (c.appOp.params.some((x) => x.name === o.name) ? "only with --app" : "only without --app") : "";
    const array = p.schema.type === "array";
    const desc = [p.description && plain(p.description.replace(/\s+/g, " ")), `[${typeLabel(p.schema)}${p.required ? ", required" : ""}${where ? ", " + where : ""}]`, array && "comma separated or repeated"].filter(Boolean).join(" ");
    const opt = new Option(p.schema.type === "boolean" ? `--${o.flag}` : `--${o.flag} <${array ? "a,b" : "value"}>`, desc);
    if (array) opt.argParser((v: string, prev?: string[]) => [...(prev ?? []), v]);
    leaf.addOption(opt);
  }
  if ([c.appOp, c.globalOp].some((o) => o?.body)) leaf.option("--data <json>", "request body: JSON, @file.json or - for stdin");
  if ([c.appOp, c.globalOp].some((o) => o?.confirm)) leaf.option("--confirm-token <token>", "token from a CONFIRMATION_REQUIRED response");
}

export function register(program: Cli, rt: Runtime): void {
  for (const c of managementCommands()) {
    // A shadowed command stays searchable: its entry names the hand-written command of the same name.
    const op = (c.appOp ?? c.globalOp)!;
    rt.searchIndex.push({
      command: usageString(c, rt.d.name), resource: c.resource, action: c.action, summary: plain(c.summary),
      description: op.description, method: op.method, path: op.path, scope: op.scope,
    });
    if (rt.taken.has(`${c.resource} ${c.action}`)) continue;
    leafOf(group(program, c.resource), c, rt);
    // The old resource of 0.4.0 scripts: it runs, and no help, reference, search or completion shows it.
    if (c.legacy) leafOf(program.commands.find((x) => x.name() === c.legacy) ?? program.command(c.legacy, { hidden: true }).description(`Management API: ${c.legacy}`), c, rt);
  }
}

function leafOf(parent: Cli, c: Command, rt: Runtime): void {
  const leaf = parent.command(c.action).description(plain(c.summary));
  addGlobals(leaf);
  addOperationOptions(leaf, c);
  leaf.addHelpText("after", helpText(c, rt.d.name));
  leaf.action(async (opts: Record<string, unknown>) => runOp(rt, c, opts));
}

interface Parsed { values: Values; g: Globals; data?: string; confirmToken?: string }

async function parseOpts(rt: Runtime, c: Command, opts: Record<string, unknown>): Promise<Parsed> {
  const values: Values = {};
  if (opts.app !== undefined) values.application_id = opts.app;
  for (const o of c.options) {
    const raw = opts[attr(o.flag)];
    if (raw !== undefined) values[o.name] = parseValue(paramOf(c, o.name), raw);
  }
  const data = opts.data === undefined ? undefined : await readData(String(opts.data), rt.d.readStdin);
  return { values, g: opts as Globals, data, confirmToken: opts.confirmToken as string | undefined };
}

export async function runOp(rt: Runtime, c: Command, opts: Record<string, unknown>): Promise<void> {
  const { values, g, data, confirmToken } = await parseOpts(rt, c, opts);
  const build = (env: Env, token?: string): Req => {
    const b = buildRequest(c, values, env, data, token);
    return { method: b.op.method, url: b.url, pathTemplate: b.op.path, body: b.body, confirm: b.op.confirm };
  };
  return execute(rt, g, build, { name: `${c.resource}-${c.action}`, confirmToken, columns: values.fields as string[] | undefined, notFound: notFound(c, values) });
}

/** `placement "plc_1" not found in app app_1`, from the path ids of the request. None when the app is the only one. */
function notFound(c: Command, values: Values): string | undefined {
  const id = (values.application_id ? c.appOp : c.globalOp)?.params.filter((p) => p.in === "path" && p.name !== "application_id").pop();
  if (!id) return undefined;
  return `${singular(c.resource)} "${values[id.name]}" not found${values.application_id ? ` in app ${values.application_id}` : ""}`;
}

export interface Req { method: string; url: string; pathTemplate: string; body?: string; confirm?: Op["confirm"] }

/**
 * One Management request, for the generated commands and `api`: the Management key, `--env` on every write
 * (also with --dry-run), the masked dry-run, then the send and the confirm flow.
 */
export async function execute(
  rt: Runtime,
  g: Globals,
  build: (env: Env, token?: string) => Req,
  meta: { name: string; confirmToken?: string; columns?: string[]; notFound?: string; refuseRedirect?: boolean },
): Promise<void> {
  const ctx = await rt.ctx(g, "management");
  const first = build(ctx.env, meta.confirmToken);
  if (first.method !== "GET" && !g.env) throw refused("A write needs --env prod or --env staging. Add it, also with --dry-run.");
  if (first.method !== "GET") checkAllowlist(rt, ctx.env, first.url);
  const out = { env: ctx.env, key: ctx.key, name: meta.name, columns: meta.columns };

  if (g.dryRun) {
    const headers: Record<string, string> = { Authorization: "Bearer <redacted>", Accept: "application/json", "User-Agent": userAgent(ctx) };
    if (first.body !== undefined) headers["Content-Type"] = "application/json";
    return emit(rt.d, g, { method: first.method, url: first.url, headers, body: first.body === undefined ? null : JSON.parse(first.body) }, out);
  }

  let sentToken = meta.confirmToken;
  let built = first;
  for (;;) {
    const res = await send(ctx, { method: built.method, url: built.url, pathTemplate: built.pathTemplate, body: built.body, refuseRedirect: meta.refuseRedirect });
    if (res.status >= 200 && res.status < 300) return emit(rt.d, g, res.text.trim() ? safeJson(res.text) : {}, out);
    const err = toError(res);
    if (meta.notFound && err.exit === EXIT.NOT_FOUND) err.messages.unshift(meta.notFound);
    const token = err.extra.confirm_token as string | undefined;
    // No retry when the request has no known confirm placement (an `api` path outside the spec).
    if (err.code !== "CONFIRMATION_REQUIRED" || sentToken || !token || !built.confirm) throw err;
    rt.d.stderr(`Impact: ${err.messages.join(" ")}\n`);
    const approved = g.yes || (rt.d.isTTY && /^y(es)?$/i.test((await rt.d.prompt("Apply this change? [y/N] ")).trim()));
    if (!approved) throw err;
    sentToken = token;
    built = build(ctx.env, token);
  }
}

/** `write_allowlist` of the config: when the env has a list, a write must target one of its apps. */
function checkAllowlist(rt: Runtime, env: Env, url: string): void {
  const apps = rt.cfg.write_allowlist?.[env];
  if (!apps) return;
  const app = /\/applications\/([^/?]+)/.exec(new URL(url).pathname)?.[1];
  if (!app) throw refused(`write_allowlist.${env} is set in the config, and this write targets no app. Writes go to the listed apps only.`);
  if (!apps.includes(decodeURIComponent(app))) throw refused(`${decodeURIComponent(app)} is not in write_allowlist.${env} of the config. Writes go to the listed apps only.`);
}

export const safeJson = (t: string): unknown => {
  try {
    return JSON.parse(t);
  } catch {
    return { raw: t.slice(0, 2000) };
  }
};

/** Map an error response to a CliError. A confirm token is kept in `extra`. */
export function toError(res: HttpResponse): CliError {
  const body = safeJson(res.text) as { error?: { code?: string; messages?: string[] }; confirm_token?: string } | null;
  const code = body?.error?.code ?? `HTTP_${res.status}`;
  const messages = body?.error?.messages?.length ? body.error.messages : [`HTTP ${res.status}`];
  const extra: Record<string, unknown> = {};
  if (body?.confirm_token) extra.confirm_token = body.confirm_token;
  if (res.status === 429) {
    const reset = res.headers.get("x-ratelimit-reset");
    if (reset) extra.retry_at = Number(reset);
  }
  return new CliError(exitFor(body?.error?.code, res.status), code, messages, extra, res.status);
}

