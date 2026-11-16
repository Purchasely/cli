//! Owner: P7a
use crate::{
    cli::{Command, OptionDef, Parsed},
    core::{
        errors::usage,
        json::Value,
        output::{self, EmitOptions},
        runtime::{Runtime, add_globals},
    },
};
use std::collections::BTreeSet;

const STOP: &[&str] = &[
    "the", "a", "an", "of", "to", "for", "in", "on", "and", "or", "my", "all", "me", "with", "by",
    "from", "is", "i", "want", "how", "do",
];
pub fn tokens(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| t.len() > 1 && !STOP.contains(t))
        .map(|t| {
            if let Some(stem) = t.strip_suffix("ies") {
                format!("{stem}y")
            } else if t.len() > 3 {
                t.strip_suffix('s').unwrap_or(t).to_owned()
            } else {
                t.to_owned()
            }
        })
        .collect()
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Item {
    command: String,
    resource: String,
    action: String,
    summary: String,
    description: String,
    method: String,
    path: String,
    scope: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub command: String,
    pub summary: String,
    pub method: String,
    pub path: String,
    pub scope: String,
}
fn walk(node: &Command, path: &mut Vec<String>, root_name: &str, out: &mut Vec<Item>) {
    let children: Vec<_> = node
        .commands
        .iter()
        .filter(|c| !c.hidden && c.name != "help")
        .collect();
    if children.is_empty() && !path.is_empty() {
        let resource = path[0].clone();
        let action = path.iter().skip(1).cloned().collect::<Vec<_>>().join(" ");
        let summary = node.description.clone();
        out.push(Item {
            command: format!("{root_name} {}", path.join(" ")),
            resource,
            action,
            description: summary.clone(),
            summary,
            method: "-".into(),
            path: "-".into(),
            scope: "cli".into(),
        });
    } else {
        for c in children {
            path.push(c.name.clone());
            walk(c, path, root_name, out);
            path.pop();
        }
    }
}
fn score(query: &[String], e: &Item) -> usize {
    let buckets = [
        (tokens(&e.resource), 3),
        (tokens(&e.action), 3),
        (tokens(&e.summary), 2),
        (tokens(&e.description), 1),
        (tokens(&e.scope), 1),
    ];
    query
        .iter()
        .map(|q| {
            buckets
                .iter()
                .map(|(words, w)| if words.contains(q) { *w } else { 0 })
                .sum::<usize>()
        })
        .sum()
}
pub fn search_tree(root: &Command, query: &str, limit: usize) -> Vec<Match> {
    let mut items = Vec::new();
    for c in root.commands.iter().filter(|c| !c.hidden) {
        let mut p = vec![c.name.clone()];
        walk(c, &mut p, &root.name, &mut items);
    }
    let q = tokens(query);
    let mut scored: Vec<_> = items
        .into_iter()
        .map(|e| (score(&q, &e), e))
        .filter(|(s, _)| *s > 0)
        .collect();
    scored.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    scored
        .into_iter()
        .take(limit)
        .map(|(_, e)| Match {
            command: e.command,
            summary: e.summary,
            method: e.method,
            path: e.path,
            scope: e.scope,
        })
        .collect()
}
fn result_value(matches: Vec<Match>) -> Value {
    Value::Array(
        matches
            .into_iter()
            .map(|m| {
                Value::Object(vec![
                    ("command".into(), Value::from(m.command)),
                    ("summary".into(), Value::from(m.summary)),
                    ("method".into(), Value::from(m.method)),
                    ("path".into(), Value::from(m.path)),
                    ("scope".into(), Value::from(m.scope)),
                ])
            })
            .collect(),
    )
}
pub fn register(program: &mut Command, rt: &mut Runtime) {
    rt.taken.insert("search".into());
    let mut command = Command::new("search")
        .description("Find a command for a task")
        .argument(crate::cli::Argument::required(
            "<task>",
            "what you want to do",
        ));
    add_globals(&mut command);
    command
        .options
        .push(OptionDef::value("--limit <n>", "number of results").default("10"));
    command=command.after_help(format!("\nPrints a JSON array, best match first. It needs no credential.\nExample:\n  {} search \"list the placements of an app\"\n",rt.d.name));
    program.commands.push(command);
    let snapshot = program.clone();
    let leaf = program
        .commands
        .iter_mut()
        .find(|c| c.name == "search")
        .unwrap();
    *leaf = leaf.clone().action(move |p: &Parsed, rt: &mut Runtime| {
        let limit = p
            .opt("limit")
            .unwrap_or("10")
            .parse::<usize>()
            .ok()
            .filter(|n| *n > 0)
            .ok_or_else(|| usage("--limit must be a positive integer."))?;
        let task = p.arguments.first().map(String::as_str).unwrap_or("");
        let found = search_tree(&snapshot, task, usize::MAX);
        // API search entries use the same generated flags and operation metadata as help.
        let (_, ops) = crate::management::spec::embedded_spec();
        let generated = crate::management::spec::derive_commands(&ops);
        let mut candidates: Vec<Item> = found
            .into_iter()
            .map(|m| {
                let resource = m.command.split_whitespace().nth(1).unwrap_or("").to_owned();
                let action = m
                    .command
                    .split_whitespace()
                    .skip(2)
                    .collect::<Vec<_>>()
                    .join(" ");
                Item {
                    command: m.command,
                    resource,
                    action,
                    summary: m.summary.clone(),
                    description: m.summary,
                    method: m.method,
                    path: m.path,
                    scope: m.scope,
                }
            })
            .collect();
        // These commands register after search, but the TypeScript command walk runs at search time.
        for (command, summary) in [
            ("purge", "Delete the cached client data"),
            (
                "doctor",
                "Check the install, the key, the network and the version",
            ),
        ] {
            let full_command = format!("{} {command}", rt.d.name);
            if candidates.iter().any(|item| item.command == full_command) {
                continue;
            }
            candidates.push(Item {
                command: full_command,
                resource: command.into(),
                action: String::new(),
                summary: summary.into(),
                description: summary.into(),
                method: "-".into(),
                path: "-".into(),
                scope: "cli".into(),
            });
        }
        let existing: BTreeSet<_> = candidates.iter().map(|m| m.command.clone()).collect();
        let taken: BTreeSet<_> = rt.taken.iter().cloned().collect();

        // The summary command is handwritten and carries its own index entry in the TS runtime.
        if taken.contains("dashboard summary") {
            candidates.push(Item {
                command: format!("{} dashboard summary --app app_xxx --output table", rt.d.name),
                resource: "dashboard".into(),
                action: "summary".into(),
                summary: "Analytics dashboard of an app: key metrics with trend charts".into(),
                description: "Dashboard of one app in the terminal: MRR, active subscriptions, paywall unique viewers and conversions over the last days, with a sparkline chart per metric (--output table). Overview of revenue, subscriptions, conversion and stats".into(),
                method: "GET".into(),
                path: "-".into(),
                scope: "analytics.dashboards:view".into(),
            });
        }
        for g in generated {
            let command = crate::management::help::usage_string(&g, &rt.d.name);
            if existing.contains(&command) {
                continue;
            }
            let op = g.app_op.as_ref().or(g.global_op.as_ref());
            let Some(op) = op else { continue };
            candidates.push(Item {
                command,
                resource: g.resource,
                action: g.action,
                summary: g.summary,
                description: op.description.clone(),
                method: op.method.clone(),
                path: op.path.clone(),
                scope: op.scope.clone(),
            });
        }
        let demo = "Try the CLI with no key: sample data of a fictional app, for a first test";
        candidates.push(Item {
            command: format!("{} demo", rt.d.name),
            resource: "demo".into(),
            action: String::new(),
            summary: demo.into(),
            description: demo.into(),
            method: "-".into(),
            path: "-".into(),
            scope: "cli".into(),
        });
        let q = tokens(task);
        candidates.retain(|m| score(&q, m) > 0);
        candidates.sort_by(|left, right| {
            score(&q, right)
                .cmp(&score(&q, left))
                .then_with(|| (left.scope == "cli").cmp(&(right.scope == "cli")))
        });
        candidates.truncate(limit);
        let env = p.opt("env").unwrap_or("prod").to_owned();
        let value = result_value(candidates.into_iter().map(|m| Match {
            command: m.command,
            summary: m.summary,
            method: m.method,
            path: m.path,
            scope: m.scope,
        }).collect());
        output::emit(
            &rt.d,
            &value,
            &EmitOptions {
                table: p.opt("output") == Some("table"),
                out: p.opt("out").map(str::to_owned),
                env,
                key: None,
                name: "search".into(),
                columns: None,
                width: rt.d.columns.unwrap_or(120) as usize,
            },
        );
        Ok(())
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn search_tokens_match_basic_plural_and_stop_words() {
        assert_eq!(
            tokens("How do I list placements for apps"),
            vec!["list", "placement", "app"]
        );
    }
    #[test]
    fn search_scores_command_names_above_descriptions() {
        let tree = Command::new("purchasely").command(
            Command::new("apps").command(Command::new("list").description("List available items")),
        );
        let matches = search_tree(&tree, "apps list", 10);
        assert_eq!(
            matches.first().map(|m| m.command.as_str()),
            Some("purchasely apps list")
        );
    }
}
