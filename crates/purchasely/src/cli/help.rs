//! A port of the commander 15 `Help.formatHelp` (`node_modules/commander/lib/help.js`), plus the
//! `addHelpText("after")` text that commander writes after it.
use super::{Argument, Command, OptionDef};

const MIN_WIDTH_TO_WRAP: usize = 40;

/// The help of `command`. `ancestors` are the names of its parents, root first (commander `commandUsage`).
pub fn help(command: &Command, ancestors: &[&str], width: usize) -> String {
    let help_command = has_help_command(command);
    let options = visible_options(command);
    let arguments = if command.arguments.iter().any(|a| !a.description.is_empty()) {
        command.arguments.iter().collect()
    } else {
        Vec::new()
    };
    let mut commands = command
        .commands
        .iter()
        .filter(|c| !c.hidden)
        .map(|c| (subcommand_term(c), c.description.clone()))
        .collect::<Vec<_>>();
    if help_command {
        commands.push((
            "help [command]".to_owned(),
            "display help for command".to_owned(),
        ));
    }
    let term_width = options
        .iter()
        .map(|(t, _)| t.chars().count())
        .chain(commands.iter().map(|(t, _)| t.chars().count()))
        .chain(arguments.iter().map(|a| arg_name(a).chars().count()))
        .max()
        .unwrap_or(0);

    let mut name = command.name.clone();
    if let Some(alias) = command.aliases.first() {
        name = format!("{name}|{alias}");
    }
    let mut usage = vec!["[options]".to_owned()];
    if !command.commands.is_empty() {
        usage.push("[command]".into());
    }
    usage.extend(command.arguments.iter().map(|a| a.syntax.clone()));
    let mut prefix = ancestors.join(" ");
    if !prefix.is_empty() {
        prefix.push(' ');
    }
    let mut output = vec![
        format!("Usage: {prefix}{name} {}", usage.join(" ")),
        String::new(),
    ];
    if !command.description.is_empty() {
        output.push(box_wrap(&command.description, width));
        output.push(String::new());
    }
    let mut list = |heading: &str, items: Vec<String>| {
        if !items.is_empty() {
            output.push(heading.to_owned());
            output.extend(items);
            output.push(String::new());
        }
    };
    list(
        "Arguments:",
        arguments
            .iter()
            .map(|a| format_item(&arg_name(a), term_width, &argument_description(a), width))
            .collect(),
    );
    list(
        "Options:",
        options
            .iter()
            .map(|(t, d)| format_item(t, term_width, d, width))
            .collect(),
    );
    list(
        "Commands:",
        commands
            .iter()
            .map(|(t, d)| format_item(t, term_width, d, width))
            .collect(),
    );
    let mut out = output.join("\n");
    if !command.after_help.is_empty() {
        out.push_str(&command.after_help);
        out.push('\n');
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// commander `_getHelpCommand`: a group with subcommands and no action lists `help [command]`.
fn has_help_command(command: &Command) -> bool {
    !command.commands.is_empty()
        && (command.help_command || command.action.is_none())
        && !command.commands.iter().any(|c| c.name == "help")
}

/// The version option first (TS calls `.version()` before any option), then the options, then `-h, --help`.
fn visible_options(command: &Command) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if command.version.is_some() {
        out.push((
            "-V, --version".to_owned(),
            "output the version number".to_owned(),
        ));
    }
    out.extend(
        command
            .options
            .iter()
            .filter(|o| !o.hidden)
            .map(|o| (o.flags.clone(), option_description(o))),
    );
    out.push((
        "-h, --help".to_owned(),
        "display help for command".to_owned(),
    ));
    out
}

fn option_description(o: &OptionDef) -> String {
    let mut extra = Vec::new();
    if !o.choices.is_empty() {
        let choices = o
            .choices
            .iter()
            .map(|c| format!("{c:?}"))
            .collect::<Vec<_>>();
        extra.push(format!("choices: {}", choices.join(", ")));
    }
    if let Some(default) = o.default.as_ref().filter(|_| !o.boolean) {
        if default == "[]" {
            extra.push("default: []".to_owned());
        } else {
            extra.push(format!("default: {default:?}"));
        }
    }
    match (extra.is_empty(), o.description.is_empty()) {
        (true, _) => o.description.clone(),
        (false, true) => format!("({})", extra.join(", ")),
        (false, false) => format!("{} ({})", o.description, extra.join(", ")),
    }
}

/// commander `subcommandTerm`: the name, the first alias, `[options]` and the arguments.
fn subcommand_term(c: &Command) -> String {
    let mut term = c.name.clone();
    if let Some(alias) = c.aliases.first() {
        term = format!("{term}|{alias}");
    }
    if !c.options.is_empty() {
        term.push_str(" [options]");
    }
    for a in &c.arguments {
        term.push(' ');
        term.push_str(&a.syntax);
    }
    term
}

/// commander `argument.name()`: the syntax without its brackets and `...`.
fn arg_name(a: &Argument) -> String {
    a.syntax
        .trim_matches(['<', '>', '[', ']'])
        .trim_end_matches("...")
        .to_owned()
}

fn argument_description(argument: &Argument) -> String {
    if argument.choices.is_empty() {
        argument.description.clone()
    } else {
        format!(
            "{} (choices: {})",
            argument.description,
            argument
                .choices
                .iter()
                .map(|choice| format!("{choice:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn format_item(term: &str, term_width: usize, description: &str, width: usize) -> String {
    if description.is_empty() {
        return format!("  {term}");
    }
    let padded = format!("{term:<term_width$}");
    let remaining = width.saturating_sub(term_width + 4);
    let formatted = if remaining < MIN_WIDTH_TO_WRAP || preformatted(description) {
        description.to_owned()
    } else {
        box_wrap(description, remaining).replace('\n', &format!("\n{}", " ".repeat(term_width + 2)))
    };
    format!("  {padded}  {}", formatted.replace('\n', "\n  "))
}

/// commander `preformatted`: a line break followed by a blank that is not a line break.
fn preformatted(s: &str) -> bool {
    s.split('\n')
        .skip(1)
        .any(|line| line.starts_with(|c: char| c.is_whitespace() && c != '\r'))
}

/// commander `boxWrap`: wrap at whitespace, keep the line breaks.
fn box_wrap(s: &str, width: usize) -> String {
    if width < MIN_WIDTH_TO_WRAP {
        return s.to_owned();
    }
    let mut lines = Vec::new();
    for line in s.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)) {
        // chunks of /[\s]*[^\s]+/g
        let mut chunks = Vec::new();
        let mut rest = line;
        while let Some(start) = rest.find(|c: char| !c.is_whitespace()) {
            let end = rest[start..]
                .find(char::is_whitespace)
                .map_or(rest.len(), |e| start + e);
            chunks.push(&rest[..end]);
            rest = &rest[end..];
        }
        let Some((first, others)) = chunks.split_first() else {
            lines.push(String::new());
            continue;
        };
        let mut current = (*first).to_owned();
        let mut current_width = first.chars().count();
        for chunk in others {
            let w = chunk.chars().count();
            if current_width + w <= width {
                current.push_str(chunk);
                current_width += w;
                continue;
            }
            lines.push(current);
            let next = chunk.trim_start();
            current = next.to_owned();
            current_width = next.chars().count();
        }
        lines.push(current);
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::help;
    use crate::cli::{Argument, Command, OptionDef};
    #[test]
    fn root_layout() {
        let command = Command::new("purchasely")
            .description("Purchasely command line tool")
            .version("1.2.3")
            .command(Command::new("auth").description("Manage the key"));
        assert_eq!(
            help(&command, &[], 80),
            "Usage: purchasely [options] [command]\n\nPurchasely command line tool\n\nOptions:\n  -V, --version   output the version number\n  -h, --help      display help for command\n\nCommands:\n  auth            Manage the key\n  help [command]  display help for command\n"
        );
    }
    #[test]
    fn leaf_layout_wraps_and_appends_the_after_text() {
        let command = Command::new("login")
            .description("Store the key")
            .argument(Argument::required("<key>", ""))
            .option(OptionDef::value(
                "--api-key <key>",
                "the key (visible in the shell history: prefer the prompt or stdin)",
            ))
            .after_help("\nMore text.\n");
        assert_eq!(
            help(&command, &["purchasely", "auth"], 80),
            "Usage: purchasely auth login [options] <key>\n\nStore the key\n\nOptions:\n  --api-key <key>  the key (visible in the shell history: prefer the prompt or\n                   stdin)\n  -h, --help       display help for command\n\nMore text.\n\n"
        );
    }
}
