import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { limits } from "../src/core/http.js";
import { KEY, setup, type Reply } from "./helpers.js";

const tooMany = (headers: Record<string, string> = {}): Reply => ({ status: 429, body: { error: { code: "RATE_LIMITED", messages: ["slow down"] } }, headers });
const list = ["placements", "list", "--app", "app_1"];

describe("breaker", () => {
  it("waits for Retry-After, retries once, halves the rate", async () => {
    const t = setup({ replies: [tooMany({ "Retry-After": "30" }), { body: { placements: [] } }] });
    const r = await t.exec(...list);
    expect(r.code).toBe(0);
    expect(t.calls).toHaveLength(2);
    expect(t.calls[1].at - t.calls[0].at).toBeGreaterThanOrEqual(30_000);
    const b = limits(t.d, "prod", "management", KEY).breaker;
    expect(b.state().rate_factor).toBe(0.5);
  });

  it("falls back to X-RateLimit-Reset, then to 60 s, never 1 s", async () => {
    const reset = setup({ replies: [tooMany({ "X-RateLimit-Reset": String(1_700_000_000 + 45) }), { body: {} }] });
    await reset.exec(...list);
    expect(reset.calls[1].at - reset.calls[0].at).toBeGreaterThanOrEqual(44_000);
    const none = setup({ replies: [tooMany(), { body: {} }] });
    await none.exec(...list);
    expect(none.calls[1].at - none.calls[0].at).toBeGreaterThanOrEqual(60_000);
  });

  it("retries only once: two 429 exit 7", async () => {
    const t = setup({ replies: [tooMany({ "Retry-After": "1" })] });
    expect((await t.exec(...list)).code).toBe(7);
    expect(t.calls).toHaveLength(2);
  });

  it("halves down to 1/8 at most, and restores after 10 minutes without a 429", async () => {
    const t = setup({ replies: [tooMany({ "Retry-After": "1" })] });
    const b = () => limits(t.d, "prod", "management", KEY).breaker;
    for (let i = 0; i < 4; i++) await t.exec(...list);
    expect(b().state().rate_factor).toBe(0.125);
    await t.d.sleep(10 * 60_000 + 1);
    expect(b().state().rate_factor).toBe(1);
  });

  it("every process waits for paused_until", async () => {
    const t = setup({ replies: [{ body: {} }] });
    const { breaker } = limits(t.d, "prod", "management", KEY);
    await breaker.record(20_000);
    const before = t.tick();
    await t.exec(...list);
    expect(t.calls[0].at - before).toBeGreaterThanOrEqual(20_000);
  });

  it("stops with exit 7 after 20 rejections in 5 minutes, then refuses at once", async () => {
    const t = setup({ replies: [tooMany({ "Retry-After": "1" })] });
    for (let i = 0; i < 10; i++) await t.exec(...list);
    expect(t.calls).toHaveLength(20);
    const r = await t.exec(...list);
    expect(r.code).toBe(7);
    expect(r.errJson().error.code).toBe("RATE_LIMIT_STOP");
    expect(r.err).toContain("Wait 5 minutes");
    expect(t.calls).toHaveLength(20);
  });
});

describe("rate options", () => {
  it("refuses a rate above the cap unless --i-asked-backend", async () => {
    const t = setup();
    expect((await t.exec(...list, "--rate", "5")).code).toBe(2);
    expect(t.calls).toHaveLength(0);
    expect((await t.exec(...list, "--rate", "5", "--i-asked-backend")).code).toBe(0);
    expect((await t.exec(...list, "--rate", "0.1")).code).toBe(0);
    expect((await t.exec(...list, "--rate", "0")).code).toBe(2);
  });

  it("spaces sequential reads by 1/0.8 s", async () => {
    const t = setup();
    await t.exec(...list);
    await t.exec(...list);
    expect(t.calls[1].at - t.calls[0].at).toBeGreaterThanOrEqual(1250);
  });

  it("spaces writes by 7 s", async () => {
    const t = setup();
    const w = ["placements", "delete", "--env", "prod", "--app", "app_1", "--id", "p"];
    await t.exec(...w);
    await t.exec(...w);
    expect(t.calls[1].at - t.calls[0].at).toBeGreaterThanOrEqual(7000);
  });
});

describe("usage log and command", () => {
  it("logs one line per request without the key or query values", async () => {
    const t = setup({ replies: [tooMany({ "Retry-After": "1" }), { body: {} }] });
    await t.exec(...list, "--fields", "secretfield");
    const raw = readFileSync(join(t.d.stateDir, "usage.jsonl"), "utf8");
    const lines = raw.trim().split("\n").map((l) => JSON.parse(l));
    expect(lines).toHaveLength(2);
    expect(lines[0]).toMatchObject({ env: "prod", surface: "management", method: "GET", path: "/management/v1/applications/{application_id}/placements", status: 429, rate_limited: true });
    expect(lines[0].request_id).toMatch(/^[0-9a-f-]{36}$/);
    expect(lines[1].status).toBe(200);
    expect(raw).not.toContain("SECRET123");
    expect(raw).not.toContain("secretfield");
    expect(raw).not.toContain("app_1");
  });

  it("purchasely usage shows the last hour, rejections, limiter and breaker", async () => {
    const t = setup({ replies: [tooMany({ "Retry-After": "1" }), { body: {} }] });
    await t.exec(...list);
    const u = (await t.exec("usage")).json();
    expect(u.last_hour).toHaveLength(1);
    expect(u.last_hour[0]).toMatchObject({ env: "prod", surface: "management", requests: 2, rejections: 1 });
    expect(u.current.breaker).toMatchObject({ rate_factor: 0.5, rejections_last_5_min: 1 });
    expect(u.current.in_flight).toBe(0);
    await t.d.sleep(25 * 3600_000);
    expect((await t.exec("usage")).json().last_hour).toEqual([]);
  });
});

describe("large output and purge", () => {
  const big = { placements: [{ blob: "x".repeat(250 * 1024) }] };

  it("writes a JSON over 200 KB to the cache dir with 0700 dirs", async () => {
    const t = setup({ replies: [{ body: big }] });
    const r = await t.exec(...list);
    const { file, bytes } = r.json();
    expect(bytes).toBeGreaterThan(200 * 1024);
    expect(file.startsWith(join(t.d.cacheDir, "prod"))).toBe(true);
    expect(JSON.parse(readFileSync(file, "utf8")).placements[0].blob).toHaveLength(250 * 1024);
    expect(statSync(join(file, "..")).mode & 0o777).toBe(0o700);
    expect(statSync(file).mode & 0o777).toBe(0o600);
    expect(file).not.toContain("SECRET123");
  });

  it("honours --out file and --out -", async () => {
    const t = setup({ replies: [{ body: big }] });
    const f = join(t.root, "out.json");
    expect((await t.exec(...list, "--out", f)).json()).toMatchObject({ file: f });
    expect(existsSync(f)).toBe(true);
    expect((await t.exec(...list, "--out", "-")).out.length).toBeGreaterThan(250 * 1024);
    const small = setup({ replies: [{ body: { a: 1 } }] });
    const g = join(small.root, "small.json");
    await small.exec(...list, "--out", g);
    expect(JSON.parse(readFileSync(g, "utf8"))).toEqual({ a: 1 });
  });

  it("purge deletes the cache, for one env or all", async () => {
    const t = setup({ replies: [{ body: big }] });
    await t.exec(...list);
    await t.exec(...list, "--env", "staging");
    expect(readdirSync(t.d.cacheDir).sort()).toEqual(["prod", "staging"]);
    await t.exec("purge", "--env", "staging");
    expect(readdirSync(t.d.cacheDir)).toEqual(["prod"]);
    await t.exec("purge");
    expect(existsSync(t.d.cacheDir)).toBe(false);
  });
});
