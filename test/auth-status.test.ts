import { describe, expect, it } from "vitest";
import { credentialDisplay, keyHash } from "../src/core/credentials.js";
import { logUsage } from "../src/core/usage.js";
import { KEY, setup } from "./helpers.js";

describe("auth status prints its result once", () => {
  it("with no key: nothing on stdout, one error JSON on stderr, exit 3", async () => {
    const t = setup({ env: { PURCHASELY_API_KEY: "" } });
    const r = await t.exec("auth", "status");
    expect(r.code).toBe(3);
    expect(r.out).toBe("");
    const jsonLines = r.err.trim().split("\n").filter((l) => l.startsWith("{"));
    expect(jsonLines).toHaveLength(1);
    expect(JSON.parse(jsonLines[0]).error.code).toBe("UNAUTHORIZED");
  });

  it("with a key: one JSON object on stdout, exit 0", async () => {
    const r = await setup().exec("auth", "status");
    expect(r.code).toBe(0);
    expect(r.out.trim().split("\n")).toHaveLength(1);
    expect(r.json()).toMatchObject({ source: "env", key: credentialDisplay("management", KEY) });
  });
});

describe("one key label in auth status and usage", () => {
  const at = (t: ReturnType<typeof setup>, surface: string, key: string) =>
    logUsage(t.d, { env: "prod", surface, key: keyHash(key), method: "GET", path: "/x", status: 200, rate_limited: false, request_id: "r" });

  it("labels the current key with the label of auth status and an older key with its hash", async () => {
    const t = setup();
    at(t, "management", KEY);
    at(t, "management", "pk_old_key_ZZZZ");
    const status = (await t.exec("auth", "status")).json();
    const u = (await t.exec("usage")).json();
    const row = (k: string) => u.last_hour.find((r: { key: string }) => r.key === keyHash(k));
    expect(row(KEY)).toMatchObject({ key: keyHash(KEY), label: status.key });
    expect(row("pk_old_key_ZZZZ")).toMatchObject({ key: keyHash("pk_old_key_ZZZZ"), label: `hash:${keyHash("pk_old_key_ZZZZ")}` });
    expect(u.current).toMatchObject({ key: keyHash(KEY), label: status.key });
    expect(u.surfaces[0].label).toBe(status.key);
    expect(JSON.stringify(u)).not.toContain(KEY);
  });

});
