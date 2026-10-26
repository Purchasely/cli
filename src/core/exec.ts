import { execFile, spawn } from "node:child_process";
import type { Deps } from "./types.js";

const KILL_GRACE_MS = 2000;

/** Real `Deps.exec`: no shell, a timeout (SIGTERM, then SIGKILL after a grace), stdin from `input`. A missing binary rejects (ENOENT). */
export const realExec: Deps["exec"] = (file, args, o) =>
  new Promise((resolve, reject) => {
    if (o.detach) {
      const child = spawn(file, args, { cwd: o.cwd, detached: true, stdio: "ignore" });
      child.on("error", () => undefined);
      child.unref();
      return resolve({ code: 0, stdout: "", stderr: "", timedOut: false });
    }
    const child = execFile(file, args, { cwd: o.cwd, timeout: o.timeoutMs, maxBuffer: 256 * 1024 * 1024, encoding: "utf8" }, (err, stdout, stderr) => {
      clearTimeout(hard);
      const e = err as (NodeJS.ErrnoException & { killed?: boolean }) | null;
      if (e && typeof e.code === "string" && !e.killed) return reject(e);
      resolve({ code: e ? (typeof e.code === "number" ? e.code : 1) : 0, stdout, stderr, timedOut: !!e?.killed });
    });
    // The soft timeout sends SIGTERM, which a child can ignore: then the callback never fires.
    const hard = o.timeoutMs > 0 ? setTimeout(() => child.kill("SIGKILL"), o.timeoutMs + KILL_GRACE_MS) : undefined; // 0 means no timeout
    child.stdin?.on("error", () => undefined); // the child may exit before it reads `input`
    child.stdin?.end(o.input);
  });
