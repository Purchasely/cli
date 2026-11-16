//! Owner: P7a
use crate::{
    cli::{Command, OptionDef, Parsed, help},
    core::{
        errors::usage,
        reference::{self, Group, Leaf, Reference},
        reference_html,
        runtime::Runtime,
    },
};

fn collect(root: &Command, rt: &Runtime) -> Reference {
    let mut groups = Vec::new();
    for top in root.commands.iter().filter(|c| !c.hidden) {
        let mut leaves = Vec::new();
        collect_leaves(top, vec![top.name.clone()], root.name.as_str(), &mut leaves);
        leaves.sort_by_key(|left| left.path.join(" "));
        groups.push(Group {
            name: top.name.clone(),
            id: format!("g-{}", top.name),
            description: top.description.clone(),
            extension: rt.extension_top.contains(&top.name),
            leaves,
        });
    }
    groups.sort_by(|left, right| left.name.cmp(&right.name));
    Reference {
        name: rt.d.name.clone(),
        version: rt.d.version.clone(),
        host: rt.d.host_url.clone(),
        description: root.description.clone(),
        root: help::help(root, &[], 100),
        groups,
        section: rt
            .d
            .ext
            .as_ref()
            .map(|e| (e.section.0.into(), e.section.1.into())),
    }
}
fn completed_tree(rt: &mut Runtime) -> Command {
    let mut root = rt
        .command_tree
        .clone()
        .expect("the command tree is finalized before dispatch");
    let Some(modules) = rt.d.ext.as_ref().map(|extension| extension.modules.clone()) else {
        return root;
    };
    let before: std::collections::BTreeSet<_> =
        root.commands.iter().map(|c| c.name.clone()).collect();
    let taken = rt.taken.clone();
    let search_index = rt.search_index.clone();
    let search_extra = rt.search_extra.clone();
    for register in modules {
        register(&mut root, rt);
    }
    rt.extension_top = root
        .commands
        .iter()
        .filter(|command| !before.contains(&command.name))
        .map(|command| command.name.clone())
        .collect();
    rt.taken = taken;
    rt.search_index = search_index;
    rt.search_extra = search_extra;
    root
}
fn collect_leaves(node: &Command, path: Vec<String>, root_name: &str, out: &mut Vec<Leaf>) {
    let mut ancestors = vec![root_name.to_owned()];
    ancestors.extend(path.iter().take(path.len().saturating_sub(1)).cloned());
    let help_text = help::help(
        node,
        &ancestors.iter().map(String::as_str).collect::<Vec<_>>(),
        100,
    );
    let id = path.join("-");
    out.push(Leaf {
        path: path.clone(),
        id,
        summary: node.description.clone(),
        help: help_text,
    });
    for child in node
        .commands
        .iter()
        .filter(|c| !c.hidden && c.name != "help")
    {
        let mut next = path.clone();
        next.push(child.name.clone());
        collect_leaves(child, next, root_name, out);
    }
}
pub fn register(program: &mut Command, rt: &mut Runtime) {
    rt.taken.insert("reference".into());
    let cmd = Command::new("reference")
        .description(
            "Print the full command reference of this version (Markdown, man, HTML or JSON)",
        )
        .option(
            OptionDef::value("--format <format>", "markdown (default), man, html or json")
                .default("markdown"),
        );
    program.commands.push(cmd);
    let leaf = program
        .commands
        .iter_mut()
        .find(|c| c.name == "reference")
        .unwrap();
    *leaf=leaf.clone().after_help("\n--format json prints the tree of commands, arguments and options as JSON, for agents. Hidden commands are left out.\n");
    *leaf = leaf.clone().action(move |p: &Parsed, rt: &mut Runtime| {
        let format = p.opt("format").unwrap_or("markdown");
        if !["markdown", "man", "html", "json"].contains(&format) {
            return Err(usage("--format must be markdown, man, html or json."));
        }
        let tree = completed_tree(rt);
        let text = if format == "json" {
            crate::core::json::stringify_pretty(&reference::json(
                &tree,
                &rt.extension_top,
                rt.d.ext.is_some(),
            ))
            .unwrap_or_else(|_| "{}".into())
                + "\n"
        } else {
            let r = collect(&tree, rt);
            match format {
                "man" => reference::man(&r, &format_date(rt.d.host.now())),
                "html" => reference_html::html(&r),
                _ => reference::markdown(&r),
            }
        };
        rt.d.host.stdout(&text);
        Ok(())
    });
}
fn format_date(now: i64) -> String {
    let days = now.div_euclid(86_400_000);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    y += if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn date_is_utc_calendar_date() {
        assert_eq!(format_date(0), "1970-01-01");
        assert_eq!(format_date(-1), "1969-12-31");
    }
}
