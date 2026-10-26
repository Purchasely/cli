import { Command } from "commander";
import { describe, expect, it } from "vitest";
import { realExec } from "../src/core/exec.js";
import { group } from "../src/core/runtime.js";

describe("plumbing", () => {
  it("group reuses a command and keeps the apps alias", () => {
    const program = new Command("purchasely");
    const first = group(program, "applications");
    expect(group(program, "applications")).toBe(first);
    expect(first.aliases()).toEqual(["apps"]);
    expect(program.commands).toHaveLength(1);
  });

  it("real exec: stdin, exit code, timeout, missing binary", async () => {
    const o = { timeoutMs: 5000 };
    expect(await realExec(process.execPath, ["-e", "process.stdin.pipe(process.stdout)"], { ...o, input: "hello" })).toMatchObject({ code: 0, stdout: "hello", timedOut: false });
    expect(await realExec(process.execPath, ["-e", "process.exit(3)"], o)).toMatchObject({ code: 3, timedOut: false });
    expect(await realExec(process.execPath, ["-e", "setTimeout(()=>{},9999)"], { timeoutMs: 100 })).toMatchObject({ timedOut: true });
    await expect(realExec("no-such-binary-xyz", [], o)).rejects.toMatchObject({ code: "ENOENT" });
  });
});
