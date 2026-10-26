import { describe, expect, it } from "vitest";
import { setup } from "./helpers.js";

describe("--dry-run", () => {
  it("prints the request, redacts the key and does not call fetch", async () => {
    const t = setup();
    const r = await t.exec("placements", "update", "--env", "prod", "--app", "app_1", "--id", "plc_1", "--data", '{"name":"X"}', "--dry-run");
    expect(r.code).toBe(0);
    expect(t.calls).toHaveLength(0);
    const j = r.json();
    expect(j.method).toBe("PATCH");
    expect(j.url).toBe("https://api.purchasely.io/management/v1/applications/app_1/placements/plc_1");
    expect(j.headers.Authorization).toBe("Bearer <redacted>");
    expect(j.body).toEqual({ name: "X" });
    expect(r.out + r.err).not.toContain("SECRET123");
  });

  it("works without an API key", async () => {
    const t = setup({ env: { PURCHASELY_API_KEY: "" } });
    const r = await t.exec("placements", "delete", "--env", "staging", "--app", "app_1", "--id", "plc_1", "--dry-run");
    expect(r.code).toBe(0);
    expect(r.json().url.startsWith("https://api-staging.purchasely.io/")).toBe(true);
  });
});
