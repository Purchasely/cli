import type { Command, Op, Param, Schema } from "./spec.js";
import { kebab } from "./naming.js";

/** Spec texts use em-dashes and en-dashes. Help text must not. */
export const plain = (s: string) => s.replace(/\s*[\u2013\u2014]\s*/g, ", ");

export const typeLabel = (s: Schema): string => {
  if (s.enum) return `enum: ${s.enum.join("|")}`;
  if (s.type === "array") return `array of ${typeLabel(s.items ?? {})}`;
  if (s.type) return s.type;
  if (s.anyOf) return s.anyOf.map(typeLabel).join(" or ");
  return "any";
};

export const flagOf = (p: Param) => (p.name === "application_id" ? "--app" : `--${kebab(p.name)}`);

/** Required flags of one variant, as a usage string. */
export function requiredFlags(op: Op): string {
  const parts = op.params
    .filter((p) => p.required && p.name !== "confirm_token")
    .map((p) => `${flagOf(p)} ${p.name === "application_id" ? "app_xxx" : `<${kebab(p.name)}>`}`);
  if (op.body?.required) parts.push("--data @body.json");
  return parts.join(" ");
}

export const usageString = (c: Command, name = "purchasely"): string => {
  const op = (c.appOp ?? c.globalOp)!;
  const env = op.method === "GET" ? "" : " --env prod";
  return `${name} ${c.resource} ${c.action} ${requiredFlags(op)}${env}`.replace(/\s+/g, " ").trim();
};

export function helpText(c: Command, name = "purchasely"): string {
  const lines: string[] = [""];
  const ops = [c.appOp, c.globalOp].filter((o): o is Op => !!o);
  const first = ops[0];
  lines.push(plain(first.description.replace(/\n+\*\*Required scope:\*\*.*$/s, "").trim()), "");
  lines.push(`Required scope: ${first.scope}`);
  lines.push("Request:");
  for (const o of ops) lines.push(`  ${o.method} ${o.path}${ops.length > 1 ? (o.app ? "  (with --app)" : "  (without --app)") : ""}`);
  if (first.method !== "GET") lines.push("", "A write needs --env prod or --env staging. Run it with --dry-run first.");
  if (first.body) {
    lines.push("", `Body (${first.body.required ? "required" : "optional"}): --data '<json>' or --data @file.json or --data -`);
    const req = first.body.schema.required ?? [];
    for (const [name, s] of Object.entries(first.body.schema.properties ?? {})) {
      lines.push(`  ${name} (${typeLabel(s)}${req.includes(name) ? ", required" : ""}) ${plain((s.description ?? "").replace(/\s+/g, " "))}`.trimEnd());
    }
  }
  const confirmOp = ops.find((o) => o.confirm);
  if (confirmOp) {
    lines.push(
      "",
      `Confirmation flow: a risky change first returns the impact and a confirm_token (${confirmOp.confirm}).`,
      "  Add --yes to confirm at once, answer the prompt, or resend with --confirm-token <token>.",
    );
  }
  lines.push("", `Example:\n  ${usageString(c, name)}`);
  return lines.join("\n");
}
