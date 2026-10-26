import { Argument, type Command } from "commander";
import { tree, type Node } from "../core/reference.js";
import type { Runtime } from "../core/runtime.js";

const SHELLS = ["bash", "zsh", "fish"] as const;
type Shell = (typeof SHELLS)[number];

/** One entry per node of the visible tree: its path, its subcommands and its long options (`--help` is added here). */
interface Entry { path: string; subs: Node[]; opts: { name: string; description: string }[] }
const entries = (n: Node, path: string[] = []): Entry[] => [
  { path: path.join(" "), subs: n.commands, opts: [...n.options.flatMap((o) => (o.long ? [{ name: o.long, description: o.description }] : [])), { name: "--help", description: "display help for the command" }] },
  ...n.commands.flatMap((c) => entries(c, [...path, c.name])),
];

const oneLine = (s: string) => s.replace(/\s+/g, " ").trim();
/** Quote for bash and zsh single quotes. */
const sq = (s: string) => `'${oneLine(s).replace(/'/g, `'\\''`)}'`;
/** Quote for fish single quotes, where only `\` and `'` are special. */
const fq = (s: string) => `'${oneLine(s).replace(/[\\']/g, "\\$&")}'`;

function bash(name: string, es: Entry[]): string {
  const cases = (words: (e: Entry) => string[]) => es.map((e) => `    ${sq(e.path)}) echo ${words(e).map(sq).join(" ")} ;;`).join("\n");
  return `# bash completion for ${name}. Add to ~/.bashrc: source <(${name} completion bash)
_${name}_subs() {
  case "$1" in
${cases((e) => e.subs.map((s) => s.name))}
  esac
}
_${name}_opts() {
  case "$1" in
${cases((e) => e.opts.map((o) => o.name))}
  esac
}
_${name}_complete() {
  local cur="\${COMP_WORDS[COMP_CWORD]}" path="" i w
  for ((i = 1; i < COMP_CWORD; i++)); do
    w="\${COMP_WORDS[i]}"
    case " $(_${name}_subs "$path") " in *" $w "*) path="\${path:+$path }$w" ;; esac
  done
  COMPREPLY=($(compgen -W "$(_${name}_subs "$path") $(_${name}_opts "$path")" -- "$cur"))
}
complete -F _${name}_complete ${name}
`;
}

function zsh(name: string, es: Entry[]): string {
  const list = (items: string[]) => `(${items.join(" ")})`;
  const cases = es.map((e) => `    ${sq(e.path)})
      names=${list(e.subs.map((s) => sq(s.name)))}
      subs=${list(e.subs.map((s) => sq(`${s.name}:${s.description}`)))}
      opts=${list(e.opts.map((o) => sq(`${o.name}:${o.description}`)))} ;;`).join("\n");
  return `#compdef ${name}
# zsh completion for ${name}. Add to ~/.zshrc, after compinit: source <(${name} completion zsh)
_${name}_node() {
  case "$1" in
${cases}
  esac
}
_${name}() {
  local cpath="" w i
  local -a names subs opts
  _${name}_node ""
  for ((i = 2; i < CURRENT; i++)); do
    w=$words[i]
    if (( \${names[(Ie)$w]} )); then
      cpath="\${cpath:+$cpath }$w"
      _${name}_node "$cpath"
    fi
  done
  _describe -t commands 'command' subs
  _describe -t options 'option' opts
}
if [ "$funcstack[1]" = "_${name}" ]; then _${name} "$@"; else compdef _${name} ${name}; fi
`;
}

function fish(name: string, es: Entry[]): string {
  const subs = es.filter((e) => e.subs.length).map((e) => `    case ${fq(e.path)}\n      printf '%s\\n' ${e.subs.map((s) => fq(s.name)).join(" ")}`).join("\n");
  const lines = es.flatMap((e) => [
    ...e.subs.map((s) => `complete -c ${name} -n '__${name}_at ${e.path}' -a ${fq(s.name)} -d ${fq(s.description)}`),
    ...e.opts.map((o) => `complete -c ${name} -n '__${name}_at ${e.path}' -l ${o.name.slice(2)} -d ${fq(o.description)}`),
  ]);
  return `# fish completion for ${name}. Add to ~/.config/fish/config.fish: ${name} completion fish | source
function __${name}_subs
  switch "$argv"
${subs}
  end
end
function __${name}_at
  set -l path
  for w in (commandline -opc)[2..-1]
    if contains -- $w (__${name}_subs $path)
      set path $path $w
    end
  end
  test "$path" = "$argv"
end
complete -c ${name} -f
${lines.join("\n")}
`;
}

const SCRIPTS: Record<Shell, (name: string, es: Entry[]) => string> = { bash, zsh, fish };

/** `completion <shell>`: prints a script for the visible command tree. It installs nothing. */
export function register(program: Command, rt: Runtime): void {
  const { d } = rt;
  program.command("completion")
    .description("Print a shell completion script (bash, zsh or fish)")
    .addArgument(new Argument("<shell>", "the shell of the script").choices(SHELLS))
    .addHelpText("after", `
It prints the script and installs nothing. Put one line in the startup file of the shell:
  bash  source <(${d.name} completion bash)     in ~/.bashrc
  zsh   source <(${d.name} completion zsh)      in ~/.zshrc, after compinit
  fish  ${d.name} completion fish | source      in ~/.config/fish/config.fish
The script holds the commands, the subcommands and the long options of this version. It makes no request.
`)
    .action((shell: Shell) => d.stdout(SCRIPTS[shell](d.name, entries(tree(program)))));
}
