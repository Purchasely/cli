import { describe, expect, it } from "vitest";
import { deriveCommands, loadSpec, parseOps } from "../src/surfaces/management/spec.js";
import { setup } from "./helpers.js";

const cmds = deriveCommands(parseOps(loadSpec()));
const V2 = ["conversion", "mau-reports", "paid-subscription-movements", "paid-subscription-retention", "paywall-latency", "sdk-evolution", "sdk-versions", "subscription-activations", "subscription-evolution", "subscription-reactivations"];
const RANGE = ["--app", "app_1", "--date-range", "2026-01-01,2026-10-01"];

describe("one dashboard group", () => {
  it("holds the actions of the spec tag Dashboard V2 next to the others, with no name collision", () => {
    const actions = cmds.filter((c) => c.resource === "dashboard").map((c) => c.action);
    expect(new Set(actions).size).toBe(actions.length);
    expect(actions).toHaveLength(22);
    expect(actions).toEqual(expect.arrayContaining(V2));
    expect(actions).toContain("header-live");
    expect(actions.filter((a) => a.endsWith("-dashboard"))).toEqual([]);
    expect(cmds.filter((c) => c.resource === "dashboard-v2")).toEqual([]);
  });

  it("dashboard conversion builds the same request as the old dashboard-v2 conversion", async () => {
    const a = setup();
    const b = setup();
    expect((await a.exec("dashboard", "conversion", ...RANGE)).code).toBe(0);
    expect((await b.exec("dashboard-v2", "conversion", ...RANGE)).code).toBe(0);
    expect(a.calls).toHaveLength(1);
    expect(b.calls.map((c) => [c.init.method, c.url])).toEqual(a.calls.map((c) => [c.init.method, c.url]));
    expect(a.calls[0].url).toContain("/applications/app_1/dashboard_v2/conversion?");
  });

  it("every old dashboard-v2 action still runs", async () => {
    for (const action of V2) {
      const t = setup();
      const dated = !["mau-reports", "paywall-latency"].includes(action);
      expect((await t.exec("dashboard-v2", action, ...(dated ? RANGE : ["--app", "app_1"]))).code, action).toBe(0);
      expect(t.calls, action).toHaveLength(1);
    }
  });

  it("dashboard-v2 is hidden from help, the reference, search and completion", async () => {
    const t = setup({ env: { PURCHASELY_API_KEY: "" } });
    const shown = [
      (await t.exec("--help")).out,
      (await t.exec("dashboard", "--help")).out,
      (await t.exec("dashboard", "conversion", "--help")).out,
      (await t.exec("search", "conversion paywall retention", "--limit", "50")).out,
      ...(await Promise.all(["markdown", "man", "html", "json"].map((f) => t.exec("reference", "--format", f)))).map((r) => r.out),
      ...(await Promise.all(["bash", "zsh", "fish"].map((s) => t.exec("completion", s)))).map((r) => r.out),
    ];
    shown.forEach((out, i) => {
      expect(out.length, String(i)).toBeGreaterThan(0);
      expect(out, String(i)).not.toContain("dashboard-v2");
    });
    expect((await t.exec("dashboard")).out).not.toContain("dashboard-v2");
    expect((await t.exec("dashboard", "--output", "json")).json().map((x: { command: string }) => x.command)).toContain("purchasely dashboard conversion");
  });

  it("the JSON list has one row per dashboard command: summary and the 22 actions", async () => {
    const t = setup();
    t.d.io = { stdin: { isTTY: false }, stdout: { isTTY: false } } as any;
    expect((await t.exec("dashboard")).json()).toHaveLength(23);
  });

  it("the help of a dashboard action names no internal system", async () => {
    const t = setup();
    const h = (await t.exec("dashboard", "paid-subscription-retention", "--help")).out;
    expect(h).toContain("Paid-subscription retention cohort matrix for a single application");
    expect(h).not.toMatch(/clickhouse|dynamodb|redis|sidekiq/i);
    const s = (await t.exec("search", "paid subscription retention cohort")).out;
    expect(s).not.toMatch(/clickhouse|dynamodb|redis|sidekiq/i);
  });
});

describe("plain help", () => {
  it("names no internal system in any summary or description", () => {
    const ops = parseOps(loadSpec());
    expect(ops.filter((o) => /ClickHouse/.test(o.summary + o.description)).map((o) => o.path)).toEqual([]);
  });
});
