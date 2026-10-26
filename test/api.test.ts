import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { EXT, EXTRA_TOKEN, KEY, setup, type Reply } from "./helpers.js";

const need = (token = "tok_1"): Reply => ({ status: 422, body: { error: { code: "CONFIRMATION_REQUIRED", messages: ["Used by 3 live campaigns"] }, confirm_token: token } });
const ok: Reply = { status: 200, body: { deleted: true } };
const del = ["api", "DELETE", "/applications/app_1/placements/plc_1", "--env", "prod"];
const put = ["api", "PUT", "/applications/app_1/webhooks", "--env", "prod", "--data", '{"events":{"a":{"enabled":true}}}'];
const files = (dir: string): string[] => readdirSync(dir).flatMap((f) => (statSync(join(dir, f)).isDirectory() ? files(join(dir, f)) : [join(dir, f)]));

describe("api", () => {
  it("sends a GET under /management/v1 with the query and the Management key", async () => {
    for (const path of ["/applications/app_1/placements", "/management/v1/applications/app_1/placements"]) {
      const t = setup({ replies: [{ body: { placements: [] } }] });
      const r = await t.exec("api", "get", path, "--query", "limit=5", "--query", "fields=id,name");
      expect(r.code, r.err).toBe(0);
      expect(r.json()).toEqual({ placements: [] });
      expect(t.calls[0].url).toBe("https://api.purchasely.io/management/v1/applications/app_1/placements?limit=5&fields=id%2Cname");
      expect(t.calls[0].init.method).toBe("GET");
      expect(t.calls[0].init.headers.Authorization).toBe(`Bearer ${KEY}`);
      expect(t.calls[0].init.redirect).toBe("manual");
    }
  });

  it("refuses a write without --env, also with --dry-run, and sends nothing", async () => {
    for (const extra of [[], ["--dry-run"]]) {
      const t = setup();
      const r = await t.exec("api", "DELETE", "/applications/app_1/placements/plc_1", ...extra);
      expect(r.code).toBe(10);
      expect(t.calls).toHaveLength(0);
    }
  });

  it("--dry-run masks the key and sends nothing", async () => {
    const t = setup();
    const r = await t.exec("api", "POST", "/applications/app_1/placements", "--env", "staging", "--data", '{"name":"x"}', "--dry-run");
    expect(r.code, r.err).toBe(0);
    expect(r.json()).toMatchObject({
      method: "POST", url: "https://api-staging.purchasely.io/management/v1/applications/app_1/placements",
      headers: { Authorization: "Bearer <redacted>" }, body: { name: "x" },
    });
    expect(t.calls).toHaveLength(0);
  });

  it("refuses a bad method and every path that could leave /management/v1", async () => {
    const t = setup();
    expect((await t.exec("api", "TRACE", "/applications")).code).toBe(2);
    const bad = [
      "https://evil.test/x", "//evil.test/management/v1/x", "applications", "/applications/../admin", "/applications/%2e%2e/admin",
      "/applications/%2E%2E", "/applications/a%2Fb", "/applications/a%5Cb", "/applications\\x", "/applications?x=1", "/applications#x",
      "/applications//x", "/applications/", "/applications/%252e%252e", "/applications/%zz", "/./applications",
      "/applications/app_1/x/.\t./.\t./app_2/placements", "/applications/.\n./x", "/applications/a b", "/applications/a\x7fb", "/applications/a\rb",
    ];
    for (const p of bad) {
      const r = await t.exec("api", "GET", p);
      expect(r.code, p).toBe(2);
      expect(r.errJson().error.messages[0], p).toContain("The path must be under /management/v1");
    }
    expect((await t.exec("api", "GET", "/applications", "--query", "novalue")).code).toBe(2);
    expect((await t.exec("api", "GET", "/applications", "--data", "{}")).code).toBe(2);
    expect(t.calls).toHaveLength(0);
  });

  it("refuses a redirect", async () => {
    const t = setup({ replies: [{ status: 302, headers: { Location: "https://evil.test/" } }] });
    const r = await t.exec("api", "GET", "/applications");
    expect(r.code).toBe(10);
    expect(r.errJson().error.code).toBe("REFUSED");
    expect(t.calls).toHaveLength(1);
  });

  it("a 429 goes through the breaker: one retry, then exit 7", async () => {
    const t = setup({ replies: [{ status: 429, body: { error: { code: "RATE_LIMITED", messages: ["slow"] } }, headers: { "Retry-After": "2" } }] });
    expect((await t.exec("api", "GET", "/applications")).code).toBe(7);
    expect(t.calls).toHaveLength(2);
  });

  it("uses the Management key in an extension too", async () => {
    const t = setup({ name: "acme", ext: EXT, env: { ACME_EXTRA_TOKEN: EXTRA_TOKEN } });
    const r = await t.exec("api", "GET", "/applications");
    expect(r.code, r.err).toBe(0);
    expect(t.calls[0].init.headers.Authorization).toBe(`Bearer ${KEY}`);
    expect(r.err.split("\n")[0]).toContain("surface=management");
  });

  it("confirm flow: the token goes in the query for DELETE placements, in the body for PUT webhooks", async () => {
    const q = setup({ replies: [need(), ok] });
    expect((await q.exec(...del, "--yes")).code).toBe(0);
    expect(q.calls[1].url).toBe(`${q.calls[0].url}?confirm_token=tok_1`);

    const b = setup({ replies: [need(), ok] });
    expect((await b.exec(...put, "--yes")).code).toBe(0);
    expect(b.calls[1].url).toBe(b.calls[0].url);
    expect(JSON.parse(b.calls[1].init.body as string)).toEqual({ events: { a: { enabled: true } }, confirm_token: "tok_1" });

    const given = setup({ replies: [ok] });
    expect((await given.exec(...del, "--confirm-token", "tok_9")).code).toBe(0);
    expect(new URL(given.calls[0].url).searchParams.get("confirm_token")).toBe("tok_9");
  });

  it("confirm flow: declined on a TTY or not interactive exits 8 after one call", async () => {
    const no = setup({ replies: [need(), ok], isTTY: true, answers: ["n"] });
    expect((await no.exec(...del)).code).toBe(8);
    expect(no.calls).toHaveLength(1);

    const batch = setup({ replies: [need("tok_9"), ok] });
    const r = await batch.exec(...del);
    expect(r.code).toBe(8);
    expect(r.errJson().confirm_token).toBe("tok_9");
    expect(batch.calls).toHaveLength(1);
  });

  it("a path outside the spec gets no confirm retry", async () => {
    const t = setup({ replies: [need(), ok] });
    expect((await t.exec("api", "POST", "/applications/app_1/nothing_here", "--env", "prod", "--yes")).code).toBe(8);
    expect(t.calls).toHaveLength(1);
  });

  it("a path outside the spec is logged with {id} for each segment that is not a literal segment of the spec", async () => {
    const t = setup();
    expect((await t.exec("api", "GET", "/applications/app_1/nothing_here/12/export-all")).code).toBe(0);
    const line = JSON.parse(readFileSync(join(t.d.stateDir, "usage.jsonl"), "utf8").trim());
    expect(line.path).toBe("/management/v1/applications/{id}/{id}/{id}/{id}");
  });

  it("an email or a vendor id with no digit and no underscore never reaches the usage log", async () => {
    for (const p of ["/applications/app_1/subscribers/jane.doe@example.com", "/applications/app_1/placements/onboarding/premium"]) {
      const t = setup();
      expect((await t.exec("api", "GET", p)).code).toBe(0);
      const log = readFileSync(join(t.d.stateDir, "usage.jsonl"), "utf8");
      expect(log).not.toContain("jane");
      expect(log).not.toContain("onboarding");
      expect(log).not.toContain("premium");
    }
  });

  it("--confirm-token on a path outside the spec is a usage error that names the other ways", async () => {
    const t = setup({ replies: [ok] });
    const r = await t.exec("api", "DELETE", "/applications/app_1/new_thing/x", "--env", "prod", "--confirm-token", "tok_1");
    expect(r.code).toBe(2);
    expect(r.errJson().error.messages[0]).toContain("--query confirm_token=<token>");
    expect(t.calls).toHaveLength(0);
  });

  it("writes no full key to stdout, stderr or files, and logs the path template", async () => {
    const t = setup({ replies: [{ body: { placement: { id: "plc_1" } } }] });
    const a = await t.exec("api", "GET", "/applications/app_1/placements/plc_1");
    const b = await t.exec("api", "DELETE", "/applications/app_1/placements/plc_1", "--env", "prod", "--dry-run");
    expect(a.code + b.code).toBe(0);
    const text = [a.out, a.err, b.out, b.err, ...files(t.root).map((f) => readFileSync(f, "utf8"))].join("\n");
    expect(text).not.toContain("SECRET123");
    expect(text).toContain("/management/v1/applications/{application_id}/placements/{id}");
  });
});
