import { describe, expect, it } from "vitest";
import { KEY, setup, type Reply } from "./helpers.js";

const err = (status: number, code: string, extra: Record<string, unknown> = {}, headers?: Record<string, string>): Reply => ({
  status, body: { error: { code, messages: [`${code} message`] }, ...extra }, headers,
});

describe("error mapping", () => {
  const cases: [string, Reply, number][] = [
    ["UNAUTHORIZED", err(401, "UNAUTHORIZED"), 3],
    ["401 without code", { status: 401, text: "nope" }, 3],
    ["FORBIDDEN", err(403, "FORBIDDEN"), 4],
    ["FEATURE_NOT_ENABLED", err(403, "FEATURE_NOT_ENABLED"), 4],
    ["VALIDATION_FAILED", err(422, "VALIDATION_FAILED"), 5],
    ["other 422", { status: 422, body: {} }, 5],
    ["NOT_FOUND", err(404, "NOT_FOUND"), 6],
    ["404 without code", { status: 404, text: "<html>" }, 6],
    ["500", { status: 500, text: "boom" }, 1],
    ["unknown code", err(400, "WHATEVER"), 1],
  ];
  for (const [name, reply, code] of cases) {
    it(`${name} exits ${code}`, async () => {
      const t = setup({ replies: [reply] });
      const r = await t.exec("placements", "list", "--app", "app_1");
      expect(r.code).toBe(code);
      const e = r.errJson();
      expect(typeof e.error.code).toBe("string");
      expect(Array.isArray(e.error.messages)).toBe(true);
      expect(e.error.status).toBe(reply.status);
      expect(r.out).toBe("");
    });
  }

  it("RATE_LIMITED exits 7 with retry_at, after one retry", async () => {
    const t = setup({ replies: [err(429, "RATE_LIMITED", {}, { "Retry-After": "2", "X-RateLimit-Reset": "1700000100" })] });
    const r = await t.exec("placements", "list", "--app", "app_1");
    expect(r.code).toBe(7);
    expect(r.errJson().error).toMatchObject({ code: "RATE_LIMITED", retry_at: 1700000100, status: 429 });
    expect(t.calls).toHaveLength(2);
  });

  it("CONFIRMATION_REQUIRED arriving as 403 exits 8", async () => {
    const t = setup({ replies: [err(403, "CONFIRMATION_REQUIRED", { confirm_token: "tok" })] });
    const r = await t.exec("placements", "delete", "--env", "prod", "--app", "app_1", "--id", "p");
    expect(r.code).toBe(8);
  });

  it("exits 1 on a network error and never prints the key", async () => {
    const t = setup({ replies: [{ throws: Object.assign(new TypeError(`fetch failed: Authorization: Bearer ${KEY}`), { cause: { code: "ENOTFOUND" } }) }] });
    const r = await t.exec("placements", "list", "--app", "app_1");
    expect(r.code).toBe(1);
    expect(r.errJson().error.code).toBe("NETWORK_ERROR");
    expect(r.err).toContain("ENOTFOUND");
    expect(r.out + r.err).not.toContain("SECRET123");
  });

  it("exits 2 on an unknown option and an unknown command", async () => {
    const t = setup();
    expect((await t.exec("placements", "list", "--nope")).code).toBe(2);
    expect((await t.exec("nothing")).code).toBe(2);
    expect((await t.exec("placements", "list", "--output", "xml")).code).toBe(2);
  });

  it("prints help and version with exit 0", async () => {
    const t = setup();
    expect((await t.exec("--version")).out.trim()).toBe("9.9.9");
    const h = await t.exec("placements", "delete", "--help");
    expect(h.code).toBe(0);
    expect(h.out).toContain("Required scope: targeting.placements:delete");
    expect(h.out).toContain("DELETE /management/v1/applications/{application_id}/placements/{id}");
    expect(h.out).toContain("Confirmation flow");
    expect(h.out).toContain("purchasely placements delete --app app_xxx --id <id> --env prod");
    expect(h.err).toBe("");
    const body = await t.exec("placements", "create", "--help");
    expect(body.out).toContain("Body (required)");
  });

  it("has no em-dash or en-dash in the help of any command", async () => {
    const t = setup();
    const { managementCommands } = await import("../src/surfaces/management/index.js");
    for (const c of managementCommands()) {
      const h = await t.exec(c.resource, c.action, "--help");
      expect(h.code).toBe(0);
      expect(h.out, `${c.resource} ${c.action}`).not.toMatch(/[\u2013\u2014]/);
    }
    expect((await t.exec("--help")).out).not.toMatch(/[\u2013\u2014]/);
  });
});
