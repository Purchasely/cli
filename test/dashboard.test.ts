import { describe, expect, it } from "vitest";
import { sparkline } from "../src/commands/dashboard.js";
import { setup } from "./helpers.js";

const day = (i: number) => `2026-10-0${i + 1}`;
const replies = (vals: number[]) => [
  { body: { mrr_evolution_live: { data: { mrr: { values: { by_day: vals.map((v, i) => ({ period: day(i), date_kinds: ["date_period"], value: v * 10 })), by_month: [] }, variations: [] } } } } },
  { body: { subscription_evolution: vals.map((v, i) => ({ period: day(i), group_by: null, count: v })) } },
  { body: { conversion: {
    kpi: [{ unique_viewers_count: 42 }],
    primary_metric_by_period: vals.map((v, i) => ({ period: day(i), unique_viewers_count: v })),
    secondary_metric_by_period: vals.map((v, i) => ({ period: day(i), total_conversions_count: v })),
  } } },
];

describe("dashboard summary", () => {
  it("sends 3 requests and returns the panels as JSON", async () => {
    const t = setup({ replies: replies([1, 2, 3]) });
    const r = await t.exec("dashboard", "summary", "--app", "app_1", "--days", "30");
    expect(r.code).toBe(0);
    expect(t.calls.map((c) => new URL(c.url).pathname)).toEqual([
      "/management/v1/applications/app_1/dashboard/mrr_evolution_live",
      "/management/v1/applications/app_1/dashboard_v2/subscription_evolution",
      "/management/v1/applications/app_1/dashboard_v2/conversion",
    ]);
    expect(t.calls[0].url).toContain("periodicity=DAY");
    // The fake clock is 2023-11-14T22:13:20Z.
    expect(t.calls[0].url).toContain("date_range[]=2023-10-16&date_range[]=2023-11-14");
    const out = r.json();
    expect(out.panels.map((p: { name: string; value: number }) => [p.name, p.value])).toEqual([
      ["MRR", 30], ["Active subscriptions", 3], ["Paywall unique viewers", 42], ["Conversions", 6],
    ]);
  });

  it("draws a sparkline per metric with --output table", async () => {
    const t = setup({ replies: replies([1, 2, 3]) });
    const r = await t.exec("dashboard", "summary", "--app", "app_1", "--output", "table");
    expect(r.code).toBe(0);
    expect(r.out).toMatch(/^MRR\s+30\s+▁▅█$/m);
    expect(r.out).toMatch(/^Conversions\s+6\s+▁▅█$/m);
  });

  it("survives all zeros and empty series", async () => {
    expect(sparkline([0, 0, 0])).toBe("▁▁▁");
    const t = setup({ replies: [{ body: { mrr_evolution_live: {} } }, { body: { subscription_evolution: [] } }, { body: { conversion: {} } }] });
    const r = await t.exec("dashboard", "summary", "--app", "app_1", "--output", "table");
    expect(r.code).toBe(0);
    expect(r.out).toContain("(no data)");
  });

  it("reads today's MRR apart above 60 days, because a WEEK bucket starts on its first day", async () => {
    const today = { body: { mrr_evolution_live: { data: { mrr: { values: { by_day: [{ period: "2023-11-14", value: 99 }] } } } } } };
    const t = setup({ replies: [...replies([1, 2, 3]), today] });
    const r = await t.exec("dashboard", "summary", "--app", "app_1", "--days", "61");
    expect(r.code).toBe(0);
    expect(t.calls[0].url).toContain("periodicity=WEEK");
    expect(t.calls[3].url).toContain("date_range[]=2023-11-14&date_range[]=2023-11-14&periodicity=DAY");
    expect(r.json().panels[0]).toMatchObject({ name: "MRR", value: 99 });
    expect(r.json().panels[0].series).toHaveLength(3);

    const m = setup({ replies: [...replies([1]), today] });
    await m.exec("dashboard", "summary", "--app", "app_1", "--days", "184");
    expect(m.calls[0].url).toContain("periodicity=MONTH");
  });

  it("names the app when it is not found, before the server message", async () => {
    const t = setup({ replies: [{ status: 404, body: { error: { code: "NOT_FOUND", messages: ["Not found"] } } }] });
    const r = await t.exec("dashboard", "summary", "--app", "app_9");
    expect(r.code).toBe(6);
    expect(r.errJson().error.messages).toEqual(['application "app_9" not found', "Not found"]);
  });

  it("--dry-run prints the requests and sends nothing", async () => {
    const t = setup();
    const r = await t.exec("dashboard", "summary", "--app", "app_1", "--dry-run");
    expect(r.code).toBe(0);
    expect(t.calls).toHaveLength(0);
    expect(r.json()).toHaveLength(3);
    expect(r.json()[0].headers.Authorization).toBe("Bearer <redacted>");
  });

  it("maps an API error to its exit code, and an empty body to no data", async () => {
    const f = setup({ replies: [{ status: 403, body: { error: { code: "FORBIDDEN", messages: ["no scope"] } } }] });
    expect((await f.exec("dashboard", "summary", "--app", "app_1")).code).toBe(4);
    const e = setup({ replies: [{ text: "" }, { text: "null" }, { text: "not json" }] });
    const r = await e.exec("dashboard", "summary", "--app", "app_1", "--output", "table");
    expect(r.code).toBe(0);
    expect(r.out).toContain("(no data)");
  });

  it("checks --app and --days before any request", async () => {
    const t = setup();
    expect((await t.exec("dashboard", "summary")).code).toBe(2);
    for (const d of ["1", "731", "1e2", "0x1e"]) expect((await t.exec("dashboard", "summary", "--app", "app_1", "--days", d)).code, d).toBe(2);
    expect(t.calls).toHaveLength(0);
  });
});
