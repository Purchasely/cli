use super::{CliError, Command, OptionDef, Parsed};

/// commander 15 `parse`: each command reads the options it knows anywhere in its words (no positional
/// options), then dispatches to the subcommand named by its first operand. A request for help is the error
/// `HELP:<path>` (the names below the root, joined by spaces); `-V` is the error holding the version.
pub fn parse(command: &Command, argv: &[String]) -> Result<Parsed, CliError> {
    let mut path = Vec::new();
    parse_command(command, &[], &[], argv.to_vec(), &mut path)
}

fn help(path: &[String]) -> CliError {
    CliError(format!("HELP:{}", path.join(" ")))
}

fn find_command<'a>(command: &'a Command, name: &str) -> Option<&'a Command> {
    command
        .commands
        .iter()
        .find(|c| c.name == name || c.aliases.iter().any(|a| a == name))
}

/// commander `_parseCommand`.
fn parse_command(
    command: &Command,
    ancestors: &[&Command],
    operands_in: &[String],
    unknown_in: Vec<String>,
    path: &mut Vec<String>,
) -> Result<Parsed, CliError> {
    let mut out = Parsed::default();
    let (operands_p, unknown) = parse_options(command, ancestors, &unknown_in, &mut out)?;
    let operands: Vec<String> = operands_in.iter().cloned().chain(operands_p).collect();
    let args: Vec<String> = operands.iter().chain(&unknown).cloned().collect();

    if let Some(sub) = operands
        .first()
        .and_then(|name| find_command(command, name))
    {
        path.push(sub.name.clone());
        let mut lineage = ancestors.to_vec();
        lineage.push(command);
        let mut parsed = parse_command(sub, &lineage, &operands[1..], unknown, path)?;
        // run.ts preAction: a group under the root parses the options before its subcommand for itself and
        // the leaf never sees them, so they are refused.
        if !ancestors.is_empty()
            && !out.option_sources.is_empty()
            && sub.action.is_some()
            && parsed.command.is_empty()
        {
            let root = ancestors[0].name.as_str();
            return Err(CliError(format!(
                "error: Put the options after the subcommand: {root} {} {} --app ...",
                command.name, sub.name
            )));
        }
        parsed.command.insert(0, sub.name.clone());
        for (k, v) in out.options {
            parsed.options.insert(k, v);
        }
        for (k, v) in out.option_sources {
            parsed.option_sources.insert(k, v);
        }
        return Ok(parsed);
    }
    // commander `_dispatchHelpCommand`; a group's `help({ error: true })` prints on stdout (run.ts `stdoutHelp`).
    if super::help::has_help_command(command) && operands.first().is_some_and(|a| a == "help") {
        if let Some(sub) = operands.get(1).and_then(|name| find_command(command, name)) {
            path.push(sub.name.clone());
        }
        return Err(help(path));
    }
    if !command.commands.is_empty() && args.is_empty() && command.action.is_none() {
        return Err(help(path));
    }
    if !command.no_help_option && unknown.iter().any(|a| a == "-h" || a == "--help") {
        return Err(help(path));
    }
    fill_defaults(command, &mut out)?;
    let check_unknown = || -> Result<(), CliError> {
        match unknown.first() {
            Some(flag) if !command.allow_unknown_option => {
                Err(unknown_option(command, ancestors, flag))
            }
            _ => Ok(()),
        }
    };
    if command.action.is_some() {
        check_unknown()?;
        process_arguments(command, !ancestors.is_empty(), args, &mut out)?;
    } else if !operands.is_empty() {
        if !command.commands.is_empty() {
            let names = command
                .commands
                .iter()
                .filter(|c| !c.hidden)
                .flat_map(|c| {
                    std::iter::once(c.name.as_str()).chain(c.aliases.first().map(String::as_str))
                })
                .collect::<Vec<_>>();
            return Err(CliError(format!(
                "error: unknown command '{}'{}",
                operands[0],
                suggestion(&operands[0], &names)
            )));
        }
        check_unknown()?;
        process_arguments(command, !ancestors.is_empty(), args, &mut out)?;
    } else if !command.commands.is_empty() {
        check_unknown()?;
        return Err(help(path));
    } else {
        check_unknown()?;
        process_arguments(command, !ancestors.is_empty(), args, &mut out)?;
    }
    Ok(out)
}

/// commander `suggestSimilar`, with its `\n(Did you mean ...?)` text.
fn suggestion(word: &str, candidates: &[&str]) -> String {
    let options = word.starts_with("--");
    let strip = |s: &str| -> String {
        if options {
            s.strip_prefix("--").unwrap_or(s).to_owned()
        } else {
            s.to_owned()
        }
    };
    let word = strip(word);
    let mut seen = Vec::new();
    let mut similar: Vec<String> = Vec::new();
    let mut best = 3;
    for candidate in candidates {
        let candidate = strip(candidate);
        if seen.contains(&candidate) || candidate.chars().count() <= 1 {
            continue;
        }
        seen.push(candidate.clone());
        let distance = super::suggest::distance(&word, &candidate);
        let length = word.chars().count().max(candidate.chars().count());
        if (length - distance) as f64 / length as f64 > 0.4 {
            if distance < best {
                best = distance;
                similar = vec![candidate];
            } else if distance == best {
                similar.push(candidate);
            }
        }
    }
    similar.sort();
    let similar: Vec<String> = similar
        .into_iter()
        .map(|c| if options { format!("--{c}") } else { c })
        .collect();
    match similar.len() {
        0 => String::new(),
        1 => format!("\n(Did you mean {}?)", similar[0]),
        _ => format!("\n(Did you mean one of {}?)", similar.join(", ")),
    }
}

/// commander `unknownOption`: the suggestion lists the visible long options of the command and its ancestors.
fn unknown_option(command: &Command, ancestors: &[&Command], flag: &str) -> CliError {
    let mut candidates: Vec<&str> = Vec::new();
    if flag.starts_with("--") {
        // commander walks up from the command and stops at an ancestor with positional options.
        let walk = ancestors
            .iter()
            .rev()
            .copied()
            .take_while(|c| !c.positional_options);
        for c in std::iter::once(command).chain(walk) {
            if c.version.is_some() {
                candidates.push("--version");
            }
            candidates.extend(
                c.options
                    .iter()
                    .filter(|o| !o.hidden && o.long().starts_with("--"))
                    .map(OptionDef::long),
            );
            if !c.no_help_option {
                candidates.push("--help");
            }
        }
    }
    CliError(format!(
        "error: unknown option '{flag}'{}",
        suggestion(flag, &candidates)
    ))
}

fn negative_number(arg: &str, command: &Command, ancestors: &[&Command]) -> bool {
    let rest = &arg[1..];
    let (mantissa, exponent) = match rest.find('e') {
        Some(i) => (&rest[..i], Some(&rest[i + 1..])),
        None => (rest, None),
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let mantissa_ok = digits(mantissa)
        || mantissa
            .split_once('.')
            .is_some_and(|(a, b)| (a.is_empty() || digits(a)) && digits(b));
    let exponent_ok = exponent.is_none_or(|e| digits(e.strip_prefix(['+', '-']).unwrap_or(e)));
    mantissa_ok
        && exponent_ok
        && !std::iter::once(command)
            .chain(ancestors.iter().copied())
            .flat_map(|c| &c.options)
            .any(|o| {
                o.short()
                    .is_some_and(|s| s.len() == 2 && s.as_bytes()[1].is_ascii_digit())
            })
}

/// commander `parseOptions`: the options of `command` anywhere in `args`; the rest is operands and unknown.
fn parse_options(
    command: &Command,
    ancestors: &[&Command],
    args: &[String],
    out: &mut Parsed,
) -> Result<(Vec<String>, Vec<String>), CliError> {
    let maybe_option = |a: &str| a.len() > 1 && a.starts_with('-');
    let find = |flag: &str| -> Option<&OptionDef> {
        command
            .options
            .iter()
            .find(|o| o.long() == flag || o.short() == Some(flag))
    };
    let version = |flag: &str| command.version.is_some() && (flag == "-V" || flag == "--version");
    let mut operands = Vec::new();
    let mut unknown = Vec::new();
    let mut to_unknown = false;
    let mut group: Option<String> = None;
    let mut i = 0;
    while i < args.len() || group.is_some() {
        let arg = match group.take() {
            Some(g) => g,
            None => {
                i += 1;
                args[i - 1].clone()
            }
        };
        if arg == "--" {
            let dest = if to_unknown {
                &mut unknown
            } else {
                &mut operands
            };
            if to_unknown {
                dest.push(arg);
            }
            dest.extend(args[i..].iter().cloned());
            break;
        }
        if maybe_option(&arg) {
            if version(&arg) {
                return Err(CliError(command.version.clone().unwrap_or_default()));
            }
            if let Some(opt) = find(&arg) {
                if opt.boolean {
                    set(out, opt, "true".into())?;
                } else {
                    let Some(value) = args.get(i).cloned() else {
                        return Err(CliError(format!(
                            "error: option '{}' argument missing",
                            opt.flags
                        )));
                    };
                    i += 1;
                    set(out, opt, value)?;
                }
                continue;
            }
        }
        if arg.len() > 2 && arg.starts_with('-') && !arg.starts_with("--") {
            let short = &arg[..2];
            if version(short) {
                return Err(CliError(command.version.clone().unwrap_or_default()));
            }
            if let Some(opt) = find(short) {
                if opt.boolean {
                    set(out, opt, "true".into())?;
                    group = Some(format!("-{}", &arg[2..]));
                } else {
                    set(out, opt, arg[2..].to_owned())?;
                }
                continue;
            }
        }
        if let Some(index) = arg.find('=')
            && arg.starts_with("--")
            && index > 2
            && let Some(opt) = find(&arg[..index])
            && !opt.boolean
        {
            set(out, opt, arg[index + 1..].to_owned())?;
            continue;
        }
        if !to_unknown
            && maybe_option(&arg)
            && !(command.commands.is_empty() && negative_number(&arg, command, ancestors))
        {
            to_unknown = true;
        }
        if command.positional_options && operands.is_empty() && unknown.is_empty() {
            if find_command(command, &arg).is_some() {
                operands.push(arg);
                unknown.extend(args[i..].iter().cloned());
                break;
            }
            if arg == "help" && super::help::has_help_command(command) {
                operands.push(arg);
                operands.extend(args[i..].iter().cloned());
                break;
            }
        }
        if to_unknown {
            unknown.push(arg);
        } else {
            operands.push(arg);
        }
    }
    Ok((operands, unknown))
}

/// commander `option:<name>` event: choices are checked, a repeated value replaces the last unless collected.
fn set(out: &mut Parsed, opt: &OptionDef, value: String) -> Result<(), CliError> {
    if !opt.choices.is_empty() && !opt.choices.contains(&value) {
        return Err(CliError(format!(
            "error: option '{}' argument '{}' is invalid. Allowed choices are {}.",
            opt.flags,
            value,
            opt.choices.join(", ")
        )));
    }
    let key = opt.key();
    let values = out.options.entry(key.clone()).or_default();
    if !opt.repeat {
        values.clear();
    }
    values.push(value);
    out.option_sources.insert(key, "cli");
    Ok(())
}

/// The defaults of the options not given, and commander `_checkForMissingMandatoryOptions`.
fn fill_defaults(command: &Command, out: &mut Parsed) -> Result<(), CliError> {
    for opt in &command.options {
        let key = opt.key();
        if out.options.contains_key(&key) {
            continue;
        }
        if let Some(v) = &opt.default {
            // The `[]` default of a repeated option (commander shows `(default: [])`) is an empty list.
            if !(opt.repeat && v == "[]") {
                out.options.entry(key).or_default().push(v.clone());
            }
        } else if opt.required {
            return Err(CliError(format!(
                "error: required option '{}' not specified",
                opt.flags
            )));
        } else if opt.boolean {
            out.options.entry(key).or_default().push("false".into());
        }
    }
    Ok(())
}

/// commander `_processArguments`: the count, then the choices of each argument.
fn process_arguments(
    command: &Command,
    has_parent: bool,
    args: Vec<String>,
    out: &mut Parsed,
) -> Result<(), CliError> {
    let specs = &command.arguments;
    for (idx, spec) in specs.iter().enumerate() {
        if spec.required && args.get(idx).is_none() {
            return Err(CliError(format!(
                "error: missing required argument '{}'",
                spec.syntax.trim_matches(['<', '>', '[', ']', '.'])
            )));
        }
    }
    if args.len() > specs.len() && !specs.last().is_some_and(|a| a.variadic) {
        let expected = specs.len();
        return Err(CliError(format!(
            "error: too many arguments{}. Expected {expected} argument{} but got {}: {}.",
            if has_parent {
                format!(" for '{}'", command.name)
            } else {
                String::new()
            },
            if expected == 1 { "" } else { "s" },
            args.len(),
            args.join(", ")
        )));
    }
    // commander `Argument.choices()`: the message names the argument without its brackets.
    for (idx, value) in args.iter().enumerate() {
        let Some(spec) = specs
            .get(idx)
            .or_else(|| specs.last().filter(|a| a.variadic))
        else {
            continue;
        };
        if !spec.choices.is_empty() && !spec.choices.contains(value) {
            let name = spec.syntax.trim_matches(['<', '>', '[', ']', '.']);
            return Err(CliError(format!(
                "error: command-argument value '{value}' is invalid for argument '{name}'. Allowed choices are {}.",
                spec.choices.join(", ")
            )));
        }
    }
    out.arguments = args;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::OptionDef;
    #[test]
    fn allow_unknown_option_passes_unknown_flags_as_arguments() {
        let words = |list: &[&str]| list.iter().map(|w| w.to_string()).collect::<Vec<_>>();
        let command = Command::new("call")
            .argument(crate::cli::Argument::required("<tool>", "tool"))
            .argument(crate::cli::Argument::optional("[args...]", "flags"))
            .option(OptionDef::value("--env <env>", "env"));
        assert_eq!(
            parse(&command, &words(&["t", "--per-page", "5"]))
                .unwrap_err()
                .0,
            "error: unknown option '--per-page'"
        );
        let open = command.allow_unknown_option();
        let parsed = parse(
            &open,
            &words(&["t", "--per-page", "5", "--env", "staging", "--data", "{}"]),
        )
        .unwrap();
        assert_eq!(
            parsed.arguments,
            words(&["t", "--per-page", "5", "--data", "{}"])
        );
        assert_eq!(parsed.opt("env"), Some("staging"));
    }
    #[test]
    fn parses_alias_equals_and_repeated_values() {
        let command = Command::new("tool")
            .option(OptionDef::value("-f, --format <value>", "format").default("json"))
            .option(OptionDef::value("--tag <value>", "tag").repeat());
        let parsed = parse(
            &command,
            &[
                "--format=table".into(),
                "--tag".into(),
                "a".into(),
                "--tag".into(),
                "b".into(),
            ],
        )
        .unwrap();
        assert_eq!(parsed.options["format"], ["table"]);
        assert_eq!(parsed.options["tag"], ["a", "b"]);
    }
    #[test]
    fn missing_argument_and_bad_choice_are_errors() {
        let c = Command::new("x")
            .argument(crate::cli::Argument::required("<id>", "id"))
            .option(OptionDef::value("--kind <kind>", "kind").choices(&["a", "b"]));
        assert!(
            parse(&c, &[])
                .unwrap_err()
                .0
                .contains("missing required argument")
        );
        assert!(
            parse(&c, &["id".into(), "--kind=c".into()])
                .unwrap_err()
                .0
                .contains("Allowed choices")
        );
    }

    #[test]
    fn a_lone_dash_is_an_argument() {
        let c = Command::new("note")
            .argument(crate::cli::Argument::required("<id>", "id"))
            .argument(crate::cli::Argument::required("<text>", "text"))
            .option(OptionDef::value("--env <env>", "env"));
        let parsed = parse(
            &c,
            &["id".into(), "-".into(), "--env".into(), "prod".into()],
        )
        .unwrap();
        assert_eq!(parsed.arguments, ["id", "-"]);
        assert_eq!(parsed.opt("env"), Some("prod"));
    }

    #[test]
    fn argument_choices_and_missing_argument_use_commander_text() {
        let c = Command::new("completion")
            .argument(crate::cli::Argument::required("<shell>", "shell").choices(&["bash", "zsh"]));
        assert_eq!(
            parse(&c, &["fish".into()]).unwrap_err().0,
            "error: command-argument value 'fish' is invalid for argument 'shell'. Allowed choices are bash, zsh."
        );
        assert_eq!(
            parse(&c, &[]).unwrap_err().0,
            "error: missing required argument 'shell'"
        );
    }

    #[test]
    fn empty_list_default_of_a_repeated_option_gives_no_value() {
        let c = Command::new("api").option(
            OptionDef::value("--query <k=v>", "q")
                .repeat()
                .default("[]"),
        );
        assert!(!parse(&c, &[]).unwrap().options.contains_key("query"));
    }

    #[test]
    fn help_command_routes_to_nested_command() {
        let command = Command::new("dashboard")
            .help_command(true)
            .command(Command::new("summary"));
        let parsed = parse(&command, &["help".into(), "summary".into()]).unwrap_err();
        assert_eq!(parsed.0, "HELP:summary");
        let unknown = parse(&command, &["help".into(), "nope".into()]).unwrap_err();
        assert_eq!(unknown.0, "HELP:");
    }

    /// commander 15 texts and rules, checked side by side against the TS program.
    #[test]
    fn commander_parity_of_errors_and_repeats() {
        let words = |list: &[&str]| list.iter().map(|w| w.to_string()).collect::<Vec<_>>();
        let leaf = Command::new("list")
            .option(OptionDef::value("--app <app_id>", "app"))
            .option(OptionDef::value("--reason <why>", "reason"))
            .option(OptionDef::boolean("--dry-run", "dry"))
            .action(|_, _| Ok(()));
        let root = Command::new("purchasely")
            .version("1.0.0")
            .positional_options()
            .command(Command::new("screens").command(leaf));
        let err = |argv: &[&str]| parse(&root, &words(argv)).unwrap_err().0;
        // m1: a repeated option keeps the last value.
        let p = parse(&root, &words(&["screens", "list", "--app=x", "--app", "y"])).unwrap();
        assert_eq!(p.options["app"], ["y"]);
        // m2: the texts of commander.
        assert_eq!(
            err(&["screens", "list", "--app"]),
            "error: option '--app <app_id>' argument missing"
        );
        assert_eq!(
            err(&["screens", "list", "extra"]),
            "error: too many arguments for 'list'. Expected 0 arguments but got 1: extra."
        );
        assert_eq!(
            err(&["screens", "list", "--ap", "x"]),
            "error: unknown option '--ap'\n(Did you mean --app?)"
        );
        assert_eq!(
            err(&["screens", "list", "--version"]),
            "error: unknown option '--version'\n(Did you mean --reason?)"
        );
        // m3: help of an unknown command is the root help; an option before the subcommand is unknown.
        assert_eq!(err(&["nope", "--help"]), "HELP:");
        assert_eq!(err(&["help", "nope"]), "HELP:");
        assert_eq!(err(&[]), "HELP:");
        assert_eq!(err(&["screens", "list", "-h"]), "HELP:screens list");
        assert_eq!(
            err(&["--env", "staging", "screens", "list"]),
            "error: unknown option '--env'"
        );
        assert_eq!(err(&["--version", "screens"]), "1.0.0");
    }

    #[test]
    fn required_option_error_names_the_flags_as_commander() {
        let c = Command::new("tree").option(OptionDef::value("--keys <list>", "k").required());
        assert_eq!(
            parse(&c, &[]).unwrap_err().0,
            "error: required option '--keys <list>' not specified"
        );
    }

    #[test]
    fn no_help_option_leaves_help_to_the_action() {
        let command = Command::new("dashboard")
            .no_help_option()
            .option(OptionDef::boolean("-h, --help", "own help"));
        assert!(parse(&command, &["--help".into()]).unwrap().flag("help"));
        assert!(parse(&command, &["-h".into()]).unwrap().flag("help"));
        let help = crate::cli::help::help(&command, &[], 80);
        assert_eq!(help.matches("-h, --help").count(), 1);
        assert!(help.contains("own help"));
    }
}
