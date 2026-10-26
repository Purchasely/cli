import { describe, expect, it } from "vitest";
import { writeConfig } from "../src/core/config.js";
import { KEY, setup } from "./helpers.js";

type Row = { check: string; status: string; detail: string };
const byName = (rows: Row[]) => Object.fromEntries(rows.map((r) => [r.check, r]));

describe("doctor", () => {
  it("checks the version, the key and the API, and never prints the key", async () => {
    const t = setup({ replies: (c) => (c.url.endsWith("/latest.json") ? { body: { version: "9.9.9" } } : { body: { data: [] } }) });
    const r = await t.exec("doctor");
    expect(r.code).toBe(0);
    const rows = byName(r.json());
    expect(rows.version).toMatchObject({ status: "ok", detail: "9.9.9, the latest" });
    expect(rows.key).toMatchObject({ status: "ok", detail: "prod key …T123 from env" });
    expect(rows.api.status).toBe("ok");
    expect(rows["ai plugin"].status).toBe("warn");
    expect(rows.writes.detail).toBe("Writes allowed");
    expect(t.calls.map((c) => c.url)).toEqual(["https://cli.test/latest.json", "https://api.purchasely.io/management/v1/applications?per_page=1"]);
    expect(r.out + r.err).not.toContain(KEY);
  });

  it("fails when the API refuses the key, warns on a newer version", async () => {
    const t = setup({ replies: (c) => (c.url.endsWith("/latest.json") ? { body: { version: "10.0.0" } } : { status: 401, body: {} }) });
    const r = await t.exec("doctor");
    expect(r.code).toBe(1);
    const rows = byName(r.json());
    expect(rows.api).toMatchObject({ status: "fail" });
    expect(rows.version.detail).toContain("10.0.0 is available");
    expect(r.errJson().error.code).toBe("DOCTOR_FAILED");
  });

  it("with no key and offline, it sends nothing and names the fix", async () => {
    const t = setup({ env: { PURCHASELY_API_KEY: "", PURCHASELY_OFFLINE: "1", PURCHASELY_READ_ONLY: "1" } });
    writeConfig(t.d.configDir, { write_allowlist: { prod: ["app_test"] } });
    const r = await t.exec("doctor", "--output", "table");
    expect(r.code).toBe(1);
    expect(t.calls).toHaveLength(0);
    expect(r.out).toContain("auth login");
    expect(r.out).toContain("Read-only mode, write_allowlist.prod: app_test");
  });
});
