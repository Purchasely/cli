import type { Command } from "commander";
import { offline } from "../core/ai.js";
import { credentialDisplay } from "../core/credentials.js";
import { CliError, EXIT } from "../core/errors.js";
import { send } from "../core/http.js";
import { emit } from "../core/output.js";
import { addGlobals, type Runtime } from "../core/runtime.js";
import { StateStore } from "../core/state.js";
import { BASE_URLS, type Globals } from "../core/types.js";
import { autoUpdateOff, installKind, latestVersion, newer } from "../core/update.js";

type Status = "ok" | "warn" | "fail";
interface Check { check: string; status: Status; detail: string }

const HELP = `
Checks the install, the key, the network, the version and the AI plugin. It sends one read request.
It never prints the key. Exit 1 when a check fails. Send the output to support when you ask for help.

Examples:
  purchasely doctor
  purchasely doctor --output table`;

export function register(program: Command, rt: Runtime): void {
  addGlobals(program.command("doctor").description("Check the install, the key, the network and the version"))
    .addHelpText("after", HELP.replaceAll("purchasely ", `${rt.d.name} `))
    .action(async (o: Globals) => {
      const { d } = rt;
      const checks: Check[] = [];
      const add = (check: string, status: Status, detail: string) => void checks.push({ check, status, detail });
      const env = rt.env(o);

      const node = Number(process.versions.node.split(".")[0]);
      add("node", node >= 20 ? "ok" : "fail", `Node ${process.versions.node}${node >= 20 ? "" : ": the CLI needs Node 20 or newer"}`);

      const kind = installKind(d.env, import.meta.url);
      add("install", "ok", `${kind} install, automatic update ${autoUpdateOff(d.env, rt.cfg) || kind !== "managed" ? "off" : "on"}`);

      if (offline(d)) add("version", "warn", `${d.version}. PURCHASELY_OFFLINE=1 is set: no check of the latest version`);
      else {
        try {
          const latest = await latestVersion(d);
          if (latest && newer(latest, d.version)) add("version", "warn", `${d.version}, and ${latest} is available. Run \`${d.name} update\``);
          else add("version", "ok", `${d.version}, the latest`);
        } catch {
          add("version", "warn", `${d.version}. The latest version is not available: check the network`);
        }
      }

      const ctx = await rt.ctx(o, "management");
      const cred = await rt.credential(env, "management");
      if (!cred) add("key", "fail", `No Management API key for ${env}. Run \`${d.name} auth login\` or set PURCHASELY_API_KEY`);
      else add("key", "ok", `${env} key ${credentialDisplay("management", cred.key)} from ${cred.source}`);

      if (!cred) add("api", "warn", "Not checked: no key");
      else if (offline(d)) add("api", "warn", "Not checked: PURCHASELY_OFFLINE=1 is set");
      else {
        try {
          const path = "/management/v1/applications";
          const res = await send(ctx, { method: "GET", url: `${BASE_URLS[env]}${path}?per_page=1`, pathTemplate: path });
          if (res.status >= 200 && res.status < 300) add("api", "ok", `The Management API of ${env} accepts the key`);
          else if (res.status === 401) add("api", "fail", `The Management API of ${env} refuses the key (HTTP 401). Check the key and the env`);
          else add("api", "fail", `The Management API of ${env} answered HTTP ${res.status}`);
        } catch (e) {
          add("api", "fail", e instanceof CliError ? e.messages.join(" ") : "The request failed");
        }
      }

      const plugin = new StateStore(d, "ai").read<{ agents?: Record<string, unknown> } | null>("plugin", null);
      const agents = Object.keys(plugin?.agents ?? {});
      add("ai plugin", agents.length ? "ok" : "warn", agents.length ? `Installed for ${agents.join(", ")}` : `Not installed with the CLI. Run \`${d.name} ai install\``);

      const allow = rt.cfg.write_allowlist?.[env];
      add("writes", "ok", `${rt.readOnly() ? "Read-only mode" : "Writes allowed"}${allow ? `, write_allowlist.${env}: ${allow.join(", ") || "empty"}` : ""}`);

      emit(d, o, { checks }, { env, key: cred?.key ?? null, name: "doctor", columns: ["check", "status", "detail"] });
      if (checks.some((c) => c.status === "fail")) throw new CliError(EXIT.GENERIC, "DOCTOR_FAILED", checks.filter((c) => c.status === "fail").map((c) => `${c.check}: ${c.detail}`));
    });
}
