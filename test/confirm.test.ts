import { describe, expect, it } from "vitest";
import { setup, type Reply } from "./helpers.js";

const need = (token = "tok_1"): Reply => ({ status: 422, body: { error: { code: "CONFIRMATION_REQUIRED", messages: ["Used by 3 live campaigns"] }, confirm_token: token } });
const ok: Reply = { status: 200, body: { deleted: true } };
const del = ["placements", "delete", "--env", "prod", "--app", "app_1", "--id", "plc_1", "--fields", "id"];
const put = ["webhooks", "update", "--env", "prod", "--app", "app_1", "--data", '{ "events": {"a": {"enabled": true}} }'];

describe("confirm token flow", () => {
  it("--yes resends once with the token in the query", async () => {
    const t = setup({ replies: [need(), ok] });
    const r = await t.exec(...del, "--yes");
    expect(r.code).toBe(0);
    expect(t.calls).toHaveLength(2);
    const [a, b] = t.calls.map((c) => new URL(c.url));
    expect(a.searchParams.get("confirm_token")).toBeNull();
    expect(b.searchParams.get("confirm_token")).toBe("tok_1");
    expect(t.calls[1].url).toBe(`${t.calls[0].url}&confirm_token=tok_1`);
    expect(r.err).toContain("Impact: Used by 3 live campaigns");
    expect(r.json()).toEqual({ deleted: true });
  });

  it("puts the token in the body for PUT webhooks, rest unchanged", async () => {
    const t = setup({ replies: [need(), ok] });
    expect((await t.exec(...put, "--yes")).code).toBe(0);
    expect(t.calls[1].url).toBe(t.calls[0].url);
    expect(JSON.parse(t.calls[0].init.body as string)).toEqual({ events: { a: { enabled: true } } });
    expect(JSON.parse(t.calls[1].init.body as string)).toEqual({ events: { a: { enabled: true } }, confirm_token: "tok_1" });
    expect((t.calls[1].init.body as string).startsWith((t.calls[0].init.body as string).slice(0, -1))).toBe(true);
  });

  it("exits 8 without --yes when not interactive, with the token on stderr", async () => {
    const t = setup({ replies: [need("tok_9")] });
    const r = await t.exec(...del);
    expect(r.code).toBe(8);
    expect(r.errJson()).toMatchObject({ error: { code: "CONFIRMATION_REQUIRED", status: 422 }, confirm_token: "tok_9" });
    expect(t.calls).toHaveLength(1);
  });

  it("an agent can resend with --confirm-token", async () => {
    const t = setup({ replies: [ok] });
    expect((await t.exec(...del, "--confirm-token", "tok_9")).code).toBe(0);
    expect(new URL(t.calls[0].url).searchParams.get("confirm_token")).toBe("tok_9");
  });

  it("asks on a TTY: y resends, n exits 8", async () => {
    const yes = setup({ replies: [need(), ok], isTTY: true, answers: ["y"] });
    expect((await yes.exec(...del)).code).toBe(0);
    expect(yes.prompts).toEqual(["Apply this change? [y/N] "]);
    expect(yes.calls).toHaveLength(2);

    const no = setup({ replies: [need(), ok], isTTY: true, answers: ["n"] });
    const r = await no.exec(...del);
    expect(r.code).toBe(8);
    expect(no.calls).toHaveLength(1);
    expect(r.errJson().confirm_token).toBe("tok_1");
  });

  it("a second CONFIRMATION_REQUIRED exits 8 after two calls", async () => {
    const t = setup({ replies: [need("a"), { status: 403, body: { error: { code: "CONFIRMATION_REQUIRED", messages: ["stale"] }, confirm_token: "b" } }] });
    const r = await t.exec(...del, "--yes");
    expect(r.code).toBe(8);
    expect(t.calls).toHaveLength(2);
  });

  it("does not loop when the user passed a token that is refused", async () => {
    const t = setup({ replies: [need("new")] });
    expect((await t.exec(...del, "--confirm-token", "old", "--yes")).code).toBe(8);
    expect(t.calls).toHaveLength(1);
  });
});
