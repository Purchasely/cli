import type { Command } from "commander";
import { AGENTS, aiRun, type Verb } from "../core/ai.js";
import { CliError, EXIT } from "../core/errors.js";
import { emit } from "../core/output.js";
import { addGlobals, type Runtime } from "../core/runtime.js";
import type { Globals } from "../core/types.js";

const MORE: Record<Verb, string> = {
  install: "Agents: claude, codex, copilot, gemini, cursor, opencode (default all). With --dry-run it runs read-only probes and prints the plan. PURCHASELY_OFFLINE=1 blocks it.",
  status: "Per agent: detected, managed by ai install, installed by hand (true, false or unknown), plugin version, how it follows releases, last error. It changes nothing.",
  update: "A copy agent (Cursor, OpenCode) updates only when a newer release exists.",
  uninstall: "A marketplace is never removed.",
};

const VERBS: [Verb, string][] = [
  ["install", "Install the plugin in every coding agent found on this machine"],
  ["update", "Update the plugin in the agents that ai install registered"],
  ["status", "Show the agents, the plugin version and how each one follows releases"],
  ["uninstall", "Remove the plugin from the agents that ai install registered"],
];

export function register(program: Command, rt: Runtime): void {
  const ai = program.command("ai").description("Install the Purchasely AI plugin in your coding agents");
  for (const [verb, description] of VERBS) {
    rt.taken.add(`ai ${verb}`);
    addGlobals(ai.command(verb).description(description))
      .option("--agent <name>", `${AGENTS.join(", ")} or all (default: all)`)
      .addHelpText("after", `\n${MORE[verb]}\n`)
      .action(async (g: Globals & { agent?: string }) => {
        const { rows, errors } = await aiRun(rt, g, verb);
        emit(rt.d, g, { agents: rows }, { env: rt.env(g), key: null, name: `ai-${verb}` });
        if (errors.length) throw new CliError(EXIT.GENERIC, "AI_FAILED", errors);
      });
  }
}
