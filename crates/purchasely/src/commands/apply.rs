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

pub fn register(program: &mut Command, rt: &mut Runtime) {
    let mut c = Command::new("apply")
        .description("Send the writes of a plan file, after a person reads them")
        .argument(Argument::required("<file>", "plan file from --plan"));
    c.options.extend(
        super::api::globals()
            .into_iter()
            .filter(|o| o.key() != "plan"),
    );
    c=c.after_help("\nA plan file holds writes that a command recorded with --plan <file>. An agent can record them\nwith PURCHASELY_READ_ONLY=1 or with no key: nothing is sent. A person reads the plan, then applies it.\n\nThe CLI lists the writes and asks once (or --yes). A risky change still asks on its own in a terminal;\nwithout a terminal, apply stops there (exit 8). Each applied write is marked in the file, so a second\nrun sends only the rest. apply stops at the first error: exit 9 when some writes were applied.\n\nExamples:\n  purchasely placements update --app app_xxx --id plc_xxx --data @placement.json --env prod --plan plan.json\n  purchasely apply plan.json --dry-run\n  purchasely apply plan.json".replace("purchasely ", &format!("{} ", rt.d.name)));
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
    // The destination as fetch sends it, checked once: the dry run, the approval and the send all show it.
    let mut urls = Vec::new();
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
        urls.push(validate(r, i + 1)?);
    }
    let list = Value::Array(
        pending
            .iter()
            .zip(&urls)
            .map(|(&i, url)| {
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
                    ("url".into(), Value::from(url.clone())),
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
    for (&i, url) in pending.iter().zip(&urls) {
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
            "#{} {} {} {url}\n",
            i + 1,
            strv(r, "env"),
            strv(r, "method"),
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
    for (&i, url) in pending.iter().zip(&urls) {
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
        let url = url.clone();
        let body = get(&r, "body").and_then(Value::as_str);
        let confirm = get(&r, "confirm").and_then(Value::as_str);
        let path = get(&r, "pathTemplate")
            .and_then(Value::as_str)
            .unwrap_or_else(|| "/management/v1/{id}".to_owned());
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
        let env_of_request = env.clone();
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
        // TS: the allowlist check of `perform` is inside the per-request try/catch.
        let sent = exec::allowlist(rt, &env_of_request, &url)
            .and_then(|_| exec::perform(rt, &globals, base, &build, &strv(&r, "command"), true));
        match sent {
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
                emit(rt, &g, out, "apply")?;
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

/// Returns the URL as the WHATWG parser writes it (as fetch sends it), checked after normalisation.
fn validate(r: &Value, n: usize) -> Result<String, CliError> {
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
    // A dot segment can remove the Management prefix: check the URL that is sent, not the stored one.
    let raw = strv(r, "url");
    let url = exec::fetch_url(&raw);
    let expected = if env == "prod" {
        "https://api.purchasely.io/management/v1/"
    } else {
        "https://api-staging.purchasely.io/management/v1/"
    };
    // TS checks the stored string: keep that check, and also check the URL that is sent.
    if !raw.starts_with(expected) || !url.starts_with(expected) {
        return Err(bad(format!(
            "goes to {raw}, not to the Management API of {env}"
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
    // TS: `r.confirm !== undefined && r.confirm !== "query" && r.confirm !== "body"`; a null or a boolean too.
    if let Some(c) = get(r, "confirm")
        && !matches!(c.as_str().as_deref(), Some("query" | "body"))
    {
        return Err(bad("has an unknown confirm placement".into()));
    }
    Ok(url)
}
fn write_plan(file: &str, v: &Value) -> Result<(), CliError> {
    exec::write_plan(std::path::Path::new(file), v)
}
/// TS `emit(rt.d, o, value, { env: "-", key: null, name: "apply" })`: the options of the command line.
fn emit(rt: &Runtime, g: &Globals, v: Value, name: &str) -> Result<(), CliError> {
    output::emit(
        &rt.d,
        &v,
        &EmitOptions {
            table: g.output.as_deref() == Some("table"),
            out: g.out.clone(),
            env: "-".into(),
            key: None,
            name: name.into(),
            columns: None,
            width: rt.d.columns.unwrap_or(120) as usize,
        },
    )
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
    use crate::core::deps::*;
    use std::{cell::RefCell, collections::BTreeMap, path::PathBuf, rc::Rc};
    #[derive(Default)]
    struct Seen {
        urls: Vec<String>,
        stdout: String,
    }
    struct H(Rc<RefCell<Seen>>);
    impl Host for H {
        fn fetch(&self, r: Request) -> Result<Response, NetError> {
            self.0.borrow_mut().urls.push(r.url);
            Ok(Response {
                status: 200,
                body: b"{\"ok\":true}".to_vec(),
                ..Response::default()
            })
        }
        fn exec(&self, _: &str, _: &[String], _: ExecOpts) -> Result<ExecResult, ExecError> {
            Ok(ExecResult::default())
        }
        fn prompt(&self, _: &str, _: bool) -> String {
            String::new()
        }
        fn read_stdin(&self) -> String {
            String::new()
        }
        fn now(&self) -> i64 {
            1_700_000_000_000
        }
        fn wall_now(&self) -> i64 {
            1_700_000_000_000
        }
        fn pid_alive(&self, _: u32) -> bool {
            false
        }
        fn sleep(&self, _: i64) {}
        fn stdout(&self, s: &str) {
            self.0.borrow_mut().stdout.push_str(s)
        }
        fn stderr(&self, _: &str) {}
        fn keychain(&self) -> Option<&dyn Keychain> {
            None
        }
        fn io(&self) -> Option<&dyn Io> {
            None
        }
    }
    /// TS `perform` runs inside the per-request try/catch: an allowlist refusal of request 2, after
    /// request 1 was sent, is a partial failure (exit 9) with the applied/failed summary.
    #[test]
    fn allowlist_refusal_after_a_write_is_partial() {
        let root = std::env::temp_dir().join(format!("apply-allowlist-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let plan = root.join("plan.json");
        let req = |app: &str| {
            format!(
                r#"{{"command":"placements update","env":"prod","method":"PATCH","url":"https://api.purchasely.io/management/v1/applications/{app}/placements/p","body":"{{}}"}}"#
            )
        };
        std::fs::write(
            &plan,
            format!(
                r#"{{"version":1,"requests":[{},{}]}}"#,
                req("allowed"),
                req("blocked")
            ),
        )
        .unwrap();
        let seen = Rc::new(RefCell::new(Seen::default()));
        let mut env = BTreeMap::new();
        env.insert("PURCHASELY_API_KEY".into(), "pk_mgmt_live_SECRET123".into());
        let mut rt = Runtime::new(Deps {
            host: Box::new(H(seen.clone())),
            env,
            is_tty: false,
            config_dir: root.join("config"),
            state_dir: root.join("state"),
            cache_dir: root.join("cache"),
            version: "9.9.9".into(),
            pid: std::process::id(),
            name: "purchasely".into(),
            host_url: "https://cli.test".into(),
            ext: None,
            self_exe: PathBuf::new(),
            platform: "linux",
            stdout_tty: false,
            columns: None,
        });
        rt.cfg.write_allowlist = Some(json::parse(r#"{"prod":["allowed"]}"#).unwrap());
        let mut program = Command::new("purchasely");
        register(&mut program, &mut rt);
        let argv = [
            "apply".to_owned(),
            plan.to_string_lossy().into_owned(),
            "--yes".into(),
        ];
        let p = crate::cli::parse(&program, &argv).unwrap();
        let leaf = program.find(&p.command).unwrap();
        let e = (leaf.action.as_ref().unwrap().0)(&p, &mut rt).unwrap_err();
        assert_eq!((e.exit, e.code.as_str()), (EXIT::PARTIAL, "REFUSED"));
        assert!(e.messages[0].starts_with("Request #2 of the plan failed. 1 applied, 1 left."));
        let seen = seen.borrow();
        assert_eq!(seen.urls.len(), 1);
        let out = json::parse(&seen.stdout).unwrap();
        assert!(matches!(get(&out, "applied"), Some(Value::Array(a)) if a.len() == 1));
        assert_eq!(
            get(get(&out, "failed").unwrap(), "number"),
            Some(&Value::Number(2.0))
        );
        let _ = std::fs::remove_dir_all(root);
    }
    /// A hand-edited plan: fetch sends the URL with its dot segments resolved, so the allowlist checks that
    /// destination, and the request goes to the same one.
    #[test]
    fn allowlist_checks_the_normalized_destination() {
        let root = std::env::temp_dir().join(format!("apply-dots-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let plan = root.join("plan.json");
        let url = |path: &str| {
            format!(
                r#"{{"command":"placements update","env":"prod","method":"PATCH","url":"https://api.purchasely.io/management/v1/applications/{path}","body":"{{}}"}}"#
            )
        };
        let seen = Rc::new(RefCell::new(Seen::default()));
        let mut rt = apply_runtime(&root, seen.clone());
        rt.cfg.write_allowlist = Some(json::parse(r#"{"prod":["allowed"]}"#).unwrap());
        std::fs::write(
            &plan,
            format!(
                r#"{{"version":1,"requests":[{}]}}"#,
                url("allowed/../blocked/placements/p")
            ),
        )
        .unwrap();
        let e = run_apply(&mut rt, &[plan.to_str().unwrap(), "--yes"]).unwrap_err();
        assert_eq!(e.code, "REFUSED");
        assert!(
            e.messages
                .iter()
                .any(|m| m.contains("blocked is not in write_allowlist.prod")),
            "{:?}",
            e.messages
        );
        assert!(seen.borrow().urls.is_empty());
        std::fs::write(
            &plan,
            format!(
                r#"{{"version":1,"requests":[{}]}}"#,
                url("blocked/../allowed/placements/p")
            ),
        )
        .unwrap();
        run_apply(&mut rt, &[plan.to_str().unwrap(), "--yes"]).unwrap();
        assert_eq!(
            seen.borrow().urls,
            ["https://api.purchasely.io/management/v1/applications/allowed/placements/p"]
        );
        let _ = std::fs::remove_dir_all(root);
    }
    fn one_request_plan(name: &str, extra: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("apply-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let plan = root.join("plan.json");
        std::fs::write(
            &plan,
            format!(
                r#"{{"version":1,"requests":[{{"command":"placements update","env":"prod","method":"PATCH","url":"https://api.purchasely.io/management/v1/applications/app_1/placements/p","body":"{{\"name\":\"x\"}}"{extra}}}]}}"#
            ),
        )
        .unwrap();
        plan
    }
    /// TS `check`: a present confirm that is not "query" or "body" refuses the whole plan before any send.
    #[test]
    fn a_confirm_that_is_not_a_placement_refuses_the_plan() {
        for value in ["false", "null", "1"] {
            let plan = one_request_plan("confirm", &format!(r#","confirm":{value}"#));
            let seen = Rc::new(RefCell::new(Seen::default()));
            let mut rt = apply_runtime(plan.parent().unwrap(), seen.clone());
            let e = run_apply(&mut rt, &[plan.to_str().unwrap(), "--dry-run"]).unwrap_err();
            assert_eq!(
                (e.exit, e.messages[0].as_str()),
                (
                    EXIT::REFUSED,
                    "Request #1 of the plan has an unknown confirm placement. The CLI sent nothing."
                ),
                "{value}"
            );
            assert!(seen.borrow().stdout.is_empty());
            let _ = std::fs::remove_dir_all(plan.parent().unwrap());
        }
    }
    /// TS `emit(rt.d, o, ...)`: apply keeps --output table and --out.
    #[test]
    fn apply_keeps_the_output_options() {
        let plan = one_request_plan("output", "");
        let seen = Rc::new(RefCell::new(Seen::default()));
        let mut rt = apply_runtime(plan.parent().unwrap(), seen.clone());
        run_apply(
            &mut rt,
            &[plan.to_str().unwrap(), "--dry-run", "--output", "table"],
        )
        .unwrap();
        let table = seen.borrow().stdout.clone();
        assert!(!table.trim_start().starts_with('{'), "{table}");
        assert!(table.contains("placements update"), "{table}");
        let file = plan.parent().unwrap().join("out.json");
        seen.borrow_mut().stdout.clear();
        run_apply(
            &mut rt,
            &[
                plan.to_str().unwrap(),
                "--dry-run",
                "--out",
                file.to_str().unwrap(),
            ],
        )
        .unwrap();
        assert!(
            std::fs::read_to_string(&file)
                .unwrap()
                .contains("\"pending\"")
        );
        assert!(seen.borrow().stdout.contains("\"file\""));
        let _ = std::fs::remove_dir_all(plan.parent().unwrap());
    }
    fn apply_runtime(root: &std::path::Path, seen: Rc<RefCell<Seen>>) -> Runtime {
        let mut env = BTreeMap::new();
        env.insert("PURCHASELY_API_KEY".into(), "pk_mgmt_live_SECRET123".into());
        Runtime::new(Deps {
            host: Box::new(H(seen)),
            env,
            is_tty: false,
            config_dir: root.join("config"),
            state_dir: root.join("state"),
            cache_dir: root.join("cache"),
            version: "9.9.9".into(),
            pid: std::process::id(),
            name: "purchasely".into(),
            host_url: "https://cli.test".into(),
            ext: None,
            self_exe: PathBuf::new(),
            platform: "linux",
            stdout_tty: false,
            columns: None,
        })
    }
    fn run_apply(rt: &mut Runtime, args: &[&str]) -> Result<(), CliError> {
        let mut program = Command::new("purchasely");
        register(&mut program, rt);
        let mut argv = vec!["apply".to_owned()];
        argv.extend(args.iter().map(|s| (*s).to_owned()));
        let p = crate::cli::parse(&program, &argv).unwrap();
        let leaf = program.find(&p.command).unwrap();
        (leaf.action.as_ref().unwrap().0)(&p, rt)
    }
    /// The prefix check runs on the URL that is sent: dot segments that remove /management/v1 refuse
    /// the plan, and the dry run shows the normalized destination.
    #[test]
    fn apply_checks_the_management_prefix_after_normalisation() {
        let r = json::parse(
            r#"{"env":"prod","method":"DELETE","url":"https://api.purchasely.io/management/v1/../../admin/x"}"#,
        )
        .unwrap();
        let e = validate(&r, 1).unwrap_err();
        assert_eq!(e.exit, EXIT::REFUSED);
        let r = json::parse(
            r#"{"env":"prod","method":"DELETE","url":"https://api.purchasely.io/x/../management/v1/y"}"#,
        )
        .unwrap();
        assert_eq!(validate(&r, 1).unwrap_err().exit, EXIT::REFUSED);
        assert!(
            e.messages[0].contains("not to the Management API of prod"),
            "{:?}",
            e.messages
        );
        let plan = one_request_plan("normalized", "");
        let text = std::fs::read_to_string(&plan)
            .unwrap()
            .replace("applications/app_1/", "applications/x/../app_1/");
        std::fs::write(&plan, text).unwrap();
        let seen = Rc::new(RefCell::new(Seen::default()));
        let mut rt = apply_runtime(plan.parent().unwrap(), seen.clone());
        run_apply(&mut rt, &[plan.to_str().unwrap(), "--dry-run"]).unwrap();
        let out = seen.borrow().stdout.clone();
        assert!(
            out.contains("https://api.purchasely.io/management/v1/applications/app_1/placements/p"),
            "{out}"
        );
        assert!(!out.contains(".."), "{out}");
        let _ = std::fs::remove_dir_all(plan.parent().unwrap());
    }
    #[test]
    fn apply_plan_rejects_untrusted_host() {
        let r = json::parse(
            r#"{"env":"prod","method":"DELETE","url":"https://evil.test/management/v1/x"}"#,
        )
        .unwrap();
        assert_eq!(validate(&r, 1).unwrap_err().exit, EXIT::REFUSED);
    }
}
