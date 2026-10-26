import { readFileSync } from "node:fs";
import { usage } from "../../core/errors.js";
import { BASE_URLS, type Env } from "../../core/types.js";
import type { Command as Cmd, Op, Param } from "./spec.js";
import { kebab } from "./naming.js";

export type Values = Record<string, unknown>;

export const attr = (flag: string) => flag.replace(/-([a-z0-9])/g, (_, c: string) => c.toUpperCase());

const DATE = /^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2}(:\d{2}(\.\d+)?)?(Z|[+-]\d{2}:\d{2})?)?$/;

/** One occurrence of an array option: a JSON array of scalars when it starts with `[`, else `a,b`. */
function splitItems(flag: string, example: string, s: string): string[] {
  if (!s.trimStart().startsWith("[")) return s.split(",");
  let items: unknown[];
  try {
    items = JSON.parse(s);
  } catch {
    throw usage(`${flag} is not a valid JSON array. Use ${flag} ${example}.`);
  }
  if (!items.every((x) => x !== null && typeof x !== "object")) throw usage(`${flag} takes a JSON array of strings, numbers or booleans.`);
  return items.map(String);
}

/** Parse a raw option value according to its schema. */
export function parseValue(p: Param, raw: unknown): unknown {
  const flag = `--${kebab(p.name)}`;
  const check = (s: string, schema = p.schema): string => {
    if (schema.enum && !schema.enum.includes(s)) throw usage(`${flag} must be one of: ${schema.enum.join(", ")}.`);
    return s;
  };
  if (p.schema.type === "array") {
    const items = (Array.isArray(raw) ? raw : [raw]).flatMap((v) => splitItems(flag, p.name === "date_range" ? "2026-01-01,2026-10-01" : "a,b", String(v))).filter((v) => v !== "");
    if (p.name === "date_range" && (items.length !== 2 || !items.every((v) => DATE.test(v)))) {
      throw usage(`${flag} takes two dates (YYYY-MM-DD or ISO date-times): --date-range 2026-01-01,2026-10-01.`);
    }
    return items.map((v) => check(v, p.schema.items ?? {}));
  }
  if (p.schema.type === "integer") {
    if (!/^-?\d+$/.test(String(raw))) throw usage(`${flag} must be an integer.`);
    return Number(raw);
  }
  if (p.schema.type === "boolean") return raw === true || raw === "true";
  return check(String(raw));
}

export interface Built {
  op: Op;
  url: string;
  body?: string;
}

/** Pick the variant, check the options against it, and build URL and body. */
export function buildRequest(
  cmd: Cmd,
  values: Values,
  env: Env,
  data: string | undefined,
  confirmToken?: string,
): Built {
  const app = values.application_id as string | undefined;
  const op = app ? cmd.appOp : cmd.globalOp;
  if (!op) {
    throw usage(app ? `${cmd.resource} ${cmd.action} has no app-scoped form. Do not pass --app.` : `--app is required for ${cmd.resource} ${cmd.action}.`);
  }
  for (const o of cmd.options) {
    if (values[o.name] !== undefined && !op.params.some((p) => p.name === o.name)) {
      throw usage(`--${o.flag} is not accepted ${app ? "with --app" : "without --app"} (${op.method} ${op.path}).`);
    }
  }
  let path = op.path;
  const query: string[] = [];
  for (const p of op.params) {
    if (p.name === "confirm_token") continue;
    const v = p.name === "application_id" ? app : values[p.name];
    if (v === undefined) {
      if (p.required) throw usage(`--${p.name === "application_id" ? "app" : kebab(p.name)} is required.`);
      continue;
    }
    if (p.in === "path") path = path.replace(`{${p.name}}`, encodeURIComponent(String(v)));
    else if (Array.isArray(v)) for (const x of v) query.push(`${encodeURIComponent(p.name)}[]=${encodeURIComponent(String(x))}`);
    else query.push(`${encodeURIComponent(p.name)}=${encodeURIComponent(String(v))}`);
  }
  let body: string | undefined;
  if (op.body) {
    if (data === undefined && op.body.required) throw usage("--data is required: pass '<json>' or @file.json.");
    if (data !== undefined) body = jsonBody(data);
  } else if (data !== undefined) {
    throw usage(`${op.method} ${op.path} takes no body. Do not pass --data.`);
  }
  return { op, ...withToken(op.confirm, BASE_URLS[env] + path + (query.length ? `?${query.join("&")}` : ""), body, confirmToken) };
}

export function jsonBody(data: string): string {
  try {
    return JSON.stringify(JSON.parse(data));
  } catch {
    throw usage("--data is not valid JSON.");
  }
}

/** Put the confirm token where the operation reads it: the query or the JSON body. */
export function withToken(confirm: Op["confirm"], url: string, body: string | undefined, token?: string): { url: string; body?: string } {
  if (!token || !confirm) return { url, body };
  if (confirm === "query") return { url: `${url}${url.includes("?") ? "&" : "?"}confirm_token=${encodeURIComponent(token)}`, body };
  const parsed: unknown = body === undefined ? {} : JSON.parse(body);
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) throw usage("--data must be a JSON object.");
  return { url, body: JSON.stringify({ ...parsed, confirm_token: token }) };
}

export function readData(raw: string, stdin: () => Promise<string>): Promise<string> | string {
  if (raw === "-") return stdin();
  if (raw.startsWith("@")) {
    try {
      return readFileSync(raw.slice(1), "utf8");
    } catch {
      throw usage(`Cannot read the file ${raw.slice(1)}.`);
    }
  }
  return raw;
}
