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
        let mut found = search_tree(&snapshot, task, usize::MAX);
        // API commands are sourced from the embedded Management description; skip a generated name already
        // represented by a handwritten leaf.
        let (_, ops) = crate::management::spec::embedded_spec();
        let generated = crate::management::spec::derive_commands(&ops);
        let existing: BTreeSet<_> = found.iter().map(|m| m.command.clone()).collect();
        let taken: BTreeSet<_> = rt.taken.iter().cloned().collect();
        let mut candidates = std::mem::take(&mut found);
        for g in generated {
            if taken.contains(&format!("{} {}", g.resource, g.action)) {
                continue;
            }
            let command = format!("{} {} {}", rt.d.name, g.resource, g.action);
            if existing.contains(&command) {
                continue;
            }
            let op = g.app_op.as_ref().or(g.global_op.as_ref());
            candidates.push(Match {
                command,
                summary: g.summary,
                method: op.map(|o| o.method.clone()).unwrap_or_else(|| "-".into()),
                path: op.map(|o| o.path.clone()).unwrap_or_else(|| "-".into()),
                scope: op
                    .map(|o| o.scope.clone())
                    .unwrap_or_else(|| "management".into()),
            });
        }
        let q = tokens(task);
        candidates.retain(|m| {
            let item = Item {
                command: m.command.clone(),
                resource: m.command.split_whitespace().nth(1).unwrap_or("").into(),
                action: m
                    .command
                    .split_whitespace()
                    .skip(2)
                    .collect::<Vec<_>>()
                    .join(" "),
                summary: m.summary.clone(),
                description: m.summary.clone(),
                method: m.method.clone(),
                path: m.path.clone(),
                scope: m.scope.clone(),
            };
            score(&q, &item) > 0
        });
        candidates.sort_by_key(|m| {
            let item = Item {
                command: m.command.clone(),
                resource: m.command.split_whitespace().nth(1).unwrap_or("").into(),
                action: m
                    .command
                    .split_whitespace()
                    .skip(2)
                    .collect::<Vec<_>>()
                    .join(" "),
                summary: m.summary.clone(),
                description: m.summary.clone(),
                method: m.method.clone(),
                path: m.path.clone(),
                scope: m.scope.clone(),
            };
            std::cmp::Reverse(score(&q, &item))
        });
        candidates.truncate(limit);
        let env = p.opt("env").unwrap_or("prod").to_owned();
        let value = result_value(candidates);
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
