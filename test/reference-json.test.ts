import { describe, expect, it } from "vitest";
import { EXT, setup } from "./helpers.js";

interface Node { name: string; description: string; arguments: { name: string; required: boolean }[]; options: { flags: string; description: string; required: boolean; default?: unknown }[]; commands: Node[]; extension?: boolean }
interface Ref { name: string; version: string; description: string; commands: Node[] }
const find = (nodes: Node[], ...path: string[]): Node | undefined => path.reduce<Node | undefined>((n, name, i) => (i === 0 ? nodes : n?.commands)?.find((c) => c.name === name) ?? undefined, undefined);

describe("reference --format json", () => {
  it("prints the tree with arguments, options and subcommands", async () => {
    const r = await setup({ env: { PURCHASELY_API_KEY: "" } }).exec("reference", "--format", "json");
    expect(r.code, r.err).toBe(0);
    const ref = r.json() as Ref;
    expect(ref).toMatchObject({ name: "purchasely", version: "9.9.9" });
    expect(ref.description).toBe("Purchasely command line tool");
    const list = find(ref.commands, "placements", "list")!;
    expect(list.description).toBeTruthy();
    expect(list.options.map((o) => o.flags)).toEqual(expect.arrayContaining([expect.stringContaining("--app")]));
    expect(list.options.every((o) => typeof o.description === "string" && typeof o.required === "boolean")).toBe(true);
    expect(list.commands).toEqual([]);
    expect(find(ref.commands, "completion")!.arguments).toEqual([expect.objectContaining({ name: "shell", required: true })]);
    const fmt = find(ref.commands, "reference")!.options.find((o) => o.flags.startsWith("--format"))!;
    expect(fmt.default).toBe("markdown");
    expect(find(ref.commands, "auth", "login")).toBeTruthy();
  });

  it("is offline: no key, no request, no stderr line", async () => {
    const t = setup({ env: { PURCHASELY_API_KEY: "" } });
    const r = await t.exec("reference", "--format", "json");
    expect(r.code).toBe(0);
    expect(r.err).toBe("");
    expect(t.calls).toEqual([]);
  });

  it("leaves out a hidden command and a hidden option", async () => {
    const t = setup({ name: "acme", ext: { ...EXT, modules: [(p) => {
      p.command("secret-cmd", { hidden: true }).description("x");
      const v = p.command("visible-cmd").description("A visible one").option("--visible-opt <x>", "x");
      v.addOption(Object.assign(v.createOption("--gone-opt <x>", "x"), { hidden: true }));
    }] } });
    const text = (await t.exec("reference", "--format", "json")).out;
    expect(text).toContain("visible-cmd");
    expect(text).toContain("--visible-opt");
    expect(text).not.toContain("secret-cmd");
    expect(text).not.toContain("gone-opt");
    expect(text).not.toContain('"--help"');
  });

  it("marks the commands of an extension, and has no mark without one", async () => {
    const pub = (await setup().exec("reference", "--format", "json")).json() as Ref;
    expect(pub.commands.some((c) => "extension" in c)).toBe(false);
    expect(find(pub.commands, "extra")).toBeUndefined();
    const acme = (await setup({ name: "acme", ext: EXT }).exec("reference", "--format", "json")).json() as Ref;
    expect(find(acme.commands, "extra")).toMatchObject({ extension: true });
    expect(find(acme.commands, "placements")).toMatchObject({ extension: false });
  });

  it("the help of reference names json, and a bad format says so", async () => {
    const t = setup();
    expect((await t.exec("reference", "--help")).out).toContain("json");
    const r = await t.exec("reference", "--format", "yaml");
    expect(r.code).toBe(2);
    expect(r.errJson().error.messages.join(" ")).toContain("json");
  });
});
