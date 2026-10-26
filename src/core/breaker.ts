import { CliError, EXIT } from "./errors.js";
import type { Deps } from "./types.js";
import type { StateStore } from "./state.js";

const WINDOW_MS = 5 * 60_000;
const RESTORE_MS = 10 * 60_000;
const MAX_REJECTIONS = 20;
const MIN_FACTOR = 1 / 8;
export const FALLBACK_PAUSE_MS = 60_000;

export interface BreakerState {
  paused_until: number;
  rate_factor: number;
  last_429_at: number;
  rejections: number[];
}

const fresh = (): BreakerState => ({ paused_until: 0, rate_factor: 1, last_429_at: 0, rejections: [] });

/** Pause in ms from `Retry-After` (seconds), else `X-RateLimit-Reset` (unix seconds), else 60 s. */
export function pauseFromHeaders(h: Headers, now: number): number {
  const ra = Number(h.get("retry-after"));
  if (h.get("retry-after") && ra > 0) return Math.min(ra * 1000, 15 * 60_000);
  const reset = Number(h.get("x-ratelimit-reset"));
  if (h.get("x-ratelimit-reset") && reset * 1000 > now) return Math.min(reset * 1000 - now, 15 * 60_000);
  return FALLBACK_PAUSE_MS;
}

export class Breaker {
  constructor(private d: Pick<Deps, "now" | "sleep">, private store: StateStore) {}

  /** Current state, with the rate factor restored after 10 min without a 429. Read only. */
  state(): BreakerState {
    const s = { ...fresh(), ...this.store.read<Partial<BreakerState>>("breaker", {}) };
    const now = this.d.now();
    if (s.rate_factor < 1 && now - s.last_429_at > RESTORE_MS) s.rate_factor = 1;
    s.rejections = s.rejections.filter((t) => now - t < WINDOW_MS);
    return s;
  }

  /** Throw when the breaker stopped the run, else wait until every process may send again. */
  async gate(): Promise<number> {
    for (;;) {
      const s = this.state();
      if (s.rejections.length >= MAX_REJECTIONS) throw this.stopError();
      const wait = s.paused_until - this.d.now();
      if (wait <= 0) return s.rate_factor;
      await this.d.sleep(wait);
    }
  }

  /** Record a 429. Returns the pause end. Throws exit 7 at 20 rejections in 5 min. */
  async record(pauseMs: number): Promise<number> {
    const r = await this.store.withLock(() => {
      const s = this.state();
      const now = this.d.now();
      s.rejections.push(now);
      // Halve once per pause, not once per rejected caller.
      if (now >= s.paused_until) s.rate_factor = Math.max(MIN_FACTOR, s.rate_factor / 2);
      s.paused_until = Math.max(s.paused_until, now + pauseMs);
      s.last_429_at = now;
      this.store.write("breaker", s);
      return s;
    });
    if (r.rejections.length >= MAX_REJECTIONS) throw this.stopError();
    return r.paused_until;
  }

  private stopError() {
    return new CliError(EXIT.RATE_LIMITED, "RATE_LIMIT_STOP", [
      "The server rejected 20 requests in 5 minutes. The CLI stopped to protect the API.",
      "Wait 5 minutes. Run `purchasely usage` to see the state. Stop other scripts that use the same key.",
    ]);
  }
}
