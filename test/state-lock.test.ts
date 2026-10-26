import { mkdtempSync, existsSync, utimesSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { StateStore } from "../src/core/state.js";

const store = () => {
  const stateDir = mkdtempSync(join(tmpdir(), "cli-lock-"));
  const s = new StateStore({ stateDir, pid: process.pid, sleep: (ms) => new Promise((r) => setTimeout(r, ms)) }, "k");
  return { s, lock: join(stateDir, "k.lock") };
};
const back = (file: string, ms: number) => utimesSync(file, new Date(Date.now() - ms), new Date(Date.now() - ms));

describe("withLock", () => {
  it("async callback keeps the lock", async () => {
    const { s } = store();
    let open!: () => void;
    const gate = new Promise<void>((r) => (open = r));
    const log: string[] = [];
    const a = s.withLock(async () => {
      log.push("a start");
      await gate;
      log.push("a end");
    });
    const b = s.withLock(async () => {
      log.push("b start");
      log.push("b end");
    });
    await new Promise((r) => setTimeout(r, 50));
    expect(log).toEqual(["a start"]);
    open();
    await Promise.all([a, b]);
    expect(log).toEqual(["a start", "a end", "b start", "b end"]);
  });

  it("live owner is not stolen", async () => {
    const { s, lock } = store();
    writeFileSync(lock, String(process.pid));
    back(lock, 11_000);
    await expect(s.withLock(() => 1, 150)).rejects.toThrow(/state lock timeout/);
    expect(existsSync(lock)).toBe(true);

    const dead = spawnSync(process.execPath, ["-e", ""]).pid!;
    writeFileSync(lock, String(dead));
    await expect(s.withLock(() => "taken")).resolves.toBe("taken");

    writeFileSync(lock, "");
    back(lock, 11_000);
    await expect(s.withLock(() => "empty and old")).resolves.toBe("empty and old");
    expect(existsSync(lock)).toBe(false);
  });
});
