use super::{CliError, Command, Parsed};

pub fn parse(command: &Command, argv: &[String]) -> Result<Parsed, CliError> {
    let mut out = Parsed::default();
    let mut i = 0;
    let mut literal = false;
    let mut positional = 0;
    while i < argv.len() {
        let arg = &argv[i];
        if !literal && arg == "--" {
            literal = true;
            i += 1;
            continue;
        }
        if !literal && (arg == "-h" || arg == "--help") {
            return Err(CliError(format!("HELP:{}", command.name)));
        }
        if !literal && arg == "help" && super::help::has_help_command(command) {
            let mut node = command;
            for name in argv.iter().skip(i + 1) {
                let Some(sub) = node
                    .commands
                    .iter()
                    .find(|c| c.name == *name || c.aliases.contains(name))
                else {
                    return Err(CliError(format!("error: unknown command '{}'", name)));
                };
                node = sub;
            }
            return Err(CliError(format!("HELP:{}", node.name)));
        }
        if !literal && (arg == "-V" || arg == "--version") && command.version.is_some() {
            return Err(CliError(command.version.clone().unwrap()));
        }
        if !literal && arg.starts_with('-') {
            let (flag, inline) = arg
                .split_once('=')
                .map_or((arg.as_str(), None), |(a, b)| (a, Some(b)));
            let opt = command
                .options
                .iter()
                .find(|o| o.long() == flag || o.short() == Some(flag))
                .ok_or_else(|| CliError(format!("error: unknown option '{}'", flag)))?;
            let key = opt.key();
            let value = if opt.boolean {
                inline.unwrap_or("true").to_owned()
            } else {
                let v = inline
                    .map(str::to_owned)
                    // commander: a `<value>` option takes the next argument, also `-` or `-5`.
                    .or_else(|| argv.get(i + 1).cloned())
                    .ok_or_else(|| {
                        CliError(format!("error: option '{}' argument missing", flag))
                    })?;
                if inline.is_none() {
                    i += 1;
                }
                if !opt.choices.is_empty() && !opt.choices.contains(&v) {
                    return Err(CliError(format!(
                        "error: option '{}' argument '{}' is invalid. Allowed choices are {}.",
                        flag,
                        v,
                        opt.choices.join(", ")
                    )));
                }
                v
            };
            if !opt.repeat && out.options.contains_key(&key) {
                return Err(CliError(format!(
                    "error: option '{}' argument is not allowed more than once",
                    flag
                )));
            }
            out.options.entry(key.clone()).or_default().push(value);
            out.option_sources.insert(key, "cli");
        } else if let Some(sub) = command
            .commands
            .iter()
            .find(|c| c.name == *arg || c.aliases.contains(arg))
        {
            let nested = argv[i + 1..].to_vec();
            // Commander assigns options before a subcommand to the parent. Do not silently pass them to the leaf.
            let mut parsed = parse(sub, &nested)?;
            parsed.command.insert(0, sub.name.clone());
            for (k, v) in out.options {
                parsed.options.insert(k, v);
            }
            for (k, v) in out.option_sources {
                parsed.option_sources.insert(k, v);
            }
            return Ok(parsed);
        } else {
            let specs = &command.arguments;
            if let Some(spec) = specs.get(positional) {
                out.arguments.push(arg.clone());
                if !spec.variadic {
                    positional += 1;
                }
            } else if !command.commands.is_empty() {
                let names = command
                    .commands
                    .iter()
                    .filter(|c| !c.hidden)
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>();
                let hint = super::suggest::suggest(arg, &names)
                    .map(|s| format!("\n(Did you mean {}?)", s))
                    .unwrap_or_default();
                return Err(CliError(format!(
                    "error: unknown command '{}'{}",
                    arg, hint
                )));
            } else {
                return Err(CliError(format!(
                    "error: too many arguments. Expected {} argument(s) but got {}.",
                    specs.len(),
                    out.arguments.len() + 1
                )));
            }
        }
        i += 1;
    }
    for (idx, spec) in command.arguments.iter().enumerate() {
        if spec.required && out.arguments.get(idx).is_none() {
            return Err(CliError(format!(
                "error: missing required argument '{}'",
                spec.syntax.trim_matches(['<', '>', '[', ']', '.'])
            )));
        }
    }
    // commander `Argument.choices()`: the message names the argument without its brackets.
    for (idx, value) in out.arguments.iter().enumerate() {
        let Some(spec) = command
            .arguments
            .get(idx)
            .or_else(|| command.arguments.last().filter(|a| a.variadic))
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
    for opt in &command.options {
        let key = opt.key();
        if !out.options.contains_key(&key) {
            if let Some(v) = &opt.default {
                // The `[]` default of a repeated option (commander shows `(default: [])`) is an empty list.
                if !(opt.repeat && v == "[]") {
                    out.options.entry(key).or_default().push(v.clone());
                }
            } else if opt.required {
                return Err(CliError(format!(
                    "error: required option '{}' not specified",
                    opt.long()
                )));
            } else if opt.boolean {
                out.options.entry(key).or_default().push("false".into());
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::OptionDef;
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
    }
}
