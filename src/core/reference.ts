import type { Command } from "commander";
import type { Runtime } from "./runtime.js";

/** One node of the command tree below the root: a group, a subgroup or a command. `help` is what `--help` prints. */
export interface Leaf { path: string[]; id: string; summary: string; help: string }
/** One top-level command with every node below it, itself first. */
/** `extension`: a top-level command that an extension module added. */
export interface Group { name: string; id: string; description: string; extension: boolean; leaves: Leaf[] }
/** `section`: the heading and the note of the extension commands, which come after the built-in ones. */
export interface Reference { name: string; version: string; host: string; description: string; root: string; groups: Group[]; section?: { title: string; note: string } }

const WIDTH = 100;
// `visibleCommands` drops hidden commands, as `--help` does.
const subs = (c: Command) => c.createHelp().visibleCommands(c).filter((x) => x.name() !== "help");
const cmp = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0);

/** The text of `<command> --help` at a fixed width: the formatted help, then the `addHelpText("after")` texts. */
function helpOf(cmd: Command): string {
  const h = cmd.createHelp();
  h.prepareContext({ error: false, helpWidth: WIDTH, outputHasColors: false });
  let text = h.formatHelp(cmd, h);
  // The typings do not declare `emit`; `addHelpText` only registers a listener for this event.
  (cmd as unknown as { emit(event: string, ctx: object): void }).emit("afterHelp", { error: false, command: cmd, write: (s: string) => void (text += s) });
  return text;
}

const nodes = (cmd: Command, path: string[]): Leaf[] => [
  { path, id: path.join("-"), summary: cmd.description(), help: helpOf(cmd) },
  ...subs(cmd).flatMap((c) => nodes(c, [...path, c.name()])),
];

/**
 * Read the tree it gets, so `purchasely` prints only its own commands. Built-in groups first, then the groups of the
 * extension, each sorted by name, and the nodes of a group sorted by path. Nothing in the tree is changed.
 */
export function collect(program: Command, rt: Runtime): Reference {
  const groups = subs(program).map((top): Group => ({
    name: top.name(),
    id: `g-${top.name()}`,
    description: top.description(),
    extension: rt.extTop.has(top),
    leaves: nodes(top, [top.name()]).sort((a, b) => cmp(a.path.join(" "), b.path.join(" "))),
  }));
  groups.sort((a, b) => Number(a.extension) - Number(b.extension) || cmp(a.name, b.name));
  return { name: rt.d.name, version: rt.d.version, host: rt.d.host, description: program.description(), root: helpOf(program), groups, section: rt.d.ext?.section };
}

/** One visible command with its arguments, options and subcommands: the shape of `reference --format json`, also read by `completion`. */
export interface Node {
  name: string;
  description: string;
  arguments: { name: string; description: string; required: boolean; variadic: boolean }[];
  options: { flags: string; long?: string; description: string; required: boolean; default?: unknown }[];
  commands: Node[];
  extension?: boolean;
}

/** The one walk of the visible tree. Hidden commands and options are left out, and so is `--help` on every node. */
export function tree(cmd: Command): Node {
  return {
    name: cmd.name(),
    description: cmd.description(),
    arguments: cmd.registeredArguments.map((a) => ({ name: a.name(), description: a.description, required: a.required, variadic: a.variadic })),
    options: cmd.createHelp().visibleOptions(cmd).filter((o) => o.long !== "--help").map((o) => ({
      flags: o.flags, long: o.long, description: o.description, required: o.mandatory, ...(o.defaultValue === undefined ? {} : { default: o.defaultValue }),
    })),
    commands: subs(cmd).map(tree),
  };
}

/** The whole tree for agents. Built-in groups first, then the groups of the extension, each in the order of the help. */
export function json(program: Command, rt: Runtime) {
  const commands = subs(program).sort((a, b) => Number(rt.extTop.has(a)) - Number(rt.extTop.has(b))).map((c) => ({ ...tree(c), ...(rt.d.ext ? { extension: rt.extTop.has(c) } : {}) }));
  return { ...tree(program), name: rt.d.name, version: rt.d.version, commands };
}

const trim = (s: string) => s.replace(/\n+$/, "");
const fence = (s: string) => "```text\n" + trim(s) + "\n```";

/** No version and no date, so the committed file only changes when a command changes. */
export function markdown(r: Reference): string {
  const parts = [`# ${r.name} command reference`, fence(r.root)];
  let ext = false;
  for (const g of r.groups) {
    if (g.extension && !ext && r.section) {
      ext = true;
      parts.push(`## ${r.section.title}`, r.section.note);
    }
    parts.push(`## ${g.name}`, g.description);
    for (const l of g.leaves) parts.push(`### \`${r.name} ${l.path.join(" ")}\``, fence(l.help));
  }
  return parts.join("\n\n") + "\n";
}

/** roff: `\` is `\e`, `-` is `\-`, a non-ASCII character is `\[uXXXX]`, a line that starts with `.` or `'` gets `\&`. */
const roff = (s: string) =>
  s.replace(/\\/g, "\\e").replace(/-/g, "\\-").replace(/[^\x00-\x7f]/gu, (c) => `\\[u${c.codePointAt(0)!.toString(16).toUpperCase().padStart(4, "0")}]`);
const block = (s: string) => [".nf", ...trim(s).split("\n").map((l) => roff(l).trimEnd()).map((l) => (/^[.']/.test(l) ? `\\&${l}` : l === "" ? "\\&" : l)), ".fi"];

/** `date` is the pack date (YYYY-MM-DD). */
export function man(r: Reference, date: string): string {
  const title = r.name.toUpperCase();
  const section = (heading: string, groups: Group[]) => [
    `.SH ${heading}`,
    ...groups.flatMap((g) => g.leaves.flatMap((l) => [`.SS "${roff(`${r.name} ${l.path.join(" ")}`)}"`, ...block(l.help)])),
  ];
  const ext = r.groups.filter((g) => g.extension);
  const lines = [
    `.TH ${title} 1 ${date} "${r.name} ${r.version}" "Purchasely CLI"`,
    ".SH NAME",
    `${r.name} \\- ${roff(r.description)}`,
    ".SH SYNOPSIS",
    `.B ${r.name}`,
    "<command> [options]",
    ".SH DESCRIPTION",
    ...block(r.root),
    ...section("COMMANDS", r.groups.filter((g) => !g.extension)),
    ...(ext.length ? section(`"${(r.section?.title ?? "More commands").toUpperCase()}"`, ext) : []),
    ...(r.host ? [`.SH "SEE ALSO"`, roff(`${r.host}/reference`)] : []),
  ];
  return lines.join("\n") + "\n";
}
