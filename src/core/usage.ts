import { appendFileSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import type { Deps } from "./types.js";

const DAY = 24 * 3600_000;

export interface UsageLine {
  ts: string;
  env: string;
  surface: string;
  key: string;
  method: string;
  path: string;
  status: number;
  rate_limited: boolean;
  request_id: string;
}

const file = (d: Pick<Deps, "stateDir">) => join(d.stateDir, "usage.jsonl");

export function readUsage(d: Pick<Deps, "stateDir" | "now">): UsageLine[] {
  let raw = "";
  try {
    raw = readFileSync(file(d), "utf8");
  } catch {
    return [];
  }
  const lines: UsageLine[] = [];
  for (const l of raw.split("\n")) {
    try {
      const u = JSON.parse(l) as UsageLine;
      if (d.now() - Date.parse(u.ts) < DAY) lines.push(u);
    } catch {
      // skip a torn line
    }
  }
  return lines;
}

export function logUsage(d: Pick<Deps, "stateDir" | "now">, line: Omit<UsageLine, "ts">): void {
  mkdirSync(d.stateDir, { recursive: true, mode: 0o700 });
  appendFileSync(file(d), JSON.stringify({ ts: new Date(d.now()).toISOString(), ...line }) + "\n", { mode: 0o600 });
  // Prune the 24 h retention when the file grows.
  if (statSync(file(d)).size > 256 * 1024) {
    writeFileSync(file(d), readUsage(d).map((l) => JSON.stringify(l) + "\n").join(""), { mode: 0o600 });
  }
}
