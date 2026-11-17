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
    // Built-in groups first, then the extension ones, each by name.
    groups.sort_by(|a, b| (a.extension, &a.name).cmp(&(b.extension, &b.name)));
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
        let tree = rt
            .command_tree
            .clone()
            .expect("the command tree is finalized before dispatch");
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
    use crate::core::ext::Extension;

    fn leaf(name: &'static str) -> Command {
        Command::new(name)
            .description(format!("the {name}"))
            .option(OptionDef::boolean("--x", "an option"))
            .action(|_: &Parsed, _: &mut Runtime| Ok(()))
    }
    fn zeta_alpha(p: &mut Command, _: &mut Runtime) {
        p.commands.push(leaf("zeta"));
        p.commands.push(leaf("alpha"));
    }
    fn alpha_zeta(p: &mut Command, _: &mut Runtime) {
        p.commands.push(leaf("alpha"));
        p.commands.push(leaf("zeta"));
    }
    fn ext(module: crate::core::ext::Register) -> Extension {
        Extension {
            modules: vec![module],
            surface: "extra",
            surfaces: vec![("extra", "extra")],
            surface_defs: vec![],
            identity: true,
            section: ("More commands", "Only in `acme`."),
            latest: None,
            npm_update: None,
        }
    }
    /// The tree of `name` with `ext`, and its reference.
    fn reference_of(name: &str, ext: Option<Extension>) -> (Command, Reference) {
        let mut d = crate::commands::ai::tests::test_deps();
        d.name = name.into();
        d.ext = ext;
        let mut rt = Runtime::new(d);
        let (root, _) = crate::run::build(&mut rt);
        let r = collect(&root, &rt);
        (root, r)
    }

    /// reference.test.ts "leaves a hidden command out, as --help does".
    #[test]
    fn leaves_a_hidden_command_out() {
        for (name, ext) in [("purchasely", None), ("acme", Some(ext(zeta_alpha)))] {
            let (root, r) = reference_of(name, ext);
            assert!(root.commands.iter().any(|c| c.name == "play"), "{name}");
            assert!(r.groups.iter().all(|g| g.name != "play"), "{name}");
            let tops = r
                .groups
                .iter()
                .flat_map(|g| g.leaves.iter().map(|l| l.path[0].as_str()))
                .collect::<Vec<_>>();
            assert!(!tops.contains(&"play"), "{name}");
            assert!(tops.contains(&"games"), "{name}");
        }
    }

    /// reference.test.ts "sorts groups and commands by path, and does not depend on the registration order".
    #[test]
    fn sorts_groups_and_commands_by_path_whatever_the_registration_order() {
        let (_, a) = reference_of("acme", Some(ext(zeta_alpha)));
        let (_, b) = reference_of("acme", Some(ext(alpha_zeta)));
        assert_eq!(a.groups, b.groups);
        let names = |r: &Reference| {
            r.groups
                .iter()
                .filter(|g| g.extension)
                .map(|g| g.name.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(&a), ["alpha", "zeta"]);
        let (root, real) = reference_of("acme", Some(ext(zeta_alpha)));
        for extension in [false, true] {
            let part = real
                .groups
                .iter()
                .filter(|g| g.extension == extension)
                .collect::<Vec<_>>();
            let mut sorted = part.iter().map(|g| g.name.clone()).collect::<Vec<_>>();
            sorted.sort();
            assert_eq!(
                part.iter().map(|g| g.name.clone()).collect::<Vec<_>>(),
                sorted
            );
            for g in part {
                let paths = g
                    .leaves
                    .iter()
                    .map(|l| l.path.join(" "))
                    .collect::<Vec<_>>();
                let mut sorted = paths.clone();
                sorted.sort();
                assert_eq!(paths, sorted, "{}", g.name);
            }
        }
        // The built-in groups all come before the groups of the extension.
        assert_eq!(
            real.groups.iter().position(|g| g.extension),
            Some(real.groups.iter().filter(|g| !g.extension).count())
        );
        // The running tree keeps its registration order.
        let order = root
            .commands
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>();
        let zeta = order.iter().position(|n| *n == "zeta").unwrap();
        assert_eq!(order[zeta + 1], "alpha");
    }

    #[test]
    fn date_is_utc_calendar_date() {
        assert_eq!(format_date(0), "1970-01-01");
        assert_eq!(format_date(-1), "1969-12-31");
    }
}
