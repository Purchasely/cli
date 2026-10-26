import { afterEach, describe, expect, it, vi } from "vitest";
import { main } from "../src/main.js";

describe("main", () => {
  const argv = process.argv;
  const code = process.exitCode;
  afterEach(() => {
    process.argv = argv;
    process.exitCode = code;
    process.stdout.removeAllListeners("error");
    vi.restoreAllMocks();
  });

  it("ignores EPIPE on stdout, so `reference | head` prints no stack", async () => {
    vi.spyOn(process.stdout, "write").mockReturnValue(true);
    process.argv = ["node", "purchasely", "reference"];
    await main("purchasely", []);
    const err = (c: string) => Object.assign(new Error(`write ${c}`), { code: c });
    expect(() => process.stdout.emit("error", err("EPIPE"))).not.toThrow();
    // Any other stdout error is still fatal.
    expect(() => process.stdout.emit("error", err("EIO"))).toThrow("write EIO");
  });
});
