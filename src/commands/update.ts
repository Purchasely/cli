import { join } from "node:path";
import { Command, Option } from "commander";
import { updatePlugins } from "../core/ai.js";
import { CliError, EXIT } from "../core/errors.js";
import { emit } from "../core/output.js";
import { addGlobals, type Runtime } from "../core/runtime.js";
import type { Globals } from "../core/types.js";
import { DAY, installKind, latestVersion, mergeRecord, newer, noticeLine, readRecord, runInstaller, updateStore } from "../core/update.js";

export function register(program: Command, rt: Runtime): void {
  const { d } = rt;
  rt.taken.add("update");

  // The opt-outs do not apply here: an explicit request always works.
  async function check(g: Globals, background: boolean): Promise<void> {
    const kind = installKind(d.env, import.meta.url);
    const s = updateStore(d);
    if (background) {
      // Reserve the day first, so two children never both check.
      const go = await s.withLock(() => {
        if (d.now() - (readRecord(s).checked_at ?? 0) < DAY) return false;
        mergeRecord(s, { checked_at: d.now() });
        return true;
      });
      if (!go) return;
    }
    const latest = await latestVersion(d);
    if (latest) await s.withLock(() => mergeRecord(s, { latest, checked_at: d.now() }));
    let action: "updated" | "up_to_date" | "notice" = "up_to_date";
    if (!latest) {
      if (!background) d.stderr("No stable release found, nothing to compare with.\n");
    } else if (newer(latest, d.version)) {
      if (kind === "managed") {
        const r = await runInstaller(d, d.env.PURCHASELY_MANAGED_INSTALL_ROOT!, latest, background ? join(d.stateDir, `update.${d.name}.log`) : undefined);
        if (!background) d.stderr(r.stderr);
        if (r.code !== 0) throw new CliError(EXIT.GENERIC, "UPDATE_FAILED", r.stderr.trim().split("\n").slice(-5));
        action = "updated";
      } else {
        if (!background) d.stderr(noticeLine(d, kind, latest) + "\n");
        action = "notice";
      }
    }
    if (!background) emit(d, g, { kind, current: d.version, latest, action }, { env: rt.env(g), key: null, name: "update" });
  }

  addGlobals(program.command("update").description("Check for a new version and install it (managed install) or print the upgrade command"))
    .addOption(new Option("--background").hideHelp())
    .addHelpText("after", "\nA managed install installs the new version. Another install prints the upgrade command. See docs/install.md.\n")
    .action(async (g: Globals & { background?: boolean }) => {
      if (!g.background) {
        await check(g, false);
        return updatePlugins(rt, { background: false });
      }
      // the detached child is silent and always exits 0; the plugin step runs even when the CLI step fails
      try {
        await check(g, true);
      } catch {
        // see above
      }
      try {
        await updatePlugins(rt, { background: true });
      } catch {
        // see above
      }
    });
}
