import { describe, expect, it } from "vitest";
import { macKeychain } from "../src/core/credentials.js";

// The real macOS keychain: a throwaway item, deleted at the end. Off in CI (no login keychain there).
describe.skipIf(process.platform !== "darwin" || !!process.env.CI)("macOS keychain", () => {
  it("stores a secret with quotes and spaces through stdin, and reads it back", async () => {
    const secret = JSON.stringify({ email: "a b@x", password: 'p"w;1 $(x)' });
    try {
      await macKeychain.set("purchasely-cli-test", "test:keychain", secret);
      expect(await macKeychain.get("purchasely-cli-test", "test:keychain")).toBe(secret);
    } finally {
      await macKeychain.delete("purchasely-cli-test", "test:keychain");
    }
    expect(await macKeychain.get("purchasely-cli-test", "test:keychain")).toBeNull();
  });
});
