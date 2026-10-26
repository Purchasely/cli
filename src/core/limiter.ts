import { randomUUID } from "node:crypto";
import type { Deps } from "./types.js";
import type { StateStore } from "./state.js";

export const MAX_IN_FLIGHT = 4;
export const WRITE_GAP_MS = 7000;
const STALE_SLOT_MS = 5 * 60_000;

interface LimiterState {
  next_at: number;
  next_write_at: number;
  inflight: { id: string; pid: number; at: number }[];
}

const alive = (pid: number) => {
  try {
    process.kill(pid, 0);
    return true;
  } catch (e) {
    return (e as NodeJS.ErrnoException).code === "EPERM";
  }
};

/**
 * Cross-process limiter. Each caller reserves the next send time (spacing of 1/rate), so no
 * window of 1 s holds more than `rate` sends. Up to 4 requests may be in flight at once.
 */
export class Limiter {
  constructor(private d: Pick<Deps, "now" | "sleep" | "pid">, private store: StateStore) {}

  private wait = async (until: number) => {
    for (let n = this.d.now(); n < until; n = this.d.now()) await this.d.sleep(until - n);
  };

  /** `at` is the reserved send time. Call `release` when the response (or the failure) is known. */
  async acquire(opts: { rate: number; write: boolean }): Promise<{ at: number; release: () => Promise<void> }> {
    const { store, d } = this;
    const fresh = (): LimiterState => ({ next_at: 0, next_write_at: 0, inflight: [] });

    if (opts.write) {
      const slot = await store.withLock(() => {
        const s = store.read("limiter", fresh());
        const at = Math.max(d.now(), s.next_write_at);
        s.next_write_at = at + WRITE_GAP_MS;
        store.write("limiter", s);
        return at;
      });
      await this.wait(slot);
    }

    const id = randomUUID();
    for (;;) {
      const got = await store.withLock(() => {
        const s = store.read("limiter", fresh());
        s.inflight = s.inflight.filter((i) => d.now() - i.at < STALE_SLOT_MS && (i.pid === d.pid || alive(i.pid)));
        if (s.inflight.length >= MAX_IN_FLIGHT) {
          store.write("limiter", s);
          return false;
        }
        s.inflight.push({ id, pid: d.pid, at: d.now() });
        store.write("limiter", s);
        return true;
      });
      if (got) break;
      await d.sleep(25);
    }

    const slot = await store.withLock(() => {
      const s = store.read("limiter", fresh());
      const at = Math.max(d.now(), s.next_at);
      s.next_at = at + Math.ceil(1000 / opts.rate);
      store.write("limiter", s);
      return at;
    });
    await this.wait(slot);

    const release = () =>
      store.withLock(() => {
        const s = store.read("limiter", fresh());
        s.inflight = s.inflight.filter((i) => i.id !== id);
        store.write("limiter", s);
      });
    return { at: slot, release };
  }
}
