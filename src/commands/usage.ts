import { rmSync } from "node:fs";
import { join } from "node:path";
import { Command } from "commander";
import { credentialDisplay, keyHash, type Surface } from "../core/credentials.js";
import { limits } from "../core/http.js";
import { emit } from "../core/output.js";
import { addGlobals, type Runtime } from "../core/runtime.js";
import type { Globals } from "../core/types.js";
import { readUsage } from "../core/usage.js";

/**
 * Requests and rejections of the last hour, one row per env, surface and key, for the given surfaces only.
 * `key` stays the identity (the hash). `label` is the label of the current credential, else `hash:<key>`.
 */
export function lastHour(rt: Runtime, surfaces: Surface[], states: { surface: string; key: string; label: string }[]) {
  const { d } = rt;
  const labels = new Map(states.map((s) => [`${s.surface}/${s.key}`, s.label]));
  const by = new Map<string, { env: string; surface: string; key: string; label: string; requests: number; rejections: number }>();
  for (const l of readUsage(d)) {
    if (!(surfaces as string[]).includes(l.surface) || d.now() - Date.parse(l.ts) >= 3600_000) continue;
    const k = `${l.env}/${l.surface}/${l.key}`;
    const row = by.get(k) ?? { env: l.env, surface: l.surface, key: l.key, label: labels.get(`${l.surface}/${l.key}`) ?? `hash:${l.key}`, requests: 0, rejections: 0 };
    row.requests++;
    if (l.rate_limited) row.rejections++;
    by.set(k, row);
  }
  return [...by.values()];
}

/** Limiter and breaker state of one surface. `stateKey` is what the client hashes into its state file name. `label` is `credentialDisplay` of the current credential. */
export function limiterState(rt: Runtime, g: Globals, surface: Surface, stateKey: string, label: string) {
  const { d } = rt;
  const { store, breaker } = limits(d, rt.env(g), surface, stateKey);
  const b = breaker.state();
  const lim = store.read<{ inflight?: unknown[] }>("limiter", {});
  return {
    env: rt.env(g), surface, key: keyHash(stateKey), label, rate: rt.rate(surface, g),
    in_flight: lim.inflight?.length ?? 0,
    breaker: { paused_until: b.paused_until > d.now() ? new Date(b.paused_until).toISOString() : null, rate_factor: b.rate_factor, rejections_last_5_min: b.rejections.length },
  };
}

export function register(program: Command, rt: Runtime): void {
  const { d } = rt;
  rt.taken.add("usage").add("purge");

  addGlobals(program.command("usage").description("Requests of the last hour, rejections, limiter and breaker state"))
    .addHelpText("after", "\nManagement API only.\n")
    .action(async (g: Globals) => {
      const cred = await rt.credential(rt.env(g), "management");
      const states = cred ? [limiterState(rt, g, "management", cred.key, credentialDisplay("management", cred.key))] : [];
      emit(d, g, { last_hour: lastHour(rt, ["management"], states), current: states[0] ?? null, surfaces: states }, { env: rt.env(g), key: cred?.key ?? null, name: "usage" });
    });

  addGlobals(program.command("purge").description("Delete the cached client data"))
    .addHelpText("after", "\nDeletes the cached outputs over 200 KB. Without --env it deletes the cache of every environment.\n")
    .action(async (g: Globals) => {
      const target = g.env ? join(d.cacheDir, rt.env(g)) : d.cacheDir;
      rmSync(target, { recursive: true, force: true });
      d.stdout(JSON.stringify({ purged: target }) + "\n");
    });
}
