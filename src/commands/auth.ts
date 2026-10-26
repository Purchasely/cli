import { Command } from "commander";
import { writeConfig } from "../core/config.js";
import { credentialDisplay, hint, removeCredential, storeCredential, what, type Surface } from "../core/credentials.js";
import { CliError, EXIT, usage } from "../core/errors.js";
import { addGlobals, type Runtime } from "../core/runtime.js";
import type { Env, Globals } from "../core/types.js";

/**
 * The key prompt, the optional check, the store and the identity email. `check` runs before anything is stored,
 * so a rejected key leaves no trace. Used by `auth login` (Management) and by the login of an extension surface.
 */
export async function loginWithKey(
  rt: Runtime, o: Globals & { apiKey?: string; email?: string }, env: Env, surface: Surface,
  check?: (key: string, email: string) => Promise<void>,
): Promise<void> {
  const { d } = rt;
  let key = o.apiKey;
  if (!key) key = (d.isTTY ? await d.prompt(`${what(surface)}: `, { hidden: true }) : await d.readStdin()).trim();
  if (!key) throw usage(`No ${what(surface)}. Pass --api-key, type it at the prompt, or pipe it on stdin.`);
  // `purchasely` never asks for, stores or sends an email. Only an extension with `identity` does.
  let email = "";
  if (d.ext?.identity) {
    email = o.email ?? (d.isTTY ? (await d.prompt(`Your email [${rt.cfg.identity_email ?? ""}]: `)).trim() : "") ?? "";
    email = email || rt.cfg.identity_email || "";
    if (email && !email.includes("@")) throw usage("The email is not valid.");
  }
  await check?.(key, email);
  const source = await storeCredential(d, env, surface, key);
  if (d.ext?.identity) {
    rt.cfg = { ...rt.cfg, identity_email: email || undefined };
    writeConfig(d.configDir, rt.cfg);
  }
  d.stdout(JSON.stringify({ stored: true, env, surface, source, key: credentialDisplay(surface, key), identity: email || null }) + "\n");
}

/** The public auth commands: the Management API key only. */
export function register(program: Command, rt: Runtime): void {
  const { d } = rt;
  const auth = program.command("auth").description("Manage the Management API key");
  rt.taken.add("auth login").add("auth status").add("auth logout");

  // No read endpoint works for every Management scope, so the key is not checked at login.
  const login = addGlobals(auth.command("login").description("Store the Management API key"))
    .option("--api-key <key>", "the key (visible in the shell history: prefer the prompt or stdin)")
    .addHelpText("after", "\nThe key comes from the hidden prompt, from --api-key or from stdin. It is not checked against the API: no read endpoint works for every Management scope.\n");
  if (d.ext?.identity) login.option("--email <email>", "your email, sent in the User-Agent");
  login.action((o: Globals & { apiKey?: string; email?: string }) => loginWithKey(rt, o, rt.env(o), "management"));

  addGlobals(auth.command("status").description("Show where the credential comes from"))
    .addHelpText("after", "\nPrints the source of the credential (env, keychain or file), its last 4 characters and the identity. Exit 3 when there is no key.\n")
    .action(async (o: Globals) => {
      const env = rt.env(o);
      const cred = await rt.credential(env, "management");
      // Throw before the write: with no key the error JSON is the only output.
      if (!cred) throw new CliError(EXIT.AUTH, "UNAUTHORIZED", [`No ${what("management")}. ${hint("management", d.name)}`]);
      d.stdout(JSON.stringify({ env, surface: "management", source: cred.source, key: credentialDisplay("management", cred.key), identity: d.ext?.identity ? rt.cfg.identity_email ?? null : null }) + "\n");
    });

  addGlobals(auth.command("logout").description("Delete the stored credential"))
    .addHelpText("after", "\nDeletes the key stored in the keychain or in the file. A key in the environment is not touched.\n")
    .action(async (o: Globals) => {
      const env = rt.env(o);
      await removeCredential(d, env, "management");
      d.stdout(JSON.stringify({ removed: true, env, surface: "management" }) + "\n");
    });
}
