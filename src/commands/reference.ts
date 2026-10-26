import { Command } from "commander";
import { usage } from "../core/errors.js";
import { collect, json, man, markdown } from "../core/reference.js";
import { html } from "../core/reference-html.js";
import type { Runtime } from "../core/runtime.js";

/** No global options: `--out` and `--output` are refused, not ignored. Nothing here reads a credential or the network. */
export function register(program: Command, rt: Runtime): void {
  rt.taken.add("reference");
  program.command("reference")
    .description("Print the full command reference of this version (Markdown, man, HTML or JSON)")
    .option("--format <format>", "markdown (default), man, html or json", "markdown")
    .addHelpText("after", "\n--format json prints the tree of commands, arguments and options as JSON, for agents. Hidden commands are left out.\n")
    .action((opts: { format: string }) => {
      if (!["markdown", "man", "html", "json"].includes(opts.format)) throw usage("--format must be markdown, man, html or json.");
      if (opts.format === "json") return rt.d.stdout(JSON.stringify(json(program, rt), null, 2) + "\n");
      const ref = collect(program, rt);
      rt.d.stdout(opts.format === "man" ? man(ref, new Date(rt.d.now()).toISOString().slice(0, 10)) : opts.format === "html" ? html(ref) : markdown(ref));
    });
}
