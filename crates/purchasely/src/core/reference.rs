//! Command tree representation and text renderers for `reference`.
use crate::{cli::Command, core::json::Value};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Leaf {
    pub path: Vec<String>,
    pub id: String,
    pub summary: String,
    pub help: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub name: String,
    pub id: String,
    pub description: String,
    pub extension: bool,
    pub leaves: Vec<Leaf>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reference {
    pub name: String,
    pub version: String,
    pub host: String,
    pub description: String,
    pub root: String,
    pub groups: Vec<Group>,
    pub section: Option<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub name: String,
    pub description: String,
    pub arguments: Vec<Argument>,
    pub options: Vec<Opt>,
    pub commands: Vec<Node>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Argument {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub variadic: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Opt {
    pub flags: String,
    pub long: Option<String>,
    pub description: String,
    pub required: bool,
    pub default: Option<Value>,
}

pub fn tree(command: &Command) -> Node {
    let mut options = command
        .options
        .iter()
        .filter(|o| !o.hidden && o.long() != "--help")
        .map(|o| Opt {
            flags: o.flags.clone(),
            long: o
                .flags
                .split(',')
                .map(str::trim)
                .find(|f| f.starts_with("--"))
                .map(|f| f.split_whitespace().next().unwrap_or("").to_owned()),
            description: o.description.clone(),
            required: o.required,
            default: if o.long() == "--query" && o.repeat {
                Some(Value::Array(Vec::new()))
            } else {
                o.default.as_ref().map(|d| Value::from(d.clone()))
            },
        })
        .collect::<Vec<_>>();
    if command.version.is_some() {
        options.insert(
            0,
            Opt {
                flags: "-V, --version".into(),
                long: Some("--version".into()),
                description: "output the version number".into(),
                required: false,
                default: None,
            },
        );
    }
    Node {
        name: command.name.clone(),
        description: command.description.clone(),
        arguments: command
            .arguments
            .iter()
            .map(|a| Argument {
                name: a
                    .syntax
                    .trim_matches(['<', '>', '[', ']'])
                    .trim_end_matches("...")
                    .into(),
                description: a.description.clone(),
                required: a.required,
                variadic: a.variadic,
            })
            .collect(),
        options,
        commands: command
            .commands
            .iter()
            .filter(|c| !c.hidden && c.name != "help")
            .map(tree)
            .collect(),
    }
}
fn node_value(n: &Node) -> Value {
    Value::Object(vec![
        ("name".into(), Value::from(n.name.clone())),
        ("description".into(), Value::from(n.description.clone())),
        (
            "arguments".into(),
            Value::Array(
                n.arguments
                    .iter()
                    .map(|a| {
                        Value::Object(vec![
                            ("name".into(), Value::from(a.name.clone())),
                            ("description".into(), Value::from(a.description.clone())),
                            ("required".into(), Value::Bool(a.required)),
                            ("variadic".into(), Value::Bool(a.variadic)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "options".into(),
            Value::Array(
                n.options
                    .iter()
                    .map(|o| {
                        let mut fields = vec![("flags".into(), Value::from(o.flags.clone()))];
                        if let Some(long) = &o.long {
                            fields.push(("long".into(), Value::from(long.clone())));
                        }
                        fields.push(("description".into(), Value::from(o.description.clone())));
                        fields.push(("required".into(), Value::Bool(o.required)));
                        if let Some(default) = &o.default {
                            fields.push(("default".into(), default.clone()));
                        }
                        Value::Object(fields)
                    })
                    .collect(),
            ),
        ),
        (
            "commands".into(),
            Value::Array(n.commands.iter().map(node_value).collect()),
        ),
    ])
}

pub fn json(
    root: &Command,
    extension_top: &std::collections::BTreeSet<String>,
    has_extension: bool,
) -> Value {
    let mut v = node_value(&tree(root));
    if let Value::Object(fields) = &mut v {
        if has_extension
            && let Some((_, Value::Array(commands))) =
                fields.iter_mut().find(|(name, _)| name == "commands")
        {
            for command in commands {
                let name = if let Value::Object(values) = command {
                    values
                        .iter()
                        .find(|(key, _)| key == "name")
                        .and_then(|(_, value)| value.as_str())
                } else {
                    None
                };
                if let Value::Object(values) = command {
                    values.push((
                        "extension".into(),
                        Value::Bool(name.is_some_and(|n| extension_top.contains(&n))),
                    ));
                }
            }
        }
        fields.push((
            "version".into(),
            Value::from(root.version.clone().unwrap_or_default()),
        ));
    }
    v
}
fn trim(s: &str) -> &str {
    s.trim_end_matches('\n')
}
fn fence(s: &str) -> String {
    format!("```text\n{}\n```", trim(s))
}
pub fn markdown(r: &Reference) -> String {
    let mut parts = vec![format!("# {} command reference", r.name), fence(&r.root)];
    let mut ext = false;
    for g in &r.groups {
        if g.extension
            && !ext
            && let Some((title, note)) = &r.section
        {
            ext = true;
            parts.push(format!("## {title}"));
            parts.push(note.clone());
        }
        parts.push(format!("## {}", g.name));
        parts.push(g.description.clone());
        for l in &g.leaves {
            parts.push(format!("### `{} {}`", r.name, l.path.join(" ")));
            parts.push(fence(&l.help));
        }
    }
    format!("{}\n", parts.join("\n\n"))
}
fn roff(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\e"),
            '-' => out.push_str("\\-"),
            c if !c.is_ascii() => out.push_str(&format!("\\[u{:04X}]", c as u32)),
            _ => out.push(c),
        }
    }
    out
}
fn block(s: &str) -> Vec<String> {
    let mut lines = vec![".nf".into()];
    for line in trim(s).lines() {
        let line = roff(line);
        lines.push(
            if line.starts_with('.') || line.starts_with('\'') || line.is_empty() {
                format!("\\&{line}")
            } else {
                line
            },
        );
    }
    lines.push(".fi".into());
    lines
}
pub fn man(r: &Reference, date: &str) -> String {
    let mut lines = vec![
        format!(
            ".TH {} 1 {} \"{} {}\" \"Purchasely CLI\"",
            r.name.to_uppercase(),
            date,
            r.name,
            r.version
        ),
        ".SH NAME".into(),
        format!("{} \\- {}", r.name, roff(&r.description)),
        ".SH SYNOPSIS".into(),
        format!(".B {}", r.name),
        "<command> [options]".into(),
        ".SH DESCRIPTION".into(),
    ];
    lines.extend(block(&r.root));
    for (extension, heading) in [(false, "COMMANDS"), (true, "MORE COMMANDS")] {
        let groups: Vec<_> = r
            .groups
            .iter()
            .filter(|g| g.extension == extension)
            .collect();
        if groups.is_empty() {
            continue;
        }
        lines.push(format!(".SH {heading}"));
        for g in groups {
            for l in &g.leaves {
                lines.push(format!(
                    ".SS \"{}\"",
                    roff(&format!("{} {}", r.name, l.path.join(" ")))
                ));
                lines.extend(block(&l.help));
            }
        }
    }
    if !r.host.is_empty() {
        lines.extend([
            ".SH \"SEE ALSO\"".into(),
            roff(&format!("{}/reference", r.host)),
        ]);
    }
    format!("{}\n", lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn man_renders_a_two_leaf_tree() {
        let r = sample();
        let s = man(&r, "2026-11-16");
        assert!(s.starts_with(".TH PURCHASELY 1 2026-11-16"));
        assert!(s.contains(".SS \"purchasely apps list\""));
        assert!(s.contains(".SS \"purchasely apps get\""));
    }
    #[test]
    fn markdown_renders_a_two_leaf_tree() {
        let s = markdown(&sample());
        assert!(s.starts_with("# purchasely command reference\n"));
        assert!(s.contains("### `purchasely apps get`"));
        assert!(s.contains("```text\nUsage: purchasely apps get"));
    }
    #[test]
    fn json_renders_tree_names_and_descriptions() {
        let s = crate::core::json::stringify(
            &json(&small_tree(), &std::collections::BTreeSet::new(), false),
            None,
        )
        .unwrap();
        for n in ["purchasely", "apps", "list", "get"] {
            assert!(s.contains(&format!("\"name\":\"{n}\"")));
        }
    }
    fn small_tree() -> Command {
        use crate::cli::Argument;
        Command::new("purchasely").command(
            Command::new("apps")
                .description("Applications")
                .command(Command::new("list").description("List apps"))
                .command(
                    Command::new("get")
                        .description("Get app")
                        .argument(Argument::required("<id>", "app id")),
                ),
        )
    }
    fn sample() -> Reference {
        Reference {
            name: "purchasely".into(),
            version: "1.0.0".into(),
            host: "https://cli.example".into(),
            description: "Purchasely CLI".into(),
            root: "Usage: purchasely".into(),
            section: None,
            groups: vec![Group {
                name: "apps".into(),
                id: "g-apps".into(),
                description: "Applications".into(),
                extension: false,
                leaves: vec![
                    Leaf {
                        path: vec!["apps".into(), "list".into()],
                        id: "apps-list".into(),
                        summary: "List".into(),
                        help: "Usage: purchasely apps list".into(),
                    },
                    Leaf {
                        path: vec!["apps".into(), "get".into()],
                        id: "apps-get".into(),
                        summary: "Get".into(),
                        help: "Usage: purchasely apps get".into(),
                    },
                ],
            }],
        }
    }
}
