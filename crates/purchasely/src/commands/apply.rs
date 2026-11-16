//! Owner: P6b

use crate::{
    cli::{Argument, Command, Parsed},
    core::{
        errors::{CliError, EXIT, usage},
        json::{self, Value},
        output::{self, EmitOptions},
        runtime::Runtime,
        time,
    },
    management::{
        exec::{self, Globals},
        request::Built,
        spec::Op,
    },
};

pub fn register(program: &mut Command, _: &mut Runtime) {
    let mut c = Command::new("apply")
        .description("Send the writes of a plan file, after a person reads them")
        .argument(Argument::required("<file>", "plan file from --plan"));
    c.options.extend(
        super::api::globals()
            .into_iter()
            .filter(|o| o.key() != "plan"),
    );
    c=c.after_help("A plan file holds writes that a command recorded with --plan <file>. An agent can record them\nwith PURCHASELY_READ_ONLY=1 or with no key: nothing is sent. A person reads the plan, then applies it.\n\nThe CLI lists the writes and asks once (or --yes). A risky change still asks on its own in a terminal;\nwithout a terminal, apply stops there (exit 8). Each applied write is marked in the file, so a second\nrun sends only the rest. apply stops at the first error: exit 9 when some writes were applied.\n\nExamples:\n  purchasely placements update --app app_xxx --id plc_xxx --data @placement.json --env prod --plan plan.json\n  purchasely apply plan.json --dry-run\n  purchasely apply plan.json");
    c = c.action(run);
    program.commands.push(c)
}

fn run(p: &Parsed, rt: &mut Runtime) -> Result<(), CliError> {
    let file = p
        .arguments
        .first()
        .ok_or_else(|| usage("apply needs a plan file"))?;
    let mut plan = exec::read_plan(file)?;
    let pending: Vec<usize> = get(&plan, "requests")
        .and_then(|v| {
            if let Value::Array(a) = v {
                Some(a)
            } else {
                None
            }
        })
        .ok_or_else(|| usage("The plan has no requests array."))?
        .iter()
        .enumerate()
        .filter(|(_, r)| !get(r, "applied_at").is_some_and(truthy))
        .map(|(i, _)| i)
        .collect();
    for &i in &pending {
        let r = &get(&plan, "requests")
            .and_then(|v| {
                if let Value::Array(a) = v {
                    Some(a[i].clone())
                } else {
                    None
                }
            })
            .unwrap();
        validate(r, i + 1)?;
    }
    let list = Value::Array(
        pending
            .iter()
            .map(|&i| {
                let r = get(&plan, "requests")
                    .and_then(|v| {
                        if let Value::Array(a) = v {
                            Some(&a[i])
                        } else {
                            None
                        }
                    })
                    .unwrap();
                Value::Object(vec![
                    ("number".into(), Value::Number((i + 1) as f64)),
                    (
                        "command".into(),
                        get(r, "command").cloned().unwrap_or(Value::Null),
                    ),
                    ("env".into(), get(r, "env").cloned().unwrap_or(Value::Null)),
                    (
                        "method".into(),
                        get(r, "method").cloned().unwrap_or(Value::Null),
                    ),
                    ("url".into(), get(r, "url").cloned().unwrap_or(Value::Null)),
                    (
                        "body".into(),
                        get(r, "body")
                            .and_then(Value::as_str)
                            .and_then(|s| json::parse(&s).ok())
                            .unwrap_or(Value::Null),
                    ),
                ])
            })
            .collect(),
    );
    let g = Globals::from(p);
    if g.dry_run || pending.is_empty() {
        return emit(
            rt,
            &g,
            Value::Object(vec![("pending".into(), list)]),
            "apply",
        );
    }
    for &i in &pending {
        let r = get(&plan, "requests")
            .and_then(|v| {
                if let Value::Array(a) = v {
                    Some(&a[i])
                } else {
                    None
                }
            })
            .unwrap();
        rt.d.host.stderr(&format!(
            "#{} {} {} {}\n",
            i + 1,
            strv(r, "env"),
            strv(r, "method"),
            strv(r, "url")
        ));
    }
    let approved = g.yes
        || (rt.d.is_tty
            && matches!(
                rt.d.host
                    .prompt(&format!("Apply {} writes? [y/N] ", pending.len()), false)
                    .trim()
                    .to_ascii_lowercase()
                    .as_str(),
                "y" | "yes"
            ));
    if !approved {
        return Err(CliError::new(
            EXIT::CONFIRMATION,
            "CONFIRMATION_REQUIRED",
            vec![format!(
                "The plan has {} writes to apply. Read them (apply --dry-run), then run again with --yes, or answer y in a terminal.",
                pending.len()
            )],
        ));
    }
    let mut applied = Vec::new();
    for &i in &pending {
        let r = get(&plan, "requests")
            .and_then(|v| {
                if let Value::Array(a) = v {
                    Some(a[i].clone())
                } else {
                    None
                }
            })
            .unwrap();
        let env = strv(&r, "env");
        let method = strv(&r, "method");
        let url = strv(&r, "url");
        let body = get(&r, "body").and_then(Value::as_str);
        let confirm = get(&r, "confirm").and_then(Value::as_str);
        let path = get(&r, "pathTemplate")
            .and_then(Value::as_str)
            .unwrap_or_else(|| "/management/v1/{id}".to_owned());
        exec::allowlist(rt, &env, &url)?;
        let op = Op {
            method: method.clone(),
            path,
            tag: String::new(),
            summary: String::new(),
            description: String::new(),
            scope: String::new(),
            params: vec![],
            body: None,
            confirm: confirm.clone(),
            app: false,
        };
        let base = Built {
            op: op.clone(),
            url: url.clone(),
            body: body.clone(),
        };
        let op_for_build = op.clone();
        let mut globals = g.clone();
        globals.env = Some(env);
        globals.yes = false;
        globals.dry_run = false;
        globals.plan = None;
        let build = |_: &str, token: Option<&str>| {
            let (url, body) = crate::management::request::with_token(
                confirm.as_deref(),
                &url,
                body.as_deref(),
                token,
            )?;
            Ok(Built {
                op: op_for_build.clone(),
                url,
                body,
            })
        };
        match exec::perform(rt, &globals, base, &build, &strv(&r, "command"), false) {
            Ok(response) => {
                if let Some(Value::Array(reqs)) = get_mut(&mut plan, "requests")
                    && let Value::Object(o) = &mut reqs[i]
                {
                    o.retain(|(k, _)| k != "applied_at");
                    o.push((
                        "applied_at".into(),
                        Value::from(time::iso_datetime(rt.d.host.now())),
                    ));
                }
                write_plan(file, &plan)?;
                applied.push(Value::Object(vec![
                    ("number".into(), Value::Number((i + 1) as f64)),
                    (
                        "command".into(),
                        get(&r, "command").cloned().unwrap_or(Value::Null),
                    ),
                    ("response".into(), response),
                ]));
            }
            Err(e) => {
                let count = applied.len();
                let code = e.code.clone();
                let out = Value::Object(vec![
                    ("applied".into(), Value::Array(applied)),
                    (
                        "failed".into(),
                        Value::Object(vec![
                            ("number".into(), Value::Number((i + 1) as f64)),
                            (
                                "command".into(),
                                get(&r, "command").cloned().unwrap_or(Value::Null),
                            ),
                            // TS `e.toJSON().error`: the inner object, without the confirm token.
                            (
                                "error".into(),
                                get(&e.to_value(), "error").cloned().unwrap_or(Value::Null),
                            ),
                        ]),
                    ),
                ]);
                let _ = emit(rt, &g, out, "apply");
                // TS keeps every message, the extra fields (the confirm token) and the status.
                let mut messages = vec![format!(
                    "Request #{} of the plan failed. {} applied, {} left.",
                    i + 1,
                    count,
                    pending.len() - count
                )];
                messages.extend(e.messages.iter().cloned());
                let mut error = CliError::new(
                    if count == 0 { e.exit } else { EXIT::PARTIAL },
                    code,
                    messages,
                );
                error.extra = e.extra.clone();
                error.status = e.status;
                return Err(error);
            }
        }
    }
    emit(
        rt,
        &g,
        Value::Object(vec![("applied".into(), Value::Array(applied))]),
        "apply",
    )
}

fn validate(r: &Value, n: usize) -> Result<(), CliError> {
    let bad = |s: String| {
        CliError::new(
            EXIT::REFUSED,
            "REFUSED",
            vec![format!(
                "Request #{n} of the plan {s}. The CLI sent nothing."
            )],
        )
    };
    let env = strv(r, "env");
    if env != "prod" && env != "staging" {
        return Err(bad(format!("has env \"{env}\", not prod or staging")));
    }
    let method = strv(r, "method");
    if !["POST", "PUT", "PATCH", "DELETE"].contains(&method.as_str()) {
        return Err(bad(format!("has method \"{method}\", not a write")));
    }
    let url = strv(r, "url");
    let expected = if env == "prod" {
        "https://api.purchasely.io/management/v1/"
    } else {
        "https://api-staging.purchasely.io/management/v1/"
    };
    if !url.starts_with(expected) {
        return Err(bad(format!(
            "goes to {url}, not to the Management API of {env}"
        )));
    }
    if let Some(v) = get(r, "body") {
        let Some(b) = v.as_str() else {
            return Err(bad("has a body that is not a JSON string".into()));
        };
        if json::parse(&b).is_err() {
            return Err(bad("has a body that is not a JSON string".into()));
        }
    }
    if let Some(c) = get(r, "confirm").and_then(Value::as_str)
        && c != "query"
        && c != "body"
    {
        return Err(bad("has an unknown confirm placement".into()));
    }
    Ok(())
}
fn write_plan(file: &str, v: &Value) -> Result<(), CliError> {
    exec::write_plan(std::path::Path::new(file), v)
}
fn emit(rt: &Runtime, g: &Globals, v: Value, name: &str) -> Result<(), CliError> {
    let env = rt.env(g.env.as_deref()).unwrap_or_else(|_| "prod".into());
    output::emit(
        &rt.d,
        &v,
        &EmitOptions {
            table: false,
            out: g.out.clone(),
            env,
            key: None,
            name: name.into(),
            columns: None,
            width: 120,
        },
    );
    Ok(())
}
fn get<'a>(v: &'a Value, k: &str) -> Option<&'a Value> {
    if let Value::Object(o) = v {
        o.iter().find(|(n, _)| n == k).map(|(_, x)| x)
    } else {
        None
    }
}
fn get_mut<'a>(v: &'a mut Value, k: &str) -> Option<&'a mut Value> {
    if let Value::Object(o) = v {
        o.iter_mut().find(|(n, _)| n == k).map(|(_, x)| x)
    } else {
        None
    }
}
fn strv(v: &Value, k: &str) -> String {
    get(v, k).and_then(Value::as_str).unwrap_or_default()
}
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null | Value::Bool(false) => false,
        Value::Number(n) if *n == 0. => false,
        Value::String(s) if s.is_empty() => false,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn apply_plan_rejects_untrusted_host() {
        let r = json::parse(
            r#"{"env":"prod","method":"DELETE","url":"https://evil.test/management/v1/x"}"#,
        )
        .unwrap();
        assert_eq!(validate(&r, 1).unwrap_err().exit, EXIT::REFUSED);
    }
}
