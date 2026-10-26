import { describe, expect, it } from "vitest";
import { BASE_URLS } from "../src/core/types.js";
import { deriveCommands, loadSpec, parseOps } from "../src/surfaces/management/spec.js";
import { EXT, setup } from "./helpers.js";

const doc = loadSpec();
const ops = parseOps(doc);
const cmds = deriveCommands(ops);
const find = (r: string, a: string) => cmds.find((c) => c.resource === r && c.action === a);

describe("command generation", () => {
  it("reads 166 operations in 123 paths", () => {
    expect(Object.keys(doc.paths)).toHaveLength(123);
    expect(ops).toHaveLength(166);
  });

  it("keeps the prod base URL in sync with the spec", () => {
    expect(doc.servers[0].url).toBe(BASE_URLS.prod);
  });

  it("gives every command a unique name and reaches every operation", () => {
    const names = cmds.map((c) => `${c.resource} ${c.action}`);
    expect(new Set(names).size).toBe(names.length);
    const reached = cmds.flatMap((c) => [c.appOp, c.globalOp]).filter(Boolean);
    expect(new Set(reached).size).toBe(166);
    expect(cmds.length).toBe(134);
  });

  it("derives the known names", () => {
    expect(find("placements", "list")?.appOp?.path).toBe("/management/v1/applications/{application_id}/placements");
    expect(find("placements", "list")?.globalOp?.path).toBe("/management/v1/placements");
    expect(find("placements", "delete-audience")?.appOp?.method).toBe("DELETE");
    expect(find("ab-tests", "start")?.appOp?.method).toBe("PUT");
    expect(find("web-activations", "flow")).toBeTruthy();
    expect(find("web-activations", "screen")).toBeTruthy();
    const dist = find("distributions", "get")!;
    expect(dist.options.map((o) => o.flag)).toEqual(expect.arrayContaining(["plan-id", "store"]));
    expect(find("applications", "list")).toBeTruthy();
  });

  it("finds the confirm token location per operation", () => {
    const where = (r: string, a: string) => (find(r, a)!.appOp ?? find(r, a)!.globalOp)!.confirm;
    expect(where("placements", "delete")).toBe("query");
    expect(where("webhooks", "update")).toBe("body");
    expect(where("plans", "create")).toBe("body");
    expect(where("audiences", "discard")).toBe("body");
    expect(ops.filter((o) => o.confirm).length).toBeGreaterThanOrEqual(12);
  });

  it("prints a table of resource action to METHOD path", () => {
    const table = cmds.map((c) => `${c.resource} ${c.action} -> ${[c.appOp, c.globalOp].filter(Boolean).map((o) => `${o!.method} ${o!.path}`).join(" | ")}`);
    expect(table).toHaveLength(134);
  });

  it("maps apps to applications and --app-id to --app", async () => {
    const t = setup({ replies: [{ body: { applications: [] } }] });
    expect((await t.exec("apps", "list")).code).toBe(0);
    expect(t.calls[0].url).toBe("https://api.purchasely.io/management/v1/applications");
    expect((await t.exec("placements", "list", "--app-id", "app_1")).code).toBe(0);
    expect((await t.exec("placements", "list", "--app-id=app_2")).code).toBe(0);
    expect(t.calls[1].url).toContain("/applications/app_1/placements");
    expect(t.calls[2].url).toContain("/applications/app_2/placements");
  });
});

describe("search", () => {
  it("puts placements list first", async () => {
    const t = setup();
    const r = await t.exec("search", "list placements");
    expect(r.code).toBe(0);
    const res = r.json();
    expect(res[0].command).toBe("purchasely placements list --app app_xxx");
    expect(res).toHaveLength(10);
    expect(Object.keys(res[0])).toEqual(["command", "summary", "method", "path", "scope"]);
    expect((await t.exec("search", "list placements", "--limit", "2")).json()).toHaveLength(2);
    expect((await t.exec("search", "list placements", "--output", "table")).out).toContain("command");
  });
});

describe("a group with no subcommand", () => {
  it("prints its help on stdout, exits 0 and prints no error JSON, for a generated group and a group of an extension", async () => {
    for (const [name, argv, usage] of [["purchasely", ["web-flows"], "Usage: purchasely web-flows"], ["acme", ["extra"], "Usage: acme extra"]] as const) {
      const t = setup({ name, ext: name === "acme" ? EXT : undefined });
      const r = await t.exec(...argv);
      expect(r.code, argv.join(" ")).toBe(0);
      expect(r.out, argv.join(" ")).toContain(usage);
      expect(r.err, argv.join(" ")).toBe("");
      expect(t.calls).toHaveLength(0);
    }
  });

  it("still refuses an unknown subcommand", async () => {
    const r = await setup().exec("web-flows", "nope");
    expect(r.code).toBe(2);
    expect(r.errJson().error.messages[0]).toContain("unknown command 'nope'");
  });
});
