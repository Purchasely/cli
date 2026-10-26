import { execFileSync, spawn } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { beforeAll, describe, expect, it } from "vitest";
import { Limiter, MAX_IN_FLIGHT, WRITE_GAP_MS } from "../src/core/limiter.js";
import { StateStore } from "../src/core/state.js";

/** A virtual clock. Sleepers wake in time order once every task is idle. */
class FakeClock {
  t = 1_000_000;
  timers: { at: number; wake: () => void }[] = [];
  now = () => this.t;
  sleep = (ms: number) => new Promise<void>((wake) => this.timers.push({ at: this.t + ms, wake }));
  async run<T>(p: Promise<T>): Promise<T> {
    let done = false;
    const guarded = p.finally(() => void (done = true));
    while (!done) {
      await new Promise((r) => setImmediate(r));
      if (done) break;
      this.timers.sort((a, b) => a.at - b.at);
      const next = this.timers.shift();
      if (!next) throw new Error("deadlock: nothing is waiting");
      this.t = Math.max(this.t, next.at);
      next.wake();
    }
    return guarded;
  }
}

const maxInWindow = (starts: number[], windowMs: number) =>
  Math.max(...starts.map((s) => starts.filter((x) => x >= s && x < s + windowMs).length));

function maxConcurrent(spans: { start: number; end: number }[]) {
  const ev = spans.flatMap((s) => [[s.start, 1], [s.end, -1]]).sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  let cur = 0, max = 0;
  for (const [, d] of ev) max = Math.max(max, (cur += d));
  return max;
}

describe("limiter (fake clock)", () => {
  it("50 parallel callers never exceed the rate nor 4 in flight", async () => {
    const clock = new FakeClock();
    const d = { stateDir: mkdtempSync(join(tmpdir(), "cli-lim-")), sleep: clock.sleep, now: clock.now, pid: process.pid };
    const limiter = new Limiter(d, new StateStore(d, "k"));
    const rate = 5;
    const spans: { start: number; end: number }[] = [];
    const one = async () => {
      const { at, release } = await limiter.acquire({ rate, write: false });
      const start = at;
      await clock.sleep(700);
      spans.push({ start, end: clock.now() });
      await release();
    };
    await clock.run(Promise.all(Array.from({ length: 50 }, one)));
    expect(spans).toHaveLength(50);
    expect(maxInWindow(spans.map((s) => s.start), 1000)).toBeLessThanOrEqual(rate);
    expect(maxConcurrent(spans)).toBeLessThanOrEqual(MAX_IN_FLIGHT);
    expect(maxConcurrent(spans)).toBe(MAX_IN_FLIGHT); // the cap was reached: the test bites
  }, 30_000); // real lock files on disk: a slow CI runner can pass the 5 s default

  it("spaces writes by 7 s", async () => {
    const clock = new FakeClock();
    const d = { stateDir: mkdtempSync(join(tmpdir(), "cli-lim-")), sleep: clock.sleep, now: clock.now, pid: process.pid };
    const limiter = new Limiter(d, new StateStore(d, "k"));
    const ats: number[] = [];
    const one = async () => {
      const { release } = await limiter.acquire({ rate: 1, write: true });
      ats.push(clock.now());
      await release();
    };
    await clock.run(Promise.all([one(), one(), one()]));
    expect(ats[1] - ats[0]).toBeGreaterThanOrEqual(WRITE_GAP_MS);
    expect(ats[2] - ats[1]).toBeGreaterThanOrEqual(WRITE_GAP_MS);
  });

  it("frees the slots of a dead process", async () => {
    const clock = new FakeClock();
    const dir = mkdtempSync(join(tmpdir(), "cli-lim-"));
    const d = { stateDir: dir, sleep: clock.sleep, now: clock.now, pid: process.pid };
    const store = new StateStore(d, "k");
    store.write("limiter", { next_at: 0, next_write_at: 0, inflight: Array.from({ length: 4 }, (_, i) => ({ id: `x${i}`, pid: 2 ** 22 + 7, at: clock.now() })) });
    const limiter = new Limiter(d, store);
    const { release } = await clock.run(limiter.acquire({ rate: 5, write: false }));
    await release();
  });

  it("breaks the lock of a dead process", async () => {
    const clock = new FakeClock();
    const dir = mkdtempSync(join(tmpdir(), "cli-lim-"));
    const d = { stateDir: dir, sleep: clock.sleep, now: clock.now, pid: process.pid };
    const store = new StateStore(d, "k");
    writeFileSync(join(dir, "k.lock"), String(2 ** 22 + 7));
    expect(await clock.run(store.withLock(() => 42))).toBe(42);
  });
});

describe("limiter (8 real processes)", () => {
  let dist = "";
  beforeAll(() => {
    dist = mkdtempSync(join(tmpdir(), "cli-dist-"));
    const tsc = join(process.cwd(), "node_modules/typescript/bin/tsc");
    execFileSync(process.execPath, [tsc, "-p", join(process.cwd(), "tsconfig.json"), "--outDir", dist], { stdio: "pipe" });
    writeFileSync(join(dist, "package.json"), '{"type":"module"}');
  }, 60_000);

  it("keeps the same guarantees across processes", async () => {
    const stateDir = mkdtempSync(join(tmpdir(), "cli-xp-"));
    const log = join(stateDir, "log.jsonl");
    writeFileSync(log, "");
    const rate = 10;
    const children = Array.from({ length: 8 }, () =>
      new Promise<void>((resolve, reject) => {
        const c = spawn(process.execPath, [join(process.cwd(), "test/helpers/limiter-child.mjs"), dist, stateDir, log, "2", String(rate), "500"], { stdio: "inherit" });
        c.on("exit", (code) => (code === 0 ? resolve() : reject(new Error(`child exit ${code}`))));
      }),
    );
    await Promise.all(children);
    const rows = readFileSync(log, "utf8").trim().split("\n").map((l) => JSON.parse(l) as { at: number; start: number; end: number });
    expect(rows).toHaveLength(16);
    expect(maxInWindow(rows.map((r) => r.at), 1000)).toBeLessThanOrEqual(rate);
    expect(maxConcurrent(rows)).toBeLessThanOrEqual(MAX_IN_FLIGHT);
  }, 60_000);
});
