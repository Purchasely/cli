import { Command } from "commander";
import { usage } from "../core/errors.js";
import { emit } from "../core/output.js";
import { addGlobals, type Runtime } from "../core/runtime.js";

const STOP = new Set(["the", "a", "an", "of", "to", "for", "in", "on", "and", "or", "my", "all", "me", "with", "by", "from", "is", "i", "want", "how", "do"]);

export const tokens = (s: string): string[] =>
  s.toLowerCase().split(/[^a-z0-9]+/).filter((t) => t.length > 1 && !STOP.has(t))
    .map((t) => (t.endsWith("ies") ? t.slice(0, -3) + "y" : t.length > 3 ? t.replace(/s$/, "") : t));

export function register(program: Command, rt: Runtime): void {
  rt.taken.add("search");
  addGlobals(program.command("search").description("Find a command for a task").argument("<task>", "what you want to do"))
    .option("--limit <n>", "number of results", "10")
    .addHelpText("after", `\nPrints a JSON array, best match first. It needs no credential.\nExample:\n  ${rt.d.name} search "list the placements of an app"\n`)
    .action(async (task: string, opts: { limit: string; output?: string; env?: string }) => {
      const limit = Number(opts.limit);
      if (!Number.isInteger(limit) || limit < 1) throw usage("--limit must be a positive integer.");
      const q = tokens(task);
      // Entries from extension modules. Each provider reads a cache and sends nothing.
      const extra = (await Promise.all(rt.searchExtra.map((f) => f(rt.env(opts))))).flat();
      // Hand-written commands come from the commander tree, so a new one is found with no change here.
      const index = [...rt.searchIndex];
      // `visibleCommands` drops a hidden command, as --help does.
      const subs = (cmd: Command) => program.createHelp().visibleCommands(cmd).filter((c) => c.name() !== "help");
      const walk = (cmd: Command, path: string[]): void => {
        if (subs(cmd).length) return subs(cmd).forEach((c) => walk(c, [...path, c.name()]));
        const [resource, ...rest] = path, action = rest.join(" ");
        if (index.some((e) => e.resource === resource && e.action === action)) return;
        index.push({ command: `${program.name()} ${path.join(" ")}`, resource, action, summary: cmd.description(), description: cmd.description(), method: "-", path: "-", scope: "cli" });
      };
      subs(program).forEach((c) => walk(c, [c.name()]));
      const scored = [...index, ...extra].map((e) => {
        const weighted: [Set<string>, number][] = [
          [new Set(tokens(e.resource)), 3], [new Set(tokens(e.action)), 3],
          [new Set(tokens(e.summary)), 2], [new Set(tokens(e.description)), 1], [new Set(tokens(e.scope)), 1],
        ];
        const score = q.reduce((sum, t) => sum + weighted.reduce((s, [set, w]) => s + (set.has(t) ? w : 0), 0), 0);
        return { e, score };
      });
      const top = scored.filter((s) => s.score > 0).sort((a, b) => b.score - a.score).slice(0, limit)
        .map(({ e }) => ({ command: e.command, summary: e.summary, method: e.method, path: e.path, scope: e.scope }));
      emit(rt.d, { ...opts }, top, { env: "prod", key: null, name: "search" });
    });
}
