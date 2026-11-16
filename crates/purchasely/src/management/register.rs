//! Adds generated API operations to the runtime command tree.

use super::{
    help,
    spec::{self, Command as Generated, Param},
};
use crate::{
    cli::{Command, OptionDef},
    core::runtime::Runtime,
};

pub fn register(program: &mut Command, runtime: &mut Runtime) {
    let (_, ops) = spec::embedded_spec();
    for generated in spec::derive_commands(&ops) {
        if runtime
            .taken
            .contains(&format!("{} {}", generated.resource, generated.action))
        {
            continue;
        }
        register_one(program, &generated);
        if let Some(alias) = &generated.legacy {
            let group = find_or_create_group(program, alias, true);
            add_leaf(group, &generated);
        }
    }
}

fn register_one(program: &mut Command, generated: &Generated) {
    let group = find_or_create_group(program, &generated.resource, false);
    add_leaf(group, generated);
}

fn find_or_create_group<'a>(program: &'a mut Command, name: &str, hidden: bool) -> &'a mut Command {
    let index = program.commands.iter().position(|c| c.name == name);
    if let Some(index) = index {
        return &mut program.commands[index];
    }
    let mut group = Command::new(name).description(format!("Management API: {name}"));
    if hidden {
        group = group.hide();
    }
    program.commands.push(group);
    program.commands.last_mut().expect("group was appended")
}

fn add_leaf(group: &mut Command, generated: &Generated) {
    if group.commands.iter().any(|c| c.name == generated.action) {
        return;
    }
    let mut leaf = Command::new(&generated.action).description(&generated.summary);
    add_options(&mut leaf, generated);
    let ops = generated
        .app_op
        .iter()
        .chain(generated.global_op.iter())
        .collect::<Vec<_>>();
    if ops.iter().any(|op| op.method != "GET") {
        leaf = leaf.option(OptionDef::value("--plan <file>", "record the write in this plan file, send nothing. A person reads it, then runs `apply <file>`"));
    }
    leaf = leaf.after_help(help::help_text(generated, "purchasely"));
    group.commands.push(leaf);
}

fn add_options(leaf: &mut Command, generated: &Generated) {
    let has_app = generated.app_op.is_some();
    let has_global = generated.global_op.is_some();
    if has_app {
        let desc = if has_global {
            "application public id (optional: omit for all apps)"
        } else {
            "application public id (required)"
        };
        leaf.options
            .push(OptionDef::value("--app <app_id>", desc).required_if(!has_global));
    }
    for option in &generated.options {
        let param =
            find_param(generated, &option.name).expect("generated option has source parameter");
        let appears_app = generated
            .app_op
            .as_ref()
            .is_some_and(|op| op.params.iter().any(|p| p.name == option.name));
        let appears_global = generated
            .global_op
            .as_ref()
            .is_some_and(|op| op.params.iter().any(|p| p.name == option.name));
        let where_text = if has_app && has_global && appears_app != appears_global {
            if appears_app {
                ", only with --app"
            } else {
                ", only without --app"
            }
        } else {
            ""
        };
        let array = param.schema.kind.as_deref() == Some("array");
        let desc = format!(
            "{} [{}{}{}]{}",
            param.description.as_deref().unwrap_or(""),
            help::type_label(&param.schema),
            if param.required { ", required" } else { "" },
            where_text,
            if array {
                " comma separated or repeated"
            } else {
                ""
            }
        );
        let flags = if param.schema.kind.as_deref() == Some("boolean") {
            format!("--{}", option.flag)
        } else {
            format!(
                "--{} <{}>",
                option.flag,
                if array { "a,b" } else { "value" }
            )
        };
        let mut definition = if param.schema.kind.as_deref() == Some("boolean") {
            OptionDef::boolean(flags, desc)
        } else {
            OptionDef::value(flags, desc)
        };
        if array {
            definition = definition.repeat();
        }
        if param.required {
            definition = definition.required();
        }
        leaf.options.push(definition);
    }
    let ops = generated
        .app_op
        .iter()
        .chain(generated.global_op.iter())
        .collect::<Vec<_>>();
    if ops.iter().any(|op| op.body.is_some()) {
        leaf.options.push(OptionDef::value(
            "--data <json>",
            "request body: JSON, @file.json or - for stdin",
        ));
    }
    if ops.iter().any(|op| op.confirm.is_some()) {
        leaf.options.push(OptionDef::value(
            "--confirm-token <token>",
            "token from a CONFIRMATION_REQUIRED response",
        ));
    }
}

fn find_param<'a>(command: &'a Generated, name: &str) -> Option<&'a Param> {
    command
        .app_op
        .iter()
        .chain(command.global_op.iter())
        .flat_map(|op| op.params.iter())
        .find(|p| p.name == name)
}

trait RequiredOption {
    fn required_if(self, required: bool) -> Self;
}
impl RequiredOption for OptionDef {
    fn required_if(mut self, required: bool) -> Self {
        self.required = required;
        self
    }
}
