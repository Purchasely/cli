import { readFileSync } from "node:fs";
import { ROOT } from "../../core/root.js";
import { deriveName, kebab, singular, tagSlug } from "./naming.js";

export interface Schema {
  type?: string;
  enum?: string[];
  items?: Schema;
  description?: string;
  anyOf?: Schema[];
  properties?: Record<string, Schema>;
  required?: string[];
}

export interface Param {
  name: string;
  in: "path" | "query";
  required: boolean;
  description?: string;
  schema: Schema;
}

export interface Op {
  method: string;
  path: string;
  tag: string;
  summary: string;
  description: string;
  scope: string;
  params: Param[];
  body?: { required: boolean; schema: Schema };
  /** Where `confirm_token` goes, read from the spec. */
  confirm?: "query" | "body";
  app: boolean;
}

export interface Opt {
  /** Spec name (`plan_id`). */
  name: string;
  flag: string;
  kind: "path" | "query";
}

export interface Command {
  resource: string;
  action: string;
  summary: string;
  appOp?: Op;
  globalOp?: Op;
  /** Union of the options of both variants. */
  options: Opt[];
  /** The resource of the spec tag when the client sees another one: the hidden alias (`dashboard-v2`). */
  legacy?: string;
}

export interface Spec {
  baseUrl: string;
  ops: Op[];
}

const METHODS = ["get", "post", "put", "patch", "delete"];

/** The server answers 500 without `date_range` on these, although the spec says optional (SPEC_GAPS.md #23). */
const NEEDS_DATE_RANGE = new Set(
  ["conversion", "paid_subscription_movements", "paid_subscription_retention", "sdk_evolution", "sdk_versions", "subscription_activations", "subscription_evolution", "subscription_reactivations"]
    .map((a) => `GET /management/v1/applications/{application_id}/dashboard_v2/${a}`),
);

/** The spec names an internal system in these summaries; the client must not read it. Plain words, no spec edit. */
const SUMMARY_OVERRIDE: Record<string, string> = {
  "GET /management/v1/applications/{application_id}/dashboard_v2/paid_subscription_retention": "Paid-subscription retention cohort matrix for a single application",
};

/** A client reads the help: no name of an internal system in a summary or a description. */
const plain = (t: string) => t.replace(/ \(ClickHouse\)/g, "");

export function loadSpec(file = new URL("spec/management.json", ROOT)): any {
  return JSON.parse(readFileSync(file, "utf8"));
}

export function parseOps(doc: any): Op[] {
  const ops: Op[] = [];
  for (const [path, item] of Object.entries<any>(doc.paths)) {
    for (const m of METHODS) {
      const o = item[m];
      if (!o) continue;
      const params: Param[] = (o.parameters ?? []).map((p: any) => ({
        name: p.name, in: p.in, description: p.description, schema: p.schema ?? {},
        required: !!p.required || (p.name === "date_range" && NEEDS_DATE_RANGE.has(`${m.toUpperCase()} ${path}`)),
      }));
      const bodySchema: Schema | undefined = o.requestBody?.content?.["application/json"]?.schema;
      const summary: string = o.summary ?? "";
      const reworded = SUMMARY_OVERRIDE[`${m.toUpperCase()} ${path}`];
      const op: Op = {
        method: m.toUpperCase(),
        path,
        tag: o.tags[0],
        summary: plain(reworded ?? summary),
        description: plain(reworded && summary ? (o.description ?? "").replace(summary, reworded) : (o.description ?? "")),
        scope: o["x-required-scope"] ?? "",
        params,
        app: path.includes("{application_id}"),
      };
      if (o.requestBody) op.body = { required: !!o.requestBody.required, schema: bodySchema ?? {} };
      if (params.some((p) => p.in === "query" && p.name === "confirm_token")) op.confirm = "query";
      else if (bodySchema?.properties?.confirm_token) op.confirm = "body";
      ops.push(op);
    }
  }
  return ops;
}

const optsOf = (op: Op | undefined): Opt[] =>
  (op?.params ?? [])
    .filter((p) => p.name !== "application_id" && p.name !== "confirm_token")
    .map((p) => ({ name: p.name, flag: kebab(p.name), kind: p.in }));

export function deriveCommands(ops: Op[]): Command[] {
  const named = ops.map((op) => ({ op, ...deriveName(op.method, op.path, op.tag) }));
  const groups = new Map<string, typeof named>();
  for (const n of named) {
    const k = `${n.resource} ${n.action}`;
    groups.set(k, [...(groups.get(k) ?? []), n]);
  }
  const cmds = new Map<string, Command>();
  for (const [key, list] of groups) {
    const apps = list.filter((n) => n.op.app);
    const globals = list.filter((n) => !n.op.app);
    if (apps.length <= 1 && globals.length <= 1) {
      const [resource, action] = key.split(" ");
      add(cmds, resource, action, apps[0]?.op, globals[0]?.op);
      continue;
    }
    // Collision: split on the singular parent literal (`flow`, `screen`).
    for (const n of list) {
      const action = n.action === "get" ? n.parent! : `${n.action}-${n.parent}`;
      add(cmds, n.resource, action, n.op.app ? n.op : undefined, n.op.app ? undefined : n.op);
    }
  }
  return [...cmds.values()];
}

function add(cmds: Map<string, Command>, resource: string, action: string, appOp?: Op, globalOp?: Op) {
  const k = `${resource} ${action}`;
  if (cmds.has(k)) throw new Error(`duplicate command ${k}`);
  const options: Opt[] = [];
  for (const o of [...optsOf(appOp), ...optsOf(globalOp)]) if (!options.some((x) => x.name === o.name)) options.push(o);
  const slug = tagSlug((appOp ?? globalOp)!.tag);
  cmds.set(k, { resource, action, summary: (appOp ?? globalOp)!.summary, appOp, globalOp, options, ...(slug === resource ? {} : { legacy: slug }) });
}

export { singular };
