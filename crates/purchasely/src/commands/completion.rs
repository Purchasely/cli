//! Owner: P7a
use crate::{
    cli::{Argument, Command, Parsed},
    core::{errors::usage, reference::Node, runtime::Runtime},
};

#[derive(Clone)]
struct Entry {
    path: String,
    subs: Vec<Node>,
    opts: Vec<(String, String)>,
}
fn entries(node: &Node, path: Vec<String>, out: &mut Vec<Entry>) {
    let mut opts: Vec<_> = node
        .options
        .iter()
        .filter_map(|o| o.long.clone().map(|l| (l, o.description.clone())))
        .collect();
    opts.push(("--help".into(), "display help for the command".into()));
    out.push(Entry {
        path: path.join(" "),
        subs: node.commands.clone(),
        opts,
    });
    for c in &node.commands {
        let mut next = path.clone();
        next.push(c.name.clone());
        entries(c, next, out);
    }
}
fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn sq(s: &str) -> String {
    format!("'{}'", one_line(s).replace('\'', "'\\''"))
}
fn fq(s: &str) -> String {
    format!("'{}'", one_line(s).replace(['\\', '\''], "\\$&"))
}
fn render_bash(name: &str, es: &[Entry]) -> String {
    let cases = |pick: &dyn Fn(&Entry) -> Vec<String>| {
        es.iter()
            .map(|e| {
                format!(
                    "    {}) echo {} ;;",
                    sq(&e.path),
                    pick(e).iter().map(|s| sq(s)).collect::<Vec<_>>().join(" ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        "# bash completion for {name}. Add to ~/.bashrc: source <({name} completion bash)\n_{name}_subs() {{\n  case \"$1\" in\n{}\n  esac\n}}\n_{name}_opts() {{\n  case \"$1\" in\n{}\n  esac\n}}\n_{name}_complete() {{\n  local cur=\"${{COMP_WORDS[COMP_CWORD]}}\" path=\"\" i w\n  for ((i = 1; i < COMP_CWORD; i++)); do\n    w=\"${{COMP_WORDS[i]}}\"\n    case \" $(_{name}_subs \"$path\") \" in *\" $w \"*) path=\"${{path:+$path }}$w\" ;; esac\n  done\n  COMPREPLY=($(compgen -W \"$(_{name}_subs \"$path\") $(_{name}_opts \"$path\")\" -- \"$cur\"))\n}}\ncomplete -F _{name}_complete {name}\n",
        cases(&|e| e.subs.iter().map(|s| s.name.clone()).collect()),
        cases(&|e| e.opts.iter().map(|(s, _)| s.clone()).collect())
    )
}
fn render_zsh(name: &str, es: &[Entry]) -> String {
    let cases = es
        .iter()
        .map(|e| {
            format!(
                "    {})\n      names=({})\n      subs=({})\n      opts=({}) ;;",
                sq(&e.path),
                e.subs
                    .iter()
                    .map(|n| sq(&n.name))
                    .collect::<Vec<_>>()
                    .join(" "),
                e.subs
                    .iter()
                    .map(|n| sq(&format!("{}:{}", n.name, n.description)))
                    .collect::<Vec<_>>()
                    .join(" "),
                e.opts
                    .iter()
                    .map(|(n, d)| sq(&format!("{n}:{d}")))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "#compdef {name}\n_{name}_node() {{\n  case \"$1\" in\n{cases}\n  esac\n}}\n_{name}() {{\n  local cpath=\"\" w i\n  local -a names subs opts\n  _{name}_node \"\"\n  for ((i = 2; i < CURRENT; i++)); do\n    w=$words[i]\n    if (( ${{names[(Ie)$w]}} )); then\n      cpath=\"${{cpath:+$cpath }}$w\"\n      _{name}_node \"$cpath\"\n    fi\n  done\n  _describe -t commands 'command' subs\n  _describe -t options 'option' opts\n}}\nif [ \"$funcstack[1]\" = \"_{name}\" ]; then _{name} \"$@\"; else compdef _{name} {name}; fi\n"
    )
}
fn render_fish(name: &str, es: &[Entry]) -> String {
    let subs = es
        .iter()
        .filter(|e| !e.subs.is_empty())
        .map(|e| {
            format!(
                "    case {}\n      printf '%s\\n' {}",
                fq(&e.path),
                e.subs
                    .iter()
                    .map(|s| fq(&s.name))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut lines = Vec::new();
    for e in es {
        for s in &e.subs {
            lines.push(format!(
                "complete -c {name} -n '__{name}_at {}' -a {} -d {}",
                e.path,
                fq(&s.name),
                fq(&s.description)
            ));
        }
        for (n, d) in &e.opts {
            lines.push(format!(
                "complete -c {name} -n '__{name}_at {}' -l {} -d {}",
                e.path,
                n.trim_start_matches("--"),
                fq(d)
            ));
        }
    }
    let lines = lines.join("\n");
    format!(
        "# fish completion for {name}. Add to ~/.config/fish/config.fish: {name} completion fish | source\nfunction __{name}_subs\n  switch \"$argv\"\n{subs}\n  end\nend\nfunction __{name}_at\n  set -l path\n  for w in (commandline -opc)[2..-1]\n    if contains -- $w (__{name}_subs $path)\n      set path $path $w\n    end\n  end\n  test \"$path\" = \"$argv\"\nend\ncomplete -c {name} -f\n{lines}\n"
    )
}
/// Render completion for the command tree passed by the caller.
pub fn render(shell: &str, command: &Command) -> String {
    let mut es = Vec::new();
    entries(&crate::core::reference::tree(command), Vec::new(), &mut es);
    match shell {
        "bash" => render_bash(&command.name, &es),
        "zsh" => render_zsh(&command.name, &es),
        "fish" => render_fish(&command.name, &es),
        _ => String::new(),
    }
}
pub fn register(program: &mut Command, _rt: &mut Runtime) {
    let shell = Argument::required("<shell>", "the shell of the script");
    let command = Command::new("completion")
        .description("Print a shell completion script (bash, zsh or fish)")
        .argument(shell);
    let snapshot = program.clone();
    program.commands.push(command.action(move |p: &Parsed, rt| {
        let shell = p.arguments.first().map(String::as_str).unwrap_or("");
        if !["bash", "zsh", "fish"].contains(&shell) {
            return Err(usage(format!(
                "Unknown shell: {shell}. Choose bash, zsh or fish."
            )));
        }
        let out = render(shell, &snapshot);
        rt.d.host.stdout(&out);
        Ok(())
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::OptionDef;
    #[test]
    fn bash_completion_includes_two_leaf_tree_commands() {
        let s = render("bash", &sample_tree());
        assert!(s.contains("'apps list'"));
        assert!(s.contains("'apps get'"));
    }
    #[test]
    fn zsh_completion_includes_two_leaf_tree_commands() {
        let s = render("zsh", &sample_tree());
        assert!(s.contains("'list:List apps'"));
        assert!(s.contains("'get:Get app'"));
    }
    #[test]
    fn fish_completion_includes_two_leaf_tree_commands() {
        let s = render("fish", &sample_tree());
        assert!(s.contains("complete -c purchasely"));
        assert!(s.contains("apps"));
        assert!(s.contains("get"));
    }
    fn sample_tree() -> Command {
        Command::new("purchasely").command(
            Command::new("apps")
                .description("Applications")
                .command(
                    Command::new("list")
                        .description("List apps")
                        .option(OptionDef::boolean("--all", "include all")),
                )
                .command(Command::new("get").description("Get app")),
        )
    }
}
