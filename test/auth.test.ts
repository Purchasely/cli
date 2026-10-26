import { existsSync, readFileSync, statSync, writeFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { EXT, setup } from "./helpers.js";

const mode = (p: string) => statSync(p).mode & 0o777;

describe("credentials", () => {
  it("env var beats keychain beats file", async () => {
    const t = setup({ env: { PURCHASELY_API_KEY: "" } });
    expect((await t.exec("auth", "status")).code).toBe(3);

    t.keychain.fail = true;
    await t.exec("auth", "login", "--api-key", "pk_file_AAAA");
    t.keychain.fail = false;
    expect((await t.exec("auth", "status")).json().source).toBe("file");

    await t.exec("auth", "login", "--api-key", "pk_keychain_BBBB");
    const s = (await t.exec("auth", "status")).json();
    expect(s).toMatchObject({ source: "keychain", key: "…BBBB" });

    t.d.env.PURCHASELY_API_KEY = "pk_env_CCCC";
    expect((await t.exec("auth", "status")).json()).toMatchObject({ source: "env", key: "…CCCC" });
  });

  it("is per env and per surface", async () => {
    const t = setup({ env: { PURCHASELY_API_KEY: "" } });
    await t.exec("auth", "login", "--api-key", "pk_prod_1111", "--env", "prod");
    expect((await t.exec("auth", "status", "--env", "staging")).code).toBe(3);
    expect([...t.keychain.store.keys()]).toEqual(["purchasely-cli/prod:management"]);
  });

  it("writes the file with mode 0600 in a 0700 dir, also over an existing file", async () => {
    const t = setup({ name: "acme", ext: EXT, env: { PURCHASELY_API_KEY: "" } });
    t.d.keychain = null;
    mkdirSync(t.d.configDir, { recursive: true, mode: 0o755 });
    const file = join(t.d.configDir, "credentials.json");
    writeFileSync(file, "{}", { mode: 0o644 });
    await t.exec("auth", "login", "--api-key", "pk_file_AAAA", "--email", "kevin@purchasely.com");
    expect(mode(file)).toBe(0o600);
    expect(mode(t.d.configDir)).toBe(0o700);
    expect(JSON.parse(readFileSync(file, "utf8")).prod.management).toBe("pk_file_AAAA");
    expect(mode(join(t.d.configDir, "config.json"))).toBe(0o600);
    expect(JSON.parse(readFileSync(join(t.d.configDir, "config.json"), "utf8"))).toMatchObject({ identity_email: "kevin@purchasely.com" });
  });

  it("reads the key from stdin or a hidden prompt, and logs out", async () => {
    const t = setup({ env: { PURCHASELY_API_KEY: "" }, stdin: "pk_stdin_DDDD\n" });
    await t.exec("auth", "login");
    expect((await t.exec("auth", "status")).json().key).toBe("…DDDD");
    const tty = setup({ name: "acme", ext: EXT, env: { PURCHASELY_API_KEY: "" }, isTTY: true, answers: ["pk_tty_EEEE", "me@purchasely.com"] });
    await tty.exec("auth", "login");
    expect((await tty.exec("auth", "status")).json()).toMatchObject({ key: "…EEEE", identity: "me@purchasely.com" });
    expect((await tty.exec("auth", "login", "--api-key", "pk_x", "--email", "nope")).code).toBe(2);
    await t.exec("auth", "logout");
    expect((await t.exec("auth", "status")).code).toBe(3);
  });

  it("purchasely login asks only for the key and keeps no email", async () => {
    const tty = setup({ env: { PURCHASELY_API_KEY: "" }, isTTY: true, answers: ["pk_tty_EEEE", "me@purchasely.com"] });
    const r = await tty.exec("auth", "login");
    expect(r.json()).toMatchObject({ stored: true, identity: null });
    expect(tty.prompts).toEqual(["Management API key: "]);
    expect((await tty.exec("auth", "status")).json().identity).toBeNull();
    expect(existsSync(join(tty.d.configDir, "config.json")) ? readFileSync(join(tty.d.configDir, "config.json"), "utf8") : "").not.toContain("@");
    const bad = await setup({ env: { PURCHASELY_API_KEY: "" } }).exec("auth", "login", "--api-key", "pk_x", "--email", "me@purchasely.com");
    expect(bad.code).toBe(2);
    expect(bad.err).toContain("unknown option");
    expect((await setup().exec("auth", "login", "--help")).out).not.toContain("--email");
  });

  it("an extension with identity keeps the email prompt and the option", async () => {
    const tty = setup({ name: "acme", ext: EXT, env: { PURCHASELY_API_KEY: "" }, isTTY: true, answers: ["pk_tty_EEEE", "me@purchasely.com"] });
    expect((await tty.exec("auth", "login")).json()).toMatchObject({ stored: true, identity: "me@purchasely.com" });
    expect(tty.prompts).toHaveLength(2);
    expect((await tty.exec("auth", "status")).json().identity).toBe("me@purchasely.com");
    expect((await tty.exec("auth", "login", "--help")).out).toContain("--email");
  });

  it("exits 3 without a key on a real call", async () => {
    const t = setup({ env: { PURCHASELY_API_KEY: "" } });
    const r = await t.exec("placements", "list");
    expect(r.code).toBe(3);
    expect(t.calls).toHaveLength(0);
  });
});
