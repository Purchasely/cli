import { chmodSync, mkdirSync, readdirSync, statSync, unlinkSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { keyHash } from "./credentials.js";
import type { Deps, Globals } from "./types.js";

export const MAX_STDOUT_BYTES = 200 * 1024;
const CACHE_TTL_MS = 24 * 3600_000;

const cell = (v: unknown): string => (v !== null && typeof v === "object" ? JSON.stringify(v) : String(v ?? ""));

/** Rows that fit `width`, separators included: the widest column shrinks first, and a cut cell ends with "...". */
function grid(rows: string[][], width: number): string {
  const widths = rows[0].map((_, i) => Math.max(...rows.map((r) => r[i].length)));
  while (widths.reduce((a, b) => a + b, 2 * (widths.length - 1)) > width) {
    const i = widths.indexOf(Math.max(...widths));
    if (widths[i] <= 3) break;
    widths[i]--;
  }
  const fit = (c: string, w: number) => (c.length > w ? c.slice(0, w - 3) + "..." : c);
  return rows.map((r) => r.map((c, i) => fit(c, widths[i]).padEnd(widths[i])).join("  ").trimEnd()).join("\n");
}

const isRecord = (v: unknown): v is Record<string, unknown> => v !== null && typeof v === "object" && !Array.isArray(v);
const FIRST = ["id", "vendor_id", "name", "status", "type"];

/** Without `fields`: at most 6 scalar columns, `FIRST` in front. With `fields`: those columns, in that order. */
export function toTable(body: unknown, fields?: string[], width = process.stdout.columns || 120): string | null {
  if (!isRecord(body)) return null;
  const roots = Object.keys(body).filter((k) => k !== "meta");
  if (roots.length === 1) {
    const v = body[roots[0]];
    if (Array.isArray(v) && v.length > 0 && v.every(isRecord)) {
      const keys = [...new Set(v.flatMap((o) => Object.keys(o)))];
      const scalar = keys.filter((k) => v.every((o) => o[k] === null || typeof o[k] !== "object"));
      const pick = scalar.length ? scalar : keys;
      const cols = fields?.length && !fields.includes("*") ? fields : [...FIRST.filter((k) => pick.includes(k)), ...pick.filter((k) => !FIRST.includes(k))].slice(0, 6);
      return grid([cols, ...v.map((o) => cols.map((c) => cell(o[c])))], width);
    }
    if (isRecord(v)) body = v;
  }
  return grid([["key", "value"], ...Object.entries(body as Record<string, unknown>).map(([k, v]) => [k, cell(v)])], width);
}

export function cacheDirFor(d: Pick<Deps, "cacheDir" | "now">, env: string, key: string | null): string {
  const dir = join(d.cacheDir, env, key ? keyHash(key) : "anonymous");
  mkdirSync(dir, { recursive: true, mode: 0o700 });
  for (let p = dir; p.startsWith(d.cacheDir) && p !== d.cacheDir; p = join(p, "..")) chmodSync(p, 0o700);
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (d.now() - statSync(p).mtimeMs > CACHE_TTL_MS) unlinkSync(p);
  }
  return dir;
}

/** Print a JSON value or a table. Large JSON goes to a file unless `--out -`. */
export function emit(d: Deps, g: Globals, value: unknown, ctx: { env: string; key: string | null; name: string; columns?: string[] }): void {
  const table = g.output === "table" ? toTable(value, ctx.columns) : null;
  if (table !== null) return d.stdout(table + "\n");
  const text = JSON.stringify(value, null, 2) + "\n";
  const bytes = Buffer.byteLength(text);
  if (g.out === "-" || (!g.out && bytes <= MAX_STDOUT_BYTES)) return d.stdout(text);
  let file: string;
  if (g.out) file = resolve(g.out);
  else file = join(cacheDirFor(d, ctx.env, ctx.key), `${d.now()}-${ctx.name.replace(/\W+/g, "-")}.json`);
  writeFileSync(file, text, { mode: 0o600 });
  d.stdout(JSON.stringify({ file, bytes }) + "\n");
}
