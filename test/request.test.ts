import { describe, expect, it } from "vitest";
import { writeConfig, type Config } from "../src/core/config.js";
import { EXT, setup } from "./helpers.js";

describe("request building", () => {
  it("substitutes and encodes path params", async () => {
    const t = setup();
    await t.exec("placements", "get", "--app", "app_1", "--id", "plc/with space");
    expect(t.calls[0].url).toBe("https://api.purchasely.io/management/v1/applications/app_1/placements/plc%2Fwith%20space");
    expect(t.calls[0].init.method).toBe("GET");
  });

  it("sends arrays with brackets, in either input style", async () => {
    const t = setup();
    await t.exec("placements", "list", "--app", "app_1", "--fields", "id,name", "--fields", "vendor_id");
    expect(t.calls[0].url).toBe("https://api.purchasely.io/management/v1/applications/app_1/placements?fields[]=id&fields[]=name&fields[]=vendor_id");
  });

  it("parses integers and rejects bad ones", async () => {
    const t = setup();
    await t.exec("placements", "list", "--app", "app_1", "--page", "2", "--per-page", "50");
    expect(t.calls[0].url).toContain("page=2&per_page=50");
    const bad = await t.exec("placements", "list", "--app", "app_1", "--page", "two");
    expect(bad.code).toBe(2);
    expect(bad.errJson().error.code).toBe("USAGE");
    expect(t.calls).toHaveLength(1);
  });

  it("validates enums", async () => {
    const t = setup();
    const r = await t.exec("dashboard", "sdk-evolution", "--app", "app_1", "--group-by", "nope");
    expect(r.code).toBe(2);
    expect(r.err).toContain("sdk_version");
  });

  it("chooses the global or the app path", async () => {
    const t = setup();
    await t.exec("placements", "list");
    await t.exec("placements", "list", "--app", "app_1");
    expect(t.calls[0].url).toBe("https://api.purchasely.io/management/v1/placements");
    expect(t.calls[1].url).toBe("https://api.purchasely.io/management/v1/applications/app_1/placements");
  });

  it("requires --app when the command has only an app variant, and required path params", async () => {
    const t = setup();
    expect((await t.exec("placements", "get", "--id", "x")).code).toBe(2);
    expect((await t.exec("placements", "get", "--app", "app_1")).code).toBe(2);
    expect(t.calls).toHaveLength(0);
  });

  it("rejects an option the chosen variant does not accept", async () => {
    const { buildRequest } = await import("../src/surfaces/management/request.js");
    const mk = (path: string, app: boolean, names: string[]) => ({
      method: "GET", path, tag: "T", summary: "", description: "", scope: "s", app,
      params: [...(app ? [{ name: "application_id", in: "path" as const, required: true, schema: {} }] : []),
        ...names.map((name) => ({ name, in: "query" as const, required: false, schema: { type: "string" } }))],
    });
    const cmd = {
      resource: "t", action: "list", summary: "",
      appOp: mk("/management/v1/applications/{application_id}/t", true, ["a", "b"]),
      globalOp: mk("/management/v1/t", false, ["a"]),
      options: [{ name: "a", flag: "a", kind: "query" as const }, { name: "b", flag: "b", kind: "query" as const }],
    };
    expect(() => buildRequest(cmd, { b: "1" }, "prod", undefined)).toThrow(/--b is not accepted without --app/);
    expect(buildRequest(cmd, { b: "1", application_id: "app_1" }, "prod", undefined).url).toBe("https://api.purchasely.io/management/v1/applications/app_1/t?b=1");
  });

  it("sends the body from --data, @file and stdin", async () => {
    const t = setup({ stdin: '{"from":"stdin"}' });
    await t.exec("placements", "create", "--env", "staging", "--app", "app_1", "--data", '{"name":"x"}');
    expect(t.calls[0].init.body).toBe('{"name":"x"}');
    expect(t.calls[0].url.startsWith("https://api-staging.purchasely.io/")).toBe(true);
    await t.exec("placements", "create", "--env", "prod", "--app", "app_1", "--data", "-");
    expect(t.calls[1].init.body).toBe('{"from":"stdin"}');
    expect((await t.exec("placements", "create", "--env", "prod", "--app", "app_1", "--data", "{bad")).code).toBe(2);
    expect((await t.exec("placements", "create", "--env", "prod", "--app", "app_1")).code).toBe(2);
  });

  it("sends the identification headers", async () => {
    const t = setup({ name: "acme", ext: EXT });
    await t.exec("auth", "login", "--api-key", "pk_x", "--email", "kevin@purchasely.com");
    await t.exec("placements", "list", "--reason", "weekly audit");
    await t.exec("placements", "list");
    const h = t.calls[0].init.headers;
    expect(h["User-Agent"]).toBe("purchasely-cli/9.9.9 (kevin@purchasely.com; weekly audit; placements list; script)");
    expect(h["X-Request-Id"]).toMatch(/^[0-9a-f-]{36}$/);
    expect(h.Authorization).toBe("Bearer pk_mgmt_live_SECRET123");
    expect(t.calls[1].init.headers["User-Agent"]).toContain("; -; placements list; script)");
    expect(t.calls[1].init.headers["X-Request-Id"]).not.toBe(h["X-Request-Id"]);
  });

  it("never puts an email in the public User-Agent, even with identity_email in the shared config", async () => {
    const t = setup();
    writeConfig(t.d.configDir, { identity_email: "kevin@purchasely.com" } as Config);
    await t.exec("placements", "list", "--reason", "weekly audit");
    await t.exec("placements", "list");
    expect(t.calls[0].init.headers["User-Agent"]).toBe("purchasely-cli/9.9.9 (-; weekly audit; placements list; script)");
    expect(t.calls[1].init.headers["User-Agent"]).toBe("purchasely-cli/9.9.9 (-; -; placements list; script)");
    expect((await t.exec("placements", "list")).err.split("\n")[0]).toBe("purchasely env=prod surface=management identity=- key=…T123");
  });

  it("keeps the email in the User-Agent of an extension with identity", async () => {
    const t = setup({ name: "acme", ext: EXT });
    await t.exec("auth", "login", "--api-key", "pk_x", "--email", "kevin@purchasely.com");
    await t.exec("placements", "list", "--reason", "weekly audit");
    expect(t.calls[0].init.headers["User-Agent"]).toBe("purchasely-cli/9.9.9 (kevin@purchasely.com; weekly audit; placements list; script)");
  });

  it("sends the command name and the runner, never the argument values", async () => {
    for (const [opts, who] of [[{ env: { CLAUDECODE: "1", CI: "true" } }, "agent"], [{ env: { CI: "true" } }, "ci"], [{ isTTY: true }, "tty"]] as const) {
      const t = setup(opts);
      await t.exec("placements", "get", "--app", "app_secret_1", "--id", "plc_1");
      expect(t.calls[0].init.headers["User-Agent"]).toBe(`purchasely-cli/9.9.9 (-; -; placements get; ${who})`);
    }
  });

  it("read-only mode refuses a write before the send, and keeps reads and --dry-run", async () => {
    const t = setup({ env: { PURCHASELY_READ_ONLY: "1" } });
    const w = await t.exec("placements", "create", "--env", "staging", "--app", "app_1", "--data", "{}");
    expect(w.code).toBe(10);
    expect(w.err).toContain("PURCHASELY_READ_ONLY=1");
    expect(t.calls).toHaveLength(0);
    expect((await t.exec("placements", "create", "--env", "staging", "--app", "app_1", "--data", "{}", "--dry-run")).code).toBe(0);
    expect(t.calls).toHaveLength(0);
    expect((await t.exec("placements", "list")).code).toBe(0);
    expect(t.calls).toHaveLength(1);
    const c = setup();
    writeConfig(c.d.configDir, { read_only: true });
    const r = await c.exec("api", "DELETE", "/applications/app_1/placements/plc_1", "--env", "prod");
    expect(r.code).toBe(10);
    expect(r.err).toContain("read_only is true in the config");
    expect(c.calls).toHaveLength(0);
  });

  it("gives every request a time limit that also covers the body read", async () => {
    const t = setup();
    await t.exec("placements", "list");
    expect(t.calls[0].init.signal).toBeInstanceOf(AbortSignal);
  });

  it("prints the env header on stderr", async () => {
    const t = setup();
    const r = await t.exec("placements", "list", "--env", "staging");
    expect(r.err.split("\n")[0]).toBe("purchasely env=staging surface=management identity=- key=…T123");
  });

  it("needs --env on every write, also with --dry-run", async () => {
    const t = setup();
    for (const extra of [[], ["--dry-run"]]) {
      const r = await t.exec("placements", "delete", "--app", "app_1", "--id", "p", ...extra);
      expect(r.code).toBe(10);
    }
    expect(t.calls).toHaveLength(0);
    expect((await t.exec("placements", "list", "--env", "mars")).code).toBe(2);
  });

  it("reads PURCHASELY_ENV for reads only", async () => {
    const t = setup({ env: { PURCHASELY_ENV: "staging" } });
    await t.exec("placements", "list");
    expect(t.calls[0].url.startsWith("https://api-staging.purchasely.io/")).toBe(true);
    expect((await t.exec("placements", "delete", "--app", "a", "--id", "b")).code).toBe(10);
  });

  it("prints a table", async () => {
    const t = setup({ replies: [{ body: { placements: [{ id: "p1", name: "Home", extra: { a: 1 } }, { id: "p2", name: "Onboarding" }] } }] });
    const r = await t.exec("placements", "list", "--output", "table");
    expect(r.out).toContain("id  name");
    // B5: an object column shows only when --fields asks for it.
    expect(r.out).not.toContain('{"a":1}');
    const t2 = setup({ replies: [{ body: { placements: [{ id: "p1", name: "Home", extra: { a: 1 } }] } }] });
    expect((await t2.exec("placements", "list", "--output", "table", "--fields", "id,extra")).out).toContain('{"a":1}');
  });
});
