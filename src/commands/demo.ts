import type { Command } from "commander";
import { DEMO_APP } from "../core/demo.js";
import { emit } from "../core/output.js";
import { addGlobals, type Runtime } from "../core/runtime.js";
import type { Globals } from "../core/types.js";

const HELP = `
Demo mode answers every read from the built-in sample data of one fictional app (${DEMO_APP}).
It needs no key and sends no request. The data is not real. A write exits 10; --dry-run and --plan still work.
Each run prints demo=on in the stderr header line.

Example:
  export PURCHASELY_DEMO=1
  PREFIX dashboard summary --app ${DEMO_APP} --output table
  unset PURCHASELY_DEMO`;

/** `demo`: how to start demo mode. It sends nothing and needs no key. */
export function register(program: Command, rt: Runtime): void {
  const n = rt.d.name;
  addGlobals(program.command("demo").description("Try the CLI with no key: sample data of a fictional app, for a first test"))
    .addHelpText("after", HELP.replace("PREFIX", n))
    .action((o: Globals) =>
      emit(rt.d, o, {
        demo: `Demo mode answers every read from the sample data of one fictional app (${DEMO_APP}). No key, no request. The data is not real. A write exits 10.`,
        start: "export PURCHASELY_DEMO=1",
        app: DEMO_APP,
        try: [`${n} applications list`, `${n} placements list --app ${DEMO_APP}`, `${n} dashboard summary --app ${DEMO_APP} --output table`],
        stop: "unset PURCHASELY_DEMO",
      }, { env: "prod", key: null, name: "demo" }),
    );
}
