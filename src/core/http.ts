import { randomUUID } from "node:crypto";
import { Breaker, pauseFromHeaders } from "./breaker.js";
import { CliError, EXIT } from "./errors.js";
import { hint, keyHash, what, type Surface } from "./credentials.js";
import { demo, demoResponse, demoWriteError } from "./demo.js";
import { Limiter } from "./limiter.js";
import { StateStore } from "./state.js";
import { logUsage } from "./usage.js";
import type { Deps, Env } from "./types.js";

export interface Ctx {
  d: Deps;
  env: Env;
  surface: Surface;
  key: string | null;
  identity?: string;
  reason?: string;
  /** The command path, never its arguments. Sent in the User-Agent. */
  command?: string;
  /** Refuse every write before it is sent (`PURCHASELY_READ_ONLY=1` or `read_only` in the config). */
  readOnly?: boolean;
  /** Requests per second, already checked against the hard cap. */
  rate: number;
  /** Replaces the `Authorization: Bearer` header (the Console API uses its own headers). It can also replace `Accept`. */
  headers?: Record<string, string>;
}

export interface HttpRequest {
  method: string;
  url: string;
  /** Path with `{params}` unreplaced. Logged instead of the real path. */
  pathTemplate: string;
  body?: string;
  /** A write pays the write gap. Default: any method but GET. A JSON-RPC read is a POST that is not a write. */
  write?: boolean;
  /** Pause in ms when this response is a rate limit that is not a 429 status (tool tier limit, batch limit), else null. */
  rateLimited?: (res: HttpResponse) => number | null;
  /** Do not follow a redirect: a 3xx answer is a REFUSED error (`api`, whose path the user chose). */
  refuseRedirect?: boolean;
}

export interface HttpResponse {
  status: number;
  headers: Headers;
  text: string;
}

/** One request, headers and body, must end in this time. A slow query can take a minute. */
export const REQUEST_TIMEOUT_MS = 120_000;

export const stateKey = (env: string, surface: string, key: string) => `${env}-${surface}-${keyHash(key)}`;

/**
 * The variable that each agent sets in the shell of its tool calls. Checked in the source code (Codex,
 * Gemini CLI), on the machine (Claude Code), or by the vendor on its forum (Cursor). Before `CI`: Cursor sets both.
 * shortcut: no variable found for Copilot CLI and OpenCode, they count as tty, script or ci. Add them when found.
 */
const AGENT_VARS: [string, string][] = [["CLAUDECODE", "claude-code"], ["CODEX_THREAD_ID", "codex"], ["GEMINI_CLI", "gemini-cli"], ["CURSOR_AGENT", "cursor"]];

/** Who runs the CLI: `agent:<name>`, `ci`, `tty` or `script`. */
export function runner(d: Pick<Deps, "env" | "isTTY">): string {
  const agent = AGENT_VARS.find(([v]) => d.env[v]);
  if (agent) return `agent:${agent[1]}`;
  if (d.env.CI) return "ci";
  return d.isTTY ? "tty" : "script";
}

export function userAgent(ctx: Pick<Ctx, "d" | "identity" | "reason" | "command">): string {
  const clean = (s: string) => s.replace(/[\r\n();]+/g, " ").trim();
  // Only an extension with `identity` sends the email. `purchasely` never does, even when the shared config has one.
  const who = (ctx.d.ext?.identity && clean(ctx.identity ?? "")) || "-";
  return `purchasely-cli/${ctx.d.version} (${who}; ${clean(ctx.reason ?? "") || "-"}; ${clean(ctx.command ?? "") || "-"}; ${runner(ctx.d)})`;
}

export function readOnlyError(d: Pick<Deps, "env">): CliError {
  const why = d.env.PURCHASELY_READ_ONLY === "1" ? "PURCHASELY_READ_ONLY=1 is set" : "read_only is true in the config";
  return new CliError(EXIT.REFUSED, "REFUSED", [`Read-only mode: ${why}. This command writes, so the CLI sent nothing.`]);
}

export function limits(d: Deps, env: string, surface: Surface, key: string) {
  const store = new StateStore(d, stateKey(env, surface, key));
  return { store, limiter: new Limiter(d, store), breaker: new Breaker(d, store) };
}

export async function send(ctx: Ctx, req: HttpRequest): Promise<HttpResponse> {
  const { d } = ctx;
  const write = req.write ?? req.method !== "GET";
  // Demo mode answers from the sample data: no key, no limiter state, no request, no usage line.
  if (demo(d)) {
    if (write) throw demoWriteError();
    return demoResponse(req, d.name);
  }
  if (!ctx.key) throw new CliError(EXIT.AUTH, "UNAUTHORIZED", [`No ${what(ctx.surface)}. ${hint(ctx.surface, ctx.d.name)}`]);
  const { limiter, breaker } = limits(d, ctx.env, ctx.surface, ctx.key);
  if (write && ctx.readOnly) throw readOnlyError(d);

  for (let attempt = 0; ; attempt++) {
    const factor = await breaker.gate();
    const { release } = await limiter.acquire({ rate: ctx.rate * factor, write });
    const requestId = randomUUID();
    let res: Response;
    let text: string;
    try {
      res = await d.fetch(req.url, {
        method: req.method,
        headers: {
          Accept: "application/json",
          ...(ctx.headers ?? { Authorization: `Bearer ${ctx.key}` }),
          "User-Agent": userAgent(ctx),
          "X-Request-Id": requestId,
          ...(req.body !== undefined ? { "Content-Type": "application/json" } : {}),
        },
        body: req.body,
        // Also covers the body read: a server that sends the headers and then stops ends here.
        signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
        ...(req.refuseRedirect ? { redirect: "manual" as const } : {}),
      });
      text = await res.text();
    } catch (e) {
      // Never echo the error message: it could carry request details.
      const code = ((e as { cause?: { code?: string } }).cause?.code ?? (e as Error).name) || "Error";
      throw new CliError(EXIT.GENERIC, "NETWORK_ERROR", [`The request failed before a response (${code}).`]);
    } finally {
      await release();
    }
    const out: HttpResponse = { status: res.status, headers: res.headers, text };
    // One path for every rate limit: the body of the answer first, the headers of a 429 as the fallback.
    const pause = req.rateLimited?.(out) ?? (res.status === 429 ? pauseFromHeaders(res.headers, d.now()) : null);
    logUsage(d, {
      env: ctx.env,
      surface: ctx.surface,
      key: keyHash(ctx.key),
      method: req.method,
      path: req.pathTemplate,
      status: res.status,
      rate_limited: pause !== null,
      request_id: requestId,
    });
    if (req.refuseRedirect && res.status >= 300 && res.status < 400) {
      throw new CliError(EXIT.REFUSED, "REFUSED", [`The server answered with a redirect (HTTP ${res.status}). The CLI does not follow it.`], {}, res.status);
    }
    if (pause !== null) {
      await breaker.record(pause);
      if (attempt === 0) continue; // one retry after the shared pause, no loop
    }
    return out;
  }
}
