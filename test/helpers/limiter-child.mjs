// Child process for the cross-process limiter test. Uses the compiled sources.
import { appendFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

const [dist, stateDir, log, count, rate, hold] = process.argv.slice(2);
const { StateStore } = await import(pathToFileURL(`${dist}/core/state.js`));
const { Limiter } = await import(pathToFileURL(`${dist}/core/limiter.js`));
const d = { stateDir, pid: process.pid, now: Date.now, sleep: (ms) => new Promise((r) => setTimeout(r, ms)) };
const limiter = new Limiter(d, new StateStore(d, "shared"));
for (let i = 0; i < Number(count); i++) {
  const { at, release } = await limiter.acquire({ rate: Number(rate), write: false });
  const start = Date.now();
  await d.sleep(Number(hold));
  appendFileSync(log, JSON.stringify({ at, start, end: Date.now() }) + "\n");
  await release();
}
