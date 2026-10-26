import { closeSync, mkdirSync, openSync, readFileSync, renameSync, statSync, unlinkSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import type { Deps } from "./types.js";

type StateDeps = Pick<Deps, "stateDir" | "sleep" | "pid">;

const alive = (pid: number) => {
  try {
    process.kill(pid, 0);
    return true;
  } catch (e) {
    return (e as NodeJS.ErrnoException).code === "EPERM";
  }
};

/**
 * Small JSON state files shared by every process of the user for one key.
 * One lock file (O_EXCL), a dead-pid check, a timeout. The critical section may be async: the lock
 * is held until the callback settles. A live holder is never stolen.
 */
export class StateStore {
  constructor(private d: StateDeps, private key: string) {
    mkdirSync(d.stateDir, { recursive: true, mode: 0o700 });
  }
  private path = (name: string) => join(this.d.stateDir, `${this.key}.${name}`);

  read<T>(name: string, fallback: T): T {
    try {
      return JSON.parse(readFileSync(this.path(`${name}.json`), "utf8")) as T;
    } catch {
      return fallback;
    }
  }
  write(name: string, data: unknown): void {
    const tmp = this.path(`${name}.${this.d.pid}.tmp`);
    writeFileSync(tmp, JSON.stringify(data), { mode: 0o600 });
    renameSync(tmp, this.path(`${name}.json`));
  }

  private tryLock(lock: string): boolean {
    try {
      const fd = openSync(lock, "wx", 0o600);
      writeFileSync(fd, String(this.d.pid));
      closeSync(fd);
      return true;
    } catch (e) {
      if ((e as NodeJS.ErrnoException).code !== "EEXIST") throw e;
      try {
        const pid = Number(readFileSync(lock, "utf8"));
        // A lock with no readable pid is a crash between open and write: stale after 10 s.
        const stale = pid ? !alive(pid) : Date.now() - statSync(lock).mtimeMs > 10_000;
        if (stale) unlinkSync(lock);
      } catch {
        // the holder just released it
      }
      return false;
    }
  }

  async withLock<T>(fn: () => T | Promise<T>, timeoutMs = 5000): Promise<T> {
    const lock = this.path("lock");
    const start = Date.now();
    while (!this.tryLock(lock)) {
      if (Date.now() - start > timeoutMs) throw new Error("state lock timeout");
      await this.d.sleep(10);
    }
    try {
      return await fn();
    } finally {
      try { unlinkSync(lock); } catch { /* already removed */ }
    }
  }
}
