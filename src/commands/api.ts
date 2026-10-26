import type { Command } from "commander";
import { usage } from "../core/errors.js";
import { addGlobals, type Runtime } from "../core/runtime.js";
import { BASE_URLS, type Env, type Globals } from "../core/types.js";
import { execute, managementCommands } from "../surfaces/management/index.js";
import { jsonBody, readData, withToken } from "../surfaces/management/request.js";

const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE"];
const PREFIX = "/management/v1";

/** The full path under /management/v1. Each segment is checked after decoding, so an encoded `..` or `/` is refused too. */
function checkPath(raw: string): string {
  const path = raw.startsWith(`${PREFIX}/`) ? raw.slice(PREFIX.length) : raw;
  const bad = usage(`The path must be under ${PREFIX}, such as /applications/app_xxx/placements, with no query, "..", or encoded "/". Got "${raw}".`);
  if (!path.startsWith("/")) throw bad;
  for (const seg of path.slice(1).split("/")) {
    let dec: string;
    try {
      dec = decodeURIComponent(seg);
    } catch {
      throw bad;
    }
    // A "%" left after one decoding is a double encoding.
    // A control character or a space: the URL parser strips a tab or a newline, so ".\t." would become "..".
    if (!dec || dec === "." || dec === ".." || /[/\\?#%\x00-\x20\x7f]/.test(dec)) throw bad;
  }
  return PREFIX + path;
}

let literals: Set<string> | undefined;
/** The segments that no id can be: the literal segments of the spec paths. Built once. */
const literalSegments = () =>
  (literals ??= new Set(managementCommands().flatMap((c) => [c.appOp, c.globalOp]).flatMap((x) => (x ? x.path.split("/").filter((s) => s && !s.startsWith("{")) : []))));

const HELP = `
The path is relative to /management/v1, or starts with it. The CLI sends it only to the Management API of --env.
A write (POST, PUT, PATCH, DELETE) needs --env prod or --env staging, also with --dry-run.
A risky change returns CONFIRMATION_REQUIRED: add --yes, answer the prompt, or resend with --confirm-token <token>.

Examples:
  purchasely api GET /applications/app_xxx/placements --query limit=10
  purchasely api PATCH /applications/app_xxx/placements/plc_xxx --data @placement.json --env prod --dry-run`;

export function register(program: Command, rt: Runtime): void {
  addGlobals(
    program
      .command("api")
      .description("Send a raw Management API request")
      .argument("<method>", METHODS.join(", "))
      .argument("<path>", "path under /management/v1, such as /applications/app_xxx/placements"),
  )
    .option("--query <k=v>", "query parameter (repeatable)", (v: string, prev: string[]) => [...prev, v], [] as string[])
    .option("--data <json>", "request body: JSON, @file.json or - for stdin")
    .option("--confirm-token <token>", "token from a CONFIRMATION_REQUIRED response")
    .addHelpText("after", HELP.replaceAll("purchasely ", `${rt.d.name} `))
    .action(async (m: string, raw: string, o: Globals & { query: string[]; data?: string; confirmToken?: string }) => {
      const method = m.toUpperCase();
      if (!METHODS.includes(method)) throw usage(`The method must be one of ${METHODS.join(", ")}.`);
      const path = checkPath(raw);
      const query = o.query.map((kv) => {
        const i = kv.indexOf("=");
        if (i < 1) throw usage(`--query takes k=v, got "${kv}".`);
        return `${encodeURIComponent(kv.slice(0, i))}=${encodeURIComponent(kv.slice(i + 1))}`;
      });
      if (method === "GET" && o.data !== undefined) throw usage("A GET takes no body. Do not pass --data.");
      const body = o.data === undefined ? undefined : jsonBody(await readData(String(o.data), rt.d.readStdin));
      // The spec operation gives the confirm placement and the logged path template. No match: no confirm retry, and the log keeps no literal id.
      const op = managementCommands()
        .flatMap((c) => [c.appOp, c.globalOp])
        .filter((x) => x?.method === method && new RegExp(`^${x.path.replace(/\{[^}]+\}/g, "[^/]+")}$`).test(path))
        .sort((a, b) => a!.path.split("{").length - b!.path.split("{").length)[0];
      if (o.confirmToken && !op) throw usage("No operation of the API spec matches this method and path, so the token has no place. Send it with --query confirm_token=<token>, or in the --data body.");
      // No match: the log keeps a segment only when it is a literal segment of the spec.
      const template = op?.path ?? PREFIX + path.slice(PREFIX.length).replace(/[^/]+/g, (seg) => (literalSegments().has(seg) ? seg : "{id}"));
      const build = (env: Env, token?: string) => {
        const url = new URL(BASE_URLS[env] + path + (query.length ? `?${query.join("&")}` : ""));
        // The URL parser must send the path that was checked: it encodes a non-ASCII character and nothing else.
        if (url.origin !== new URL(BASE_URLS[env]).origin || url.pathname !== path.replace(/[^\x21-\x7e]/gu, encodeURIComponent)) throw usage(`The path must be under ${PREFIX}, with no dot segment after normalisation.`);
        return { method, ...withToken(op?.confirm, url.href, body, token), pathTemplate: template, confirm: op?.confirm };
      };
      return execute(rt, o, build, { name: `api-${method.toLowerCase()}`, confirmToken: o.confirmToken, refuseRedirect: true });
    });
}
