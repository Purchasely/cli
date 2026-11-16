//! A small, deterministic implementation of the Commander surface used by the CLI.
pub mod help;
pub mod parse;
pub mod suggest;

use crate::core::runtime::Runtime;
use std::{collections::BTreeMap, rc::Rc};

/// The handler of a command: it gets the parsed argv and the runtime. A closure may capture data (an operation id).
pub type Handler = dyn Fn(&Parsed, &mut Runtime) -> Result<(), crate::core::errors::CliError>;

#[derive(Clone)]
pub struct Action(pub Rc<Handler>);
impl std::fmt::Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Action")
    }
}

#[derive(Clone, Debug, Default)]
pub struct Command {
    pub name: String,
    pub description: String,
    pub version: Option<String>,
    pub aliases: Vec<String>,
    pub hidden: bool,
    pub arguments: Vec<Argument>,
    pub options: Vec<OptionDef>,
    pub commands: Vec<Command>,
    pub after_help: String,
    pub action: Option<Action>,
}

impl Command {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }
    pub fn description(mut self, value: impl Into<String>) -> Self {
        self.description = value.into();
        self
    }
    pub fn version(mut self, value: impl Into<String>) -> Self {
        self.version = Some(value.into());
        self
    }
    pub fn alias(mut self, value: impl Into<String>) -> Self {
        self.aliases.push(value.into());
        self
    }
    pub fn hide(mut self) -> Self {
        self.hidden = true;
        self
    }
    pub fn argument(mut self, value: Argument) -> Self {
        self.arguments.push(value);
        self
    }
    pub fn option(mut self, value: OptionDef) -> Self {
        self.options.push(value);
        self
    }
    pub fn command(mut self, value: Command) -> Self {
        self.commands.push(value);
        self
    }
    pub fn after_help(mut self, value: impl Into<String>) -> Self {
        self.after_help = value.into();
        self
    }
    pub fn action(
        mut self,
        f: impl Fn(&Parsed, &mut Runtime) -> Result<(), crate::core::errors::CliError> + 'static,
    ) -> Self {
        self.action = Some(Action(Rc::new(f)));
        self
    }
    /// The command at the end of a parsed path (`Parsed::command`), by name.
    pub fn find(&self, path: &[String]) -> Option<&Command> {
        path.iter().try_fold(self, |c, name| {
            c.commands.iter().find(|sub| sub.name == *name)
        })
    }
}

#[derive(Clone, Debug)]
pub struct Argument {
    pub syntax: String,
    pub description: String,
    pub required: bool,
    pub variadic: bool,
}
impl Argument {
    pub fn required(s: impl Into<String>, d: impl Into<String>) -> Self {
        Self {
            syntax: s.into(),
            description: d.into(),
            required: true,
            variadic: false,
        }
    }
    pub fn optional(s: impl Into<String>, d: impl Into<String>) -> Self {
        Self {
            syntax: s.into(),
            description: d.into(),
            required: false,
            variadic: false,
        }
    }
    pub fn variadic(mut self) -> Self {
        self.variadic = true;
        self
    }
}

#[derive(Clone, Debug)]
pub struct OptionDef {
    pub flags: String,
    pub description: String,
    pub default: Option<String>,
    pub choices: Vec<String>,
    pub required: bool,
    pub repeat: bool,
    pub hidden: bool,
    pub boolean: bool,
}
impl OptionDef {
    pub fn new(flags: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            flags: flags.into(),
            description: description.into(),
            default: None,
            choices: vec![],
            required: false,
            repeat: false,
            hidden: false,
            boolean: false,
        }
    }
    pub fn value(flags: impl Into<String>, description: impl Into<String>) -> Self {
        Self::new(flags, description)
    }
    pub fn boolean(flags: impl Into<String>, description: impl Into<String>) -> Self {
        let mut o = Self::new(flags, description);
        o.boolean = true;
        o
    }
    pub fn default(mut self, value: impl Into<String>) -> Self {
        self.default = Some(value.into());
        self
    }
    pub fn choices(mut self, values: &[&str]) -> Self {
        self.choices = values.iter().map(|s| (*s).to_owned()).collect();
        self
    }
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
    pub fn repeat(mut self) -> Self {
        self.repeat = true;
        self
    }
    pub fn hide_help(mut self) -> Self {
        self.hidden = true;
        self
    }
    pub fn long(&self) -> &str {
        self.flags
            .split(',')
            .map(str::trim)
            .find(|s| s.starts_with("--"))
            .unwrap_or(self.flags.trim())
            .split_whitespace()
            .next()
            .unwrap_or("")
    }
    pub fn short(&self) -> Option<&str> {
        self.flags
            .split(',')
            .map(str::trim)
            .find(|s| s.starts_with('-') && !s.starts_with("--"))
            .and_then(|s| s.split_whitespace().next())
    }
    pub fn key(&self) -> String {
        self.long().trim_start_matches('-').to_owned()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Parsed {
    pub options: BTreeMap<String, Vec<String>>,
    pub arguments: Vec<String>,
    pub option_sources: BTreeMap<String, &'static str>,
    pub command: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliError(pub String);
impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for CliError {}

impl Parsed {
    /// The first value of an option, as a handler reads `--env` or `--agent`.
    pub fn opt(&self, key: &str) -> Option<&str> {
        self.options.get(key)?.first().map(String::as_str)
    }
    /// A boolean option given on the command line (`--dry-run`).
    pub fn flag(&self, key: &str) -> bool {
        self.opt(key) == Some("true")
    }
}

pub fn parse(command: &Command, argv: &[String]) -> Result<Parsed, CliError> {
    parse::parse(command, argv)
}
