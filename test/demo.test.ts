import { existsSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { autoUpdateOff } from "../src/core/update.js";
import { setup } from "./helpers.js";

// No key at all: an empty PURCHASELY_API_KEY falls through to the empty fake keychain.
const demo = (env: Record<string, string> = {}) => setup({ env: { PURCHASELY_DEMO: "1", PURCHASELY_API_KEY: "", ...env } });

describe("demo mode", () => {
  it("answers the first reads from the sample data, with no key and no request", async () => {
    const t = demo();
    const reads: [string[], string][] = [
      [["applications", "list"], "mobile_applications"],
      [["apps", "get", "--app", "app_demo"], "mobile_application"],
      [["placements", "list", "--app", "app_demo"], "placements"],
      [["placements", "get", "--app", "app_demo", "--id", "home"], "placement"],
      [["placements", "list"], "placements"],
      [["screens", "list", "--app", "app_demo"], "screens"],
      [["products", "list", "--app", "app_demo"], "products"],
      [["plans", "list", "--app", "app_demo", "--product-id", "prd_demo_premium"], "plans"],
      [["campaigns", "list", "--app", "app_demo"], "campaigns"],
    ];
    for (const [argv, root] of reads) {
      const r = await t.exec(...argv);
      expect(r.code, argv.join(" ")).toBe(0);
      expect(Object.keys(r.json()), argv.join(" ")).toEqual([root]);
    }
    expect(t.calls).toHaveLength(0);
    // No limiter state, no usage log.
    expect(existsSync(join(t.root, "state"))).toBe(false);
    const apps = (await t.exec("applications", "list")).json().mobile_applications;
    expect(apps.map((a: { id: string }) => a.id)).toEqual(["app_demo"]);
  });

  it("wins over a real key: still no request", async () => {
    const t = setup({ env: { PURCHASELY_DEMO: "1" } });
    const r = await t.exec("placements", "list", "--app", "app_demo");
    expect(r.code).toBe(0);
    expect(t.calls).toHaveLength(0);
    expect(r.err).toContain("key=-");
  });

  it("prints one header line that says the data is not real", async () => {
    const r = await demo().exec("screens", "list", "--app", "app_demo");
    expect(r.err.trim().split("\n")).toEqual(["purchasely env=prod surface=management identity=- key=- demo=on (sample data, not real)"]);
  });

  it("dashboard summary renders the sample dashboard", async () => {
    const t = demo();
    const json = await t.exec("dashboard", "summary", "--app", "app_demo");
    expect(json.code).toBe(0);
    const panels = json.json().panels as { name: string; value: number; series: unknown[] }[];
    expect(panels.map((p) => p.name)).toEqual(["MRR", "Active subscriptions", "Paywall unique viewers", "Conversions"]);
    for (const p of panels) {
      expect(p.value, p.name).toBeGreaterThan(0);
      expect(p.series, p.name).toHaveLength(30);
    }
    const table = await t.exec("dashboard", "summary", "--app", "app_demo", "--days", "90", "--output", "table");
    expect(table.code).toBe(0);
    expect(table.out).toMatch(/^app_demo .*\(WEEK\)$/m);
    expect(table.out).toMatch(/^MRR\s+[\d,]+\s+[▁▂▃▄▅▆▇█]+$/m);
    expect(table.out).not.toContain("(no data)");
    expect(t.calls).toHaveLength(0);
  });

  it("an unknown read is a 404 that says demo mode has no data for it", async () => {
    const t = demo();
    const other = await t.exec("audiences", "list", "--app", "app_demo");
    expect(other.code).toBe(6);
    expect(other.errJson().error.messages.join(" ")).toContain("Demo mode (PURCHASELY_DEMO=1) has no sample data for GET /management/v1/applications/{application_id}/audiences");
    const app = await t.exec("placements", "list", "--app", "app_real");
    expect(app.code).toBe(6);
    expect(app.errJson().error.messages.join(" ")).toContain("The sample app is app_demo");
    expect(t.calls).toHaveLength(0);
  });

  it("refuses a write with exit 10 and sends nothing; --dry-run and --plan still work", async () => {
    const t = demo();
    const w = await t.exec("placements", "update", "--app", "app_demo", "--id", "plc_demo_home", "--data", '{"name":"x"}', "--env", "staging");
    expect(w.code).toBe(10);
    expect(w.errJson().error.messages[0]).toContain("Demo mode (PURCHASELY_DEMO=1) never sends a write");
    const dry = await t.exec("placements", "update", "--app", "app_demo", "--id", "plc_demo_home", "--data", '{"name":"x"}', "--env", "staging", "--dry-run");
    expect(dry.code).toBe(0);
    expect(dry.json()).toMatchObject({ method: "PATCH", body: { name: "x" } });
    const plan = join(t.root, "plan.json");
    const p = await t.exec("placements", "update", "--app", "app_demo", "--id", "plc_demo_home", "--data", '{"name":"x"}', "--env", "staging", "--plan", plan);
    expect(p.code).toBe(0);
    const a = await t.exec("apply", plan, "--yes");
    expect(a.code).toBe(10);
    expect(t.calls).toHaveLength(0);
  });

  it("starts no update check", () => {
    expect(autoUpdateOff({ PURCHASELY_DEMO: "1" }, {})).toBe(true);
  });

  it("doctor reports demo mode instead of a missing key", async () => {
    const t = demo();
    const r = await t.exec("doctor");
    const checks = r.json().checks as { check: string; status: string; detail: string }[];
    expect(checks.find((c) => c.check === "key")).toMatchObject({ status: "ok", detail: expect.stringContaining("Demo mode") });
    expect(checks.find((c) => c.check === "api")).toMatchObject({ status: "ok" });
    expect(checks.find((c) => c.check === "writes")?.detail).toContain("Demo mode");
    expect(t.calls).toHaveLength(0);
  });
});

describe("purchasely demo", () => {
  it("prints how to start, as JSON, with no key and no header line", async () => {
    const t = setup({ env: { PURCHASELY_API_KEY: "" } });
    const r = await t.exec("demo");
    expect(r.code).toBe(0);
    expect(r.err).toBe("");
    const out = r.json();
    expect(out.start).toBe("export PURCHASELY_DEMO=1");
    expect(out.app).toBe("app_demo");
    expect(out.try).toHaveLength(3);
    expect(out.try[2]).toBe("purchasely dashboard summary --app app_demo --output table");
    expect(t.calls).toHaveLength(0);
  });

  it("is the first search result for a test with no key", async () => {
    const r = await setup().exec("search", "try the cli with sample data, no key");
    expect(r.json()[0].command).toBe("purchasely demo");
  });
});
