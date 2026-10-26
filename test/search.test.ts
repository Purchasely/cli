import { describe, expect, it } from "vitest";
import { setup } from "./helpers.js";

const NONE = { PURCHASELY_API_KEY: "" };

describe("search finds every command", () => {
  it("finds the generated commands that a hand-written command shadows", async () => {
    const t = setup({ env: NONE });
    for (const [task, command] of [["list screens", "purchasely screens list"], ["get placements", "purchasely placements get"], ["list applications", "purchasely applications list"]]) {
      const r = await t.exec("search", task, "--limit", "30");
      expect(r.json().map((e: { command: string }) => e.command).some((c: string) => c.startsWith(command)), task).toBe(true);
    }
  });

  it("lists the hand-written commands", async () => {
    const t = setup({ env: NONE });
    for (const cmd of ["usage", "purge", "auth login", "auth status", "auth logout", "dashboard summary", "api", "reference"]) {
      const found = (await t.exec("search", cmd, "--limit", "50")).json().map((e: { command: string }) => e.command);
      expect(found.some((c: string) => c.startsWith(`purchasely ${cmd}`)), cmd).toBe(true);
    }
  });

  it("returns no duplicate and no help entry", async () => {
    const t = setup({ env: NONE });
    const found = (await t.exec("search", "list placements", "--limit", "200")).json().map((e: { command: string }) => e.command);
    expect(new Set(found).size).toBe(found.length);
    expect(found).not.toContain("purchasely help");
  });

  it("finds games first for a game task, and never a hidden command", async () => {
    const t = setup({ env: NONE });
    const found = (await t.exec("search", "play a game", "--limit", "50")).json().map((e: { command: string }) => e.command);
    expect(found[0]).toBe("purchasely games");
    expect(found).not.toContain("purchasely play");
    const all = (await t.exec("search", "play game terminal", "--limit", "200")).json().map((e: { command: string }) => e.command);
    expect(all).not.toContain("purchasely play");
  });
});
