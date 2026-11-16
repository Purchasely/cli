//! Adds generated API operations to the runtime command tree.

use super::{
    exec, help, naming, request,
    spec::{self, Command as Generated, Param},
};
use crate::{
    cli::{Command, OptionDef, Parsed},
    core::{
        errors::CliError,
        json::Value,
        runtime::{Runtime, add_globals},
    },
};
use std::collections::BTreeMap;

pub fn register(program: &mut Command, runtime: &mut Runtime) {
    let (_, ops) = spec::embedded_spec();
    for generated in spec::derive_commands(&ops) {
        let op = generated.app_op.as_ref().or(generated.global_op.as_ref());
        if let Some(op) = op {
            runtime
                .search_index
                .push(crate::core::runtime::SearchEntry {
                    command: help::usage_string(&generated, &runtime.d.name),
                    resource: generated.resource.clone(),
                    action: generated.action.clone(),
                    summary: help::plain(&generated.summary),
                    description: op.description.clone(),
                    method: op.method.clone(),
                    path: op.path.clone(),
                    scope: op.scope.clone(),
                });
        }
        if runtime
            .taken
            .contains(&format!("{} {}", generated.resource, generated.action))
        {
            continue;
        }
        let name = runtime.d.name.clone();
        let group = crate::core::runtime::group(program, &generated.resource);
        add_leaf(group, &generated, &name);
        // The old resource of 0.4.0 scripts: it runs, and no help, reference, search or completion shows it.
        if let Some(alias) = &generated.legacy {
            let group = find_or_create_hidden_group(program, alias);
            add_leaf(group, &generated, &name);
        }
    }
    runtime.command_tree = Some(program.clone());
}

fn find_or_create_hidden_group<'a>(program: &'a mut Command, name: &str) -> &'a mut Command {
    if let Some(index) = program.commands.iter().position(|c| c.name == name) {
        return &mut program.commands[index];
    }
    program.commands.push(
        Command::new(name)
            .description(format!("Management API: {name}"))
            .hide(),
    );
    program.commands.last_mut().expect("group was appended")
}

/// TS `leafOf`: the globals, the operation options, `--plan` for a write, the help text and the action.
fn add_leaf(group: &mut Command, generated: &Generated, name: &str) {
    let mut leaf = Command::new(&generated.action).description(help::plain(&generated.summary));
    add_globals(&mut leaf);
    add_operation_options(&mut leaf, generated);
    if ops(generated).any(|op| op.method != "GET") {
        leaf = leaf.option(OptionDef::value("--plan <file>", PLAN_HELP));
    }
    leaf = leaf.after_help(help::help_text(generated, name));
    let generated = generated.clone();
    leaf = leaf.action(move |p, rt| run_op(rt, &generated, p));
    group.commands.push(leaf);
}

pub const PLAN_HELP: &str =
    "record the write in this plan file, send nothing. A person reads it, then runs `apply <file>`";

fn ops(generated: &Generated) -> impl Iterator<Item = &spec::Op> {
    generated.app_op.iter().chain(generated.global_op.iter())
}

/// TS `addOperationOptions`: `--app`, the parameters, `--data` and `--confirm-token`.
pub fn add_operation_options(leaf: &mut Command, generated: &Generated) {
    let has_app = generated.app_op.is_some();
    let has_global = generated.global_op.is_some();
    let suffix = if has_app && !has_global {
        " (required)"
    } else if has_app {
        " (optional: omit for all apps)"
    } else {
        ""
    };
    leaf.options.push(OptionDef::value(
        "--app <app_id>",
        format!("application public id{suffix}"),
    ));
    let in_op = |op: &Option<spec::Op>, name: &str| {
        op.as_ref()
            .is_some_and(|op| op.params.iter().any(|p| p.name == name))
    };
    for option in &generated.options {
        let param =
            find_param(generated, &option.name).expect("generated option has source parameter");
        let in_app = in_op(&generated.app_op, &option.name);
        let in_global = in_op(&generated.global_op, &option.name);
        let where_text = if has_app && has_global && !(in_app && in_global) {
            if in_app {
                ", only with --app"
            } else {
                ", only without --app"
            }
        } else {
            ""
        };
        let array = param.schema.kind.as_deref() == Some("array");
        let boolean = param.schema.kind.as_deref() == Some("boolean");
        let mut parts = Vec::new();
        if let Some(description) = param.description.as_deref().filter(|d| !d.is_empty()) {
            // TS `replace(/\s+/g, " ")`: each run of blanks becomes one space.
            let mut collapsed = String::new();
            for ch in description.chars() {
                if !ch.is_whitespace() {
                    collapsed.push(ch);
                } else if !collapsed.ends_with(' ') {
                    collapsed.push(' ');
                }
            }
            parts.push(help::plain(&collapsed));
        }
        parts.push(format!(
            "[{}{}{}]",
            help::type_label(&param.schema),
            if param.required { ", required" } else { "" },
            where_text
        ));
        if array {
            parts.push("comma separated or repeated".into());
        }
        let flags = if boolean {
            format!("--{}", option.flag)
        } else {
            format!(
                "--{} <{}>",
                option.flag,
                if array { "a,b" } else { "value" }
            )
        };
        let mut definition = if boolean {
            OptionDef::boolean(flags, parts.join(" "))
        } else {
            OptionDef::value(flags, parts.join(" "))
        };
        if array {
            definition = definition.repeat();
        }
        leaf.options.push(definition);
    }
    if ops(generated).any(|op| op.body.is_some()) {
        leaf.options.push(OptionDef::value(
            "--data <json>",
            "request body: JSON, @file.json or - for stdin",
        ));
    }
    if ops(generated).any(|op| op.confirm.is_some()) {
        leaf.options.push(OptionDef::value(
            "--confirm-token <token>",
            "token from a CONFIRMATION_REQUIRED response",
        ));
    }
}

fn find_param<'a>(command: &'a Generated, name: &str) -> Option<&'a Param> {
    ops(command)
        .flat_map(|op| op.params.iter())
        .find(|p| p.name == name)
}

/// TS `runOp`: the values of the options, the body, then one Management request.
fn run_op(rt: &mut Runtime, c: &Generated, p: &Parsed) -> Result<(), CliError> {
    let mut values = BTreeMap::new();
    if let Some(app) = p.opt("app") {
        values.insert("application_id".to_owned(), Value::from(app.to_owned()));
    }
    for option in &c.options {
        // Only a flag given on the command line: the parser fills an absent boolean with false, commander does not.
        if let Some(raw) = p
            .options
            .get(&option.flag)
            .filter(|_| p.option_sources.contains_key(&option.flag))
        {
            let param = find_param(c, &option.name).expect("generated option has source parameter");
            values.insert(option.name.clone(), request::parse_value(param, raw)?);
        }
    }
    let data = match p.opt("data") {
        Some(raw) => Some(request::read_data(
            raw,
            &if raw == "-" {
                rt.d.host.read_stdin()
            } else {
                String::new()
            },
        )?),
        None => None,
    };
    let columns = values.get("fields").and_then(|v| match v {
        Value::Array(items) => Some(items.iter().filter_map(Value::as_str).collect()),
        _ => None,
    });
    let not_found = not_found(c, &values);
    let command = c.clone();
    let build = move |env: &str, token: Option<&str>| {
        request::build_request(&command, &values, env, data.as_deref(), token)
    };
    exec::execute(
        rt,
        p,
        build,
        &format!("{}-{}", c.resource, c.action),
        false,
        exec::Meta { columns, not_found },
    )
}

/// `placement "plc_1" not found in app app_1`, from the path ids of the request. None when the app is the only one.
fn not_found(c: &Generated, values: &BTreeMap<String, Value>) -> Option<String> {
    let app = values.get("application_id").and_then(Value::as_str);
    let op = if app.is_some() {
        &c.app_op
    } else {
        &c.global_op
    };
    let id = op
        .as_ref()?
        .params
        .iter()
        .rfind(|p| p.location == "path" && p.name != "application_id")?;
    let value = match values.get(&id.name) {
        Some(Value::String(s)) => String::from_utf16_lossy(s),
        Some(v) => crate::core::json::stringify(v, None).unwrap_or_default(),
        None => "undefined".into(),
    };
    Some(format!(
        "{} \"{value}\" not found{}",
        naming::singular(&c.resource),
        app.map(|a| format!(" in app {a}")).unwrap_or_default()
    ))
}
