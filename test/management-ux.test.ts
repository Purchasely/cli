import { describe, expect, it } from "vitest";
import { setup } from "./helpers.js";

const flat = (s: string) => s.replace(/\s+/g, " ");
const query = (url: string) => decodeURIComponent(new URL(url).search);

describe("B1 array values", () => {
  it("--date-range takes a JSON array or the comma form", async () => {
    for (const v of ['["2026-01-01","2026-10-01"]', "2026-01-01,2026-10-01"]) {
      const t = setup();
      const r = await t.exec("dashboard", "conversion", "--app", "app_1", "--date-range", v);
      expect(r.code, r.err).toBe(0);
      expect(query(t.calls[0].url)).toBe("?date_range[]=2026-01-01&date_range[]=2026-10-01");
    }
  });

  it("--date-range takes ISO date-times", async () => {
    const t = setup();
    const r = await t.exec("dashboard", "conversion", "--app", "app_1", "--date-range", "2026-01-01T00:00:00Z,2026-10-01T12:30:00+02:00");
    expect(r.code, r.err).toBe(0);
    expect(query(t.calls[0].url)).toBe("?date_range[]=2026-01-01T00:00:00Z&date_range[]=2026-10-01T12:30:00+02:00");
  });

  it("a bad --date-range is a usage error that shows the form, with no call", async () => {
    for (const v of ["2026-01-01", "a,b", "2026-01-01,2026-10-01,2026-11-01", '["2026-01-01"', "01/01/2026,2026-10-01"]) {
      const t = setup();
      const r = await t.exec("dashboard", "conversion", "--app", "app_1", "--date-range", v);
      expect(r.code, v).toBe(2);
      expect(r.errJson().error.messages.join(" "), v).toContain("--date-range 2026-01-01,2026-10-01");
      expect(t.calls, v).toHaveLength(0);
    }
  });

  it("any array option takes a JSON array of scalars, and refuses other items", async () => {
    const t = setup();
    expect((await t.exec("placements", "list", "--app", "app_1", "--fields", '["id","name"]')).code).toBe(0);
    expect(query(t.calls[0].url)).toBe("?fields[]=id&fields[]=name");
    const bad = await t.exec("placements", "list", "--app", "app_1", "--fields", '["id",{"a":1}]');
    expect(bad.code).toBe(2);
    expect(t.calls).toHaveLength(1);
  });
});

describe("B2 dashboard date range", () => {
  it("is required where the spec declares it", async () => {
    const t = setup();
    const r = await t.exec("dashboard", "conversion", "--app", "app_1");
    expect(r.code).toBe(2);
    expect(r.errJson().error.messages).toEqual(["--date-range is required."]);
    expect(t.calls).toHaveLength(0);
    const h = await t.exec("dashboard", "conversion", "--help");
    expect(h.out).toContain("purchasely dashboard conversion --app app_xxx --date-range <date-range>");
    expect(flat(h.out)).toMatch(/--date-range <a,b> .*?\[array of string, required\]/);
  });

  it("is not added where the spec has no date range", async () => {
    for (const action of ["mau-reports", "paywall-latency"]) {
      const t = setup();
      expect((await t.exec("dashboard", action, "--app", "app_1")).code, action).toBe(0);
      expect(t.calls).toHaveLength(1);
    }
  });
});

describe("B3 array option help", () => {
  it("shows the comma form", async () => {
    const h = await setup().exec("placements", "list", "--help");
    expect(h.out).toContain("--fields <a,b>");
    expect(flat(h.out)).toContain("comma separated or repeated");
  });
});

describe("B4 examples hold the required flags", () => {
  it("plans list", async () => {
    const t = setup();
    expect((await t.exec("plans", "list", "--help")).out).toContain("purchasely plans list --app app_xxx --product-id <product-id>");
    const hits = (await t.exec("search", "list plans", "--limit", "50")).json() as { command: string }[];
    expect(hits.map((h) => h.command)).toContain("purchasely plans list --app app_xxx --product-id <product-id>");
  });
});

describe("B5 table output", () => {
  const row = (i: number) => ({
    created_at: "2026-01-01", extra: { a: 1 }, type: "default", status: "live", name: `Home ${i}`, vendor_id: `home_${i}`, id: `plc_${i}`,
    updated_at: "2026-02-01", description: "x".repeat(300),
  });
  const table = async (...extra: string[]) => {
    const was = process.stdout.columns;
    process.stdout.columns = 100;
    try {
      const t = setup({ replies: [{ body: { placements: [row(1), row(2)], meta: { total: 2 } } }] });
      return await t.exec("placements", "list", "--app", "app_1", "--output", "table", ...extra);
    } finally {
      process.stdout.columns = was;
    }
  };

  it("shows six scalar columns, id first, by default", async () => {
    const r = await table();
    const lines = r.out.trimEnd().split("\n");
    expect(lines[0].split(/\s{2,}/)).toEqual(["id", "vendor_id", "name", "status", "type", "created_at"]);
    expect(lines[1]).toMatch(/^plc_1\s+home_1\s+Home 1\s+live\s+default\s+2026-01-01$/);
  });

  it("keeps every --fields column in the given order and fits the width", async () => {
    const fields = "description,updated_at,id,name,vendor_id,status,type";
    const r = await table("--fields", fields);
    const lines = r.out.trimEnd().split("\n");
    expect(lines[0].split(/\s{2,}/)).toEqual(fields.split(","));
    for (const l of lines) expect(l.length).toBeLessThanOrEqual(100);
    expect(lines[1]).toContain("...");
    expect(lines[1]).toContain("plc_1");
  });
});

describe("B6 not found names the resource", () => {
  const notFound = { status: 404, body: { error: { code: "NOT_FOUND", messages: ["Not found"] } } };

  it("names the id and the app before the server message", async () => {
    for (const [reply, server] of [[notFound, "Not found"], [{ status: 404, text: "<html>" }, "HTTP 404"]] as const) {
      const t = setup({ replies: [reply] });
      const r = await t.exec("placements", "get", "--app", "app_1", "--id", "plc_9");
      expect(r.code).toBe(6);
      expect(r.errJson().error.messages).toEqual(['placement "plc_9" not found in app app_1', server]);
    }
  });

  it("adds nothing when the app is the only path id", async () => {
    const t = setup({ replies: [notFound] });
    const r = await t.exec("placements", "list", "--app", "app_1");
    expect(r.code).toBe(6);
    expect(r.errJson().error.messages).toEqual(["Not found"]);
  });
});
