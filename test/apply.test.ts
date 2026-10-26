import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { writeConfig } from "../src/core/config.js";
import { setup, type Reply } from "./helpers.js";

const need: Reply = { status: 422, body: { error: { code: "CONFIRMATION_REQUIRED", messages: ["Used by 3 live campaigns"] }, confirm_token: "tok_1" } };
const update = (plan: string, id: string) => ["placements", "update", "--app", "app_1", "--id", id, "--data", `{"name":"${id}"}`, "--env", "prod", "--plan", plan];
const read = (f: string) => JSON.parse(readFileSync(f, "utf8"));

describe("plan, then apply", () => {
  it("--plan records the write and sends nothing, also in read-only mode with no key", async () => {
    const t = setup({ env: { PURCHASELY_API_KEY: "", PURCHASELY_READ_ONLY: "1" } });
    const plan = join(t.root, "plan.json");
    const r = await t.exec(...update(plan, "plc_1"));
    expect(r.code).toBe(0);
    expect(r.json()).toMatchObject({ planned: plan, number: 1, method: "PATCH", body: { name: "plc_1" } });
    await t.exec(...update(plan, "plc_2"));
    expect(t.calls).toHaveLength(0);
    const saved = read(plan);
    expect(saved.version).toBe(1);
    expect(saved.requests.map((x: { url: string }) => x.url)).toEqual([
      "https://api.purchasely.io/management/v1/applications/app_1/placements/plc_1",
      "https://api.purchasely.io/management/v1/applications/app_1/placements/plc_2",
    ]);
    expect(readFileSync(plan, "utf8")).not.toContain("Bearer");
  });

  it("--plan needs --env, keeps the allow-list, and refuses --dry-run and a GET", async () => {
    const t = setup();
    const plan = join(t.root, "plan.json");
    expect((await t.exec("placements", "update", "--app", "app_1", "--id", "plc_1", "--data", "{}", "--plan", plan)).code).toBe(10);
    expect((await t.exec(...update(plan, "plc_1"), "--dry-run")).code).toBe(2);
    expect((await t.exec("api", "GET", "/applications/app_1/placements", "--plan", plan)).code).toBe(2);
    writeConfig(t.d.configDir, { write_allowlist: { prod: ["app_test"] } });
    expect((await t.exec(...update(plan, "plc_1"))).code).toBe(10);
    expect(t.calls).toHaveLength(0);
  });

  it("apply lists the writes, asks once, sends them, and marks each one", async () => {
    const t = setup({ isTTY: true, answers: ["y"], replies: [{ body: { id: "plc_1" } }, { body: { id: "plc_2" } }] });
    const plan = join(t.root, "plan.json");
    await t.exec(...update(plan, "plc_1"));
    await t.exec(...update(plan, "plc_2"));
    const r = await t.exec("apply", plan);
    expect(r.code).toBe(0);
    expect(t.prompts).toEqual(["Apply 2 writes? [y/N] "]);
    expect(r.err).toContain("#1 prod PATCH https://api.purchasely.io/management/v1/applications/app_1/placements/plc_1");
    expect(r.json().applied.map((a: { number: number }) => a.number)).toEqual([1, 2]);
    expect(t.calls.map((c) => [c.init.method, c.init.body])).toEqual([["PATCH", '{"name":"plc_1"}'], ["PATCH", '{"name":"plc_2"}']]);
    expect(read(plan).requests.every((x: { applied_at?: string }) => x.applied_at)).toBe(true);
    // A second run has nothing left to send.
    expect((await t.exec("apply", plan)).json()).toEqual({ pending: [] });
    expect(t.calls).toHaveLength(2);
  });

  it("apply without a terminal needs --yes, and --dry-run only lists", async () => {
    const t = setup();
    const plan = join(t.root, "plan.json");
    await t.exec(...update(plan, "plc_1"));
    expect((await t.exec("apply", plan)).code).toBe(8);
    expect((await t.exec("apply", plan, "--dry-run")).json().pending).toHaveLength(1);
    expect(t.calls).toHaveLength(0);
    expect((await t.exec("apply", plan, "--yes")).code).toBe(0);
    expect(t.calls).toHaveLength(1);
  });

  it("apply stops at the first error with exit 9, and a second run sends only the rest", async () => {
    const t = setup({ replies: [{ body: {} }, { status: 404, body: { error: { code: "NOT_FOUND", messages: ["no"] } } }, { body: {} }] });
    const plan = join(t.root, "plan.json");
    for (const id of ["plc_1", "plc_2", "plc_3"]) await t.exec(...update(plan, id));
    const r = await t.exec("apply", plan, "--yes");
    expect(r.code).toBe(9);
    expect(r.json()).toMatchObject({ applied: [{ number: 1 }], failed: { number: 2 } });
    expect(r.errJson().error.messages[0]).toContain("Request #2 of the plan failed. 1 applied, 2 left.");
    expect((await t.exec("apply", plan, "--yes")).code).toBe(0);
    expect(t.calls.map((c) => c.url.split("/").pop())).toEqual(["plc_1", "plc_2", "plc_2", "plc_3"]);
  });

  it("--yes approves the plan, not a risky change", async () => {
    const t = setup({ replies: [need] });
    const plan = join(t.root, "plan.json");
    await t.exec("placements", "delete", "--app", "app_1", "--id", "plc_1", "--env", "prod", "--plan", plan);
    const r = await t.exec("apply", plan, "--yes");
    expect(r.code).toBe(8);
    expect(t.calls).toHaveLength(1);
    expect(read(plan).requests[0].applied_at).toBeUndefined();
  });

  it("apply sends the key only to the Management API of the env of each entry", async () => {
    const t = setup();
    const plan = join(t.root, "plan.json");
    await t.exec(...update(plan, "plc_1"));
    for (const bad of [{ url: "https://evil.example/management/v1/x" }, { url: "https://api.purchasely.io.evil.example/management/v1/x" }, { env: "staging" }, { method: "GET" }, { body: "{bad" }]) {
      const p = read(plan);
      Object.assign(p.requests[0], bad);
      writeFileSync(plan + ".bad", JSON.stringify(p));
      const r = await t.exec("apply", plan + ".bad", "--yes");
      expect(r.code, JSON.stringify(bad)).toBe(10);
      expect(r.err).toContain("Request #1 of the plan");
    }
    expect((await t.exec("apply", join(t.root, "missing.json"), "--yes")).code).toBe(2);
    expect(t.calls).toHaveLength(0);
  });

  it("apply keeps read-only mode", async () => {
    const t = setup({ env: { PURCHASELY_READ_ONLY: "1" } });
    const plan = join(t.root, "plan.json");
    await t.exec(...update(plan, "plc_1"));
    expect((await t.exec("apply", plan, "--yes")).code).toBe(10);
    expect(t.calls).toHaveLength(0);
  });
});
