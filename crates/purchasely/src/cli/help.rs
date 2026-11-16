use super::{Command, OptionDef};

pub fn help(command: &Command, width: usize) -> String {
    let mut out = String::new();
    out.push_str("Usage: ");
    out.push_str(&command.name);
    for arg in &command.arguments {
        out.push(' ');
        out.push_str(&arg.syntax);
    }
    if !command.options.is_empty() || command.version.is_some() || !command.commands.is_empty() {
        out.push_str(" [options]");
    }
    if !command.commands.is_empty() {
        out.push_str(" [command]");
    }
    out.push('\n');
    if !command.description.is_empty() {
        out.push('\n');
        out.push_str(&command.description);
        out.push('\n');
    }
    let mut options = command.options.clone();
    if command.version.is_some() {
        options.push(OptionDef::new("-V, --version", "output the version number"));
    }
    if !options.is_empty() {
        out.push_str("\nOptions:\n");
        let rows = options
            .iter()
            .filter(|o| !o.hidden)
            .map(|o| {
                (
                    o.flags.as_str(),
                    o.description.as_str(),
                    o.default.as_deref(),
                )
            })
            .collect::<Vec<_>>();
        render_rows(&mut out, &rows, width);
    }
    let args = command
        .arguments
        .iter()
        .map(|a| (a.syntax.as_str(), a.description.as_str(), None))
        .collect::<Vec<_>>();
    if !args.is_empty() {
        out.push_str("\nArguments:\n");
        render_rows(&mut out, &args, width);
    }
    let cmds = command
        .commands
        .iter()
        .filter(|c| !c.hidden)
        .map(|c| (c.name.as_str(), c.description.as_str(), None))
        .collect::<Vec<_>>();
    if !cmds.is_empty() {
        out.push_str("\nCommands:\n");
        render_rows(&mut out, &cmds, width);
    }
    if !command.after_help.is_empty() {
        out.push('\n');
        out.push_str(&command.after_help);
        if !command.after_help.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

fn render_rows(out: &mut String, rows: &[(&str, &str, Option<&str>)], width: usize) {
    let label = rows.iter().map(|r| r.0.len()).max().unwrap_or(0);
    let indent = label + 4;
    let desc_width = width.saturating_sub(indent).max(1);
    for (name, desc, default) in rows {
        let desc = if let Some(d) = default {
            format!("{} (default: {})", desc, d)
        } else {
            (*desc).to_owned()
        };
        let mut words = desc.split_whitespace();
        let first = words.next().unwrap_or("");
        out.push_str("  ");
        out.push_str(name);
        out.push_str(&" ".repeat(label - name.len() + 2));
        let mut col = 0;
        for word in std::iter::once(first).chain(words) {
            if col > 0 && col + 1 + word.len() > desc_width {
                out.push('\n');
                out.push_str(&" ".repeat(indent));
                col = 0;
            } else if col > 0 {
                out.push(' ');
                col += 1;
            }
            out.push_str(word);
            col += word.len();
        }
        out.push('\n');
    }
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
            .option(OptionDef::new("-h, --help", "display help for command"))
            .argument(Argument::optional("[command]", "command to run"));
        assert_eq!(
            help(&command, 80),
            "Usage: purchasely [command] [options]\n\nPurchasely command line tool\n\nOptions:\n  -h, --help     display help for command\n  -V, --version  output the version number\n\nArguments:\n  [command]  command to run\n"
        );
    }
}
