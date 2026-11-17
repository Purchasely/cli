//! Owner: P5b
use crate::{
    cli::{Command, Parsed},
    core::{
        ai,
        credentials::credential_display,
        demo,
        errors::{CliError, EXIT},
        http::{self, Ctx, HttpRequest},
        json::Value,
        output::{EmitOptions, emit},
        runtime::{Runtime, add_globals},
        state::StateStore,
        update::{self, InstallKind},
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Ok,
    Warn,
    Fail,
}
impl Status {
    fn text(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warn => "warn",
            Self::Fail => "fail",
        }
    }
}

fn check(name: &str, status: Status, detail: impl Into<String>) -> Value {
    Value::Object(vec![
        ("check".into(), Value::from(name.to_owned())),
        ("status".into(), Value::from(status.text().to_owned())),
        ("detail".into(), Value::from(detail.into())),
    ])
}
fn value_field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    if let Value::Object(values) = value {
        values.iter().find(|(name, _)| name == key).map(|(_, v)| v)
    } else {
        None
    }
}
fn allowlist_text(value: &Value, env: &str) -> String {
    let Some(Value::Array(items)) = value_field(value, env) else {
        return String::new();
    };
    items
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

fn doctor(p: &Parsed, rt: &mut Runtime) -> Result<(), CliError> {
    let env = rt.env(p.opt("env"))?;
    let demo = demo::enabled(&rt.d);
    let offline = ai::offline(&rt.d);
    let kind = update::install_kind_for(&rt.d);
    let auto_off = rt.d.env.contains_key("CI")
        || rt
            .d
            .env
            .get("PURCHASELY_DISABLE_AUTOUPDATE")
            .is_some_and(|v| v == "1")
        || rt.d.env.get("PURCHASELY_OFFLINE").is_some_and(|v| v == "1")
        || demo
        || rt.cfg.auto_update == Some(false)
        || kind != InstallKind::Managed;
    let mut checks = vec![check(
        "install",
        Status::Ok,
        format!(
            "{} install, automatic update {}",
            match kind {
                InstallKind::Managed => "managed",
                InstallKind::Brew => "brew",
                InstallKind::Npm => "npm",
            },
            if auto_off { "off" } else { "on" }
        ),
    )];

    if demo || offline {
        checks.push(check(
            "version",
            Status::Warn,
            format!(
                "{}. {}=1 is set: no check of the latest version",
                rt.d.version,
                if demo {
                    "PURCHASELY_DEMO"
                } else {
                    "PURCHASELY_OFFLINE"
                }
            ),
        ));
    } else {
        let latest = update::latest_version(&rt.d);
        match latest {
            Ok(Some(version)) if update::newer(&version, &rt.d.version) => checks.push(check(
                "version",
                Status::Warn,
                format!(
                    "{}, and {version} is available. Run `{} update`",
                    rt.d.version, rt.d.name
                ),
            )),
            Ok(_) => checks.push(check(
                "version",
                Status::Ok,
                format!("{}, the latest", rt.d.version),
            )),
            Err(_) => checks.push(check(
                "version",
                Status::Warn,
                format!(
                    "{}. The latest version is not available: check the network",
                    rt.d.version
                ),
            )),
        }
    }

    let cred = rt.credential(&env, "management").ok().flatten();
    if demo {
        checks.push(check("key", Status::Ok, "Demo mode (PURCHASELY_DEMO=1): no key needed. The data is built-in sample data, not real"));
    } else if let Some(cred) = &cred {
        checks.push(check(
            "key",
            Status::Ok,
            format!(
                "{env} key {} from {}",
                credential_display("management", Some(&cred.key)),
                cred.source
            ),
        ));
    } else {
        checks.push(check(
            "key",
            Status::Fail,
            format!(
                "No Management API key for {env}. Run `{} auth login` or set PURCHASELY_API_KEY",
                rt.d.name
            ),
        ));
    }

    if demo {
        checks.push(check(
            "api",
            Status::Ok,
            "Not checked: demo mode sends no request",
        ));
    } else if cred.is_none() {
        checks.push(check("api", Status::Warn, "Not checked: no key"));
    } else if offline {
        checks.push(check(
            "api",
            Status::Warn,
            "Not checked: PURCHASELY_OFFLINE=1 is set",
        ));
    } else {
        let credential = cred.as_ref().unwrap();
        let ctx = Ctx {
            d: &rt.d,
            env: env.clone(),
            surface: "management".into(),
            key: Some(credential.key.clone()),
            identity: rt.cfg.identity_email.clone(),
            reason: Some(String::new()),
            command: Some("doctor".into()),
            rate: rt.rate("management", None, false).unwrap_or(0.8),
            headers: vec![],
            read_only: rt.read_only(),
            timeout_ms: None,
        };
        let base = if env == "staging" {
            "https://api-staging.purchasely.io"
        } else {
            "https://api.purchasely.io"
        };
        let request = HttpRequest {
            method: "GET".into(),
            url: format!("{base}/management/v1/applications?per_page=1"),
            path_template: "/management/v1/applications".into(),
            body: None,
            write: Some(false),
            refuse_redirect: false,
        };
        match http::send(&ctx, &request) {
            Ok(response) if (200..300).contains(&response.status) => checks.push(check("api", Status::Ok, format!("The Management API of {env} accepts the key"))),
            Ok(response) if response.status == 401 => checks.push(check("api", Status::Fail, format!("The Management API of {env} refuses the key (HTTP 401). Check the key and the env"))),
            Ok(response) => checks.push(check("api", Status::Fail, format!("The Management API of {env} answered HTTP {}", response.status))),
            Err(error) => checks.push(check("api", Status::Fail, error.messages.join(" "))),
        }
    }

    let plugin = StateStore::new(&rt.d, "ai").read("plugin", Value::Null);
    let agents = value_field(&plugin, "agents")
        .and_then(|value| {
            if let Value::Object(items) = value {
                Some(
                    items
                        .iter()
                        .map(|(name, _)| name.clone())
                        .collect::<Vec<_>>(),
                )
            } else {
                None
            }
        })
        .unwrap_or_default();
    checks.push(if agents.is_empty() {
        check(
            "ai plugin",
            Status::Warn,
            format!("Not installed with the CLI. Run `{} ai install`", rt.d.name),
        )
    } else {
        check(
            "ai plugin",
            Status::Ok,
            format!("Installed for {}", agents.join(", ")),
        )
    });
    let allow = rt
        .cfg
        .write_allowlist
        .as_ref()
        .map(|value| allowlist_text(value, &env))
        .unwrap_or_default();
    let write_detail = if demo {
        "Demo mode: every write is refused"
    } else if rt.read_only() {
        "Read-only mode"
    } else {
        "Writes allowed"
    };
    checks.push(check(
        "writes",
        Status::Ok,
        if allow.is_empty() {
            write_detail.to_owned()
        } else {
            format!("{write_detail}, write_allowlist.{env}: {allow}")
        },
    ));

    let failures = checks
        .iter()
        .filter(|row| {
            value_field(row, "status")
                .and_then(Value::as_str)
                .as_deref()
                == Some("fail")
        })
        .filter_map(|row| {
            Some(format!(
                "{}: {}",
                value_field(row, "check")?.as_str()?,
                value_field(row, "detail")?.as_str()?
            ))
        })
        .collect::<Vec<_>>();
    emit(
        &rt.d,
        &Value::Object(vec![("checks".into(), Value::Array(checks))]),
        &EmitOptions {
            table: p.opt("output") == Some("table"),
            env,
            key: cred.map(|c| c.key),
            name: "doctor".into(),
            columns: Some(vec!["check".into(), "status".into(), "detail".into()]),
            ..EmitOptions::default()
        },
    );
    if failures.is_empty() {
        Ok(())
    } else {
        Err(CliError::new(EXIT::GENERIC, "DOCTOR_FAILED", failures))
    }
}

pub fn register(program: &mut Command, rt: &mut Runtime) {
    rt.taken.insert("doctor".into());
    let mut command = Command::new("doctor")
        .description("Check the install, the key, the network and the version");
    add_globals(&mut command);
    command = command.after_help(format!("\nChecks the install, the key, the network, the version and the AI plugin. It sends one read request.\nIt never prints the key. Exit 1 when a check fails. Send the output to support when you ask for help.\n\nExamples:\n  {} doctor\n  {} doctor --output table", rt.d.name, rt.d.name));
    program.commands.push(command.action(doctor));
}
