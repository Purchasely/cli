//! Owner: P6b
//! Shared guardrails, dry-run, confirmation, plan, and request execution.

use super::request::Built;
use crate::{
    cli::Parsed,
    core::{
        demo,
        errors::{self, CliError, EXIT, usage},
        http::{self, Ctx, HttpRequest, HttpResponse},
        json::{self, Value},
        output::{self, EmitOptions},
        runtime::Runtime,
        time,
    },
};
use std::{fs, path::Path};

#[derive(Clone, Debug)]
pub struct Globals {
    pub env: Option<String>,
    pub dry_run: bool,
    pub yes: bool,
    pub reason: Option<String>,
    pub rate: Option<String>,
    pub allow_rate: bool,
    pub plan: Option<String>,
    pub output: Option<String>,
    pub out: Option<String>,
    pub confirm_token: Option<String>,
}
impl Globals {
    pub fn from(p: &Parsed) -> Self {
        Self {
            env: p.opt("env").map(str::to_owned),
            dry_run: p.flag("dry-run"),
            yes: p.flag("yes"),
            reason: p.opt("reason").map(str::to_owned),
            rate: p.opt("rate").map(str::to_owned),
            allow_rate: p.flag("i-asked-backend"),
            plan: p.opt("plan").map(str::to_owned),
            output: p.opt("output").map(str::to_owned),
            out: p.opt("out").map(str::to_owned),
            confirm_token: p.opt("confirm-token").map(str::to_owned),
        }
    }
}

/// TS `Meta`: the `--fields` columns of a table, and the not-found message of a generated command.
#[derive(Clone, Debug, Default)]
pub struct Meta {
    pub columns: Option<Vec<String>>,
    pub not_found: Option<String>,
}

/// Execute a read through the normal HTTP path or the local demo fixture.
pub fn read_request(
    rt: &Runtime,
    ctx: &Ctx<'_>,
    req: &HttpRequest,
) -> Result<HttpResponse, CliError> {
    if demo::enabled(&rt.d) {
        if req.method != "GET" {
            return Err(demo::write_error());
        }
        let (status, value) = demo::response(req, &rt.d.name, rt.d.host.wall_now());
        Ok(HttpResponse {
            status,
            headers: vec![("content-type".into(), "application/json".into())],
            text: json::stringify(&value, None).unwrap_or_else(|_| "null".into()),
        })
    } else {
        http::send(ctx, req)
    }
}

pub fn execute(
    rt: &mut Runtime,
    p: &Parsed,
    build: impl Fn(&str, Option<&str>) -> Result<Built, CliError>,
    name: &str,
    refuse_redirect: bool,
    meta: Meta,
) -> Result<(), CliError> {
    let g = Globals::from(p);
    let built = build_first(rt, &g, &build)?;
    let value = perform(rt, &g, built, &build, name, refuse_redirect).map_err(|mut e| {
        if let Some(message) = &meta.not_found
            && e.exit == EXIT::NOT_FOUND
        {
            e.messages.insert(0, message.clone());
        }
        e
    })?;
    let ctx = context(rt, &g)?;
    output::emit(
        &rt.d,
        &value,
        &EmitOptions {
            table: g.output.as_deref() == Some("table"),
            out: g.out,
            env: ctx.env,
            key: ctx.key,
            name: name.into(),
            columns: meta.columns,
            width: rt.d.columns.unwrap_or(120) as usize,
        },
    );
    Ok(())
}

fn context<'a>(rt: &'a Runtime, g: &Globals) -> Result<Ctx<'a>, CliError> {
    let env = rt.env(g.env.as_deref())?;
    let rate = rt.rate("management", g.rate.as_deref(), g.allow_rate)?;
    let key = rt
        .credential(&env, "management")
        .map_err(|e| errors::CliError::new(EXIT::GENERIC, "CREDENTIAL_ERROR", vec![e]))?
        .map(|c| c.key);
    Ok(Ctx {
        d: &rt.d,
        env,
        surface: "management".into(),
        key,
        identity: rt.cfg.identity_email.clone(),
        reason: g.reason.clone(),
        command: rt.command.clone(),
        rate,
        headers: vec![],
        read_only: rt.read_only(),
        timeout_ms: None,
    })
}

fn build_first(
    rt: &Runtime,
    g: &Globals,
    build: &impl Fn(&str, Option<&str>) -> Result<Built, CliError>,
) -> Result<Built, CliError> {
    let env = rt.env(g.env.as_deref())?;
    let built = build(&env, g.confirm_token.as_deref())?;
    if built.op.method != "GET" && g.env.is_none() {
        return Err(errors::CliError::new(
            EXIT::REFUSED,
            "REFUSED",
            vec!["A write needs --env prod or --env staging. Add it, also with --dry-run.".into()],
        ));
    }
    if built.op.method != "GET" {
        allowlist(rt, &env, &built.url)?;
    }
    Ok(built)
}

pub(crate) fn perform(
    rt: &mut Runtime,
    g: &Globals,
    first: Built,
    build: &impl Fn(&str, Option<&str>) -> Result<Built, CliError>,
    name: &str,
    refuse_redirect: bool,
) -> Result<Value, CliError> {
    if g.plan.is_some() && g.dry_run {
        return Err(usage("Use --plan or --dry-run, not both."));
    }
    if g.plan.is_some() && first.op.method == "GET" {
        return Err(usage("--plan records a write. Run a GET without it."));
    }
    let ctx = context(rt, g)?;
    if let Some(file) = &g.plan {
        let r = build(&ctx.env, None)?;
        let number = append_plan(file, &plan_entry(name, &ctx.env, &r))?;
        return Ok(Value::Object(vec![
            (
                "planned".into(),
                Value::from(
                    // TS `resolve()`: absolute, no symlink resolution.
                    std::path::absolute(file)
                        .unwrap_or_else(|_| file.into())
                        .to_string_lossy()
                        .into_owned(),
                ),
            ),
            ("number".into(), Value::Number(number as f64)),
            ("method".into(), Value::from(r.op.method)),
            ("url".into(), Value::from(r.url)),
            (
                "body".into(),
                r.body
                    .as_deref()
                    .map(|s| json::parse(s).unwrap_or(Value::Null))
                    .unwrap_or(Value::Null),
            ),
        ]));
    }
    if g.dry_run {
        let mut h = vec![
            (
                "Authorization".into(),
                Value::from("Bearer <redacted>".to_owned()),
            ),
            ("Accept".into(), Value::from("application/json".to_owned())),
            ("User-Agent".into(), Value::from(http::ctx_user_agent(&ctx))),
        ];
        if first.body.is_some() {
            h.push((
                "Content-Type".into(),
                Value::from("application/json".to_owned()),
            ));
        }
        return Ok(Value::Object(vec![
            ("method".into(), Value::from(first.op.method)),
            ("url".into(), Value::from(first.url)),
            ("headers".into(), Value::Object(h)),
            (
                "body".into(),
                first
                    .body
                    .as_deref()
                    .map(|s| json::parse(s).unwrap_or(Value::Null))
                    .unwrap_or(Value::Null),
            ),
        ]));
    }
    let mut current = first;
    let mut sent_token = g.confirm_token.is_some();
    loop {
        let req = HttpRequest {
            method: current.op.method.clone(),
            url: current.url.clone(),
            path_template: current.op.path.clone(),
            body: current.body.clone(),
            write: None,
            refuse_redirect,
        };
        let res = read_request(rt, &ctx, &req)?;
        if (200..300).contains(&res.status) {
            return Ok(if res.text.trim().is_empty() {
                Value::Object(vec![])
            } else {
                json::parse(&res.text).unwrap_or(Value::Object(vec![(
                    "raw".into(),
                    Value::from(res.text.chars().take(2000).collect::<String>()),
                )]))
            });
        }
        let error = response_error(res.status, &res.headers, &res.text);
        let token = error.extra.get("confirm_token").and_then(Value::as_str);
        if error.code != "CONFIRMATION_REQUIRED"
            || sent_token
            || token.is_none()
            || current.op.confirm.is_none()
        {
            return Err(error);
        }
        let impact = error.messages.join(" ");
        rt.d.host.stderr(&format!("Impact: {impact}\n"));
        let approved = g.yes
            || (rt.d.is_tty && {
                let a = rt.d.host.prompt("Apply this change? [y/N] ", false);
                matches!(a.trim().to_ascii_lowercase().as_str(), "y" | "yes")
            });
        if !approved {
            return Err(error);
        }
        current = build(&ctx.env, token.as_deref())?;
        sent_token = true;
    }
}

fn response_error(status: u16, headers: &[(String, String)], text: &str) -> CliError {
    let body = json::parse(text).unwrap_or(Value::Null);
    let code = get(&body, "error")
        .and_then(|v| get(v, "code"))
        .and_then(Value::as_str)
        .unwrap_or_else(|| format!("HTTP_{status}"));
    let msgs = get(&body, "error")
        .and_then(|v| get(v, "messages"))
        .and_then(|v| {
            if let Value::Array(a) = v {
                Some(a)
            } else {
                None
            }
        })
        .filter(|a| !a.is_empty())
        .map(|a| a.iter().map(|v| v.as_str().unwrap_or_default()).collect())
        .unwrap_or_else(|| vec![format!("HTTP {status}")]);
    let mut e = CliError::new(errors::exit_for(Some(&code), status), code, msgs);
    e.status = Some(status);
    if let Some(t) = get(&body, "confirm_token") {
        e.extra.insert("confirm_token".into(), t.clone());
    }
    if status == 429
        && let Some((_, v)) = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("x-ratelimit-reset"))
        && let Ok(n) = v.parse::<f64>()
    {
        e.extra.insert("retry_at".into(), Value::Number(n));
    }
    e
}

pub fn allowlist(rt: &Runtime, env: &str, url: &str) -> Result<(), CliError> {
    if let Some(apps) = rt.cfg.write_allowlist.as_ref().and_then(|v| get(v, env)) {
        let app = url
            .split("/applications/")
            .nth(1)
            .and_then(|s| s.split('/').next())
            .unwrap_or("");
        let app = decode(app);
        let allowed = if let Value::Array(a) = apps {
            a.iter().any(|v| v.as_str().as_deref() == Some(&app))
        } else {
            true
        };
        if !allowed {
            return Err(errors::CliError::new(
                EXIT::REFUSED,
                "REFUSED",
                vec![format!(
                    "{app} is not in write_allowlist.{env} of the config. Writes go to the listed apps only."
                )],
            ));
        }
    }
    Ok(())
}
fn decode(s: &str) -> String {
    let mut out = String::new();
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(v as char);
            i += 3;
            continue;
        }
        out.push(b[i] as char);
        i += 1;
    }
    out
}
fn get<'a>(v: &'a Value, k: &str) -> Option<&'a Value> {
    if let Value::Object(o) = v {
        o.iter().find(|(n, _)| n == k).map(|(_, v)| v)
    } else {
        None
    }
}

pub fn validate_api_path(raw: &str) -> Result<String, CliError> {
    const PREFIX: &str = "/management/v1";
    let path = raw
        .strip_prefix("/management/v1/")
        .map(|s| format!("/{s}"))
        .unwrap_or_else(|| raw.to_owned());
    let bad = || {
        usage(format!(
            "The path must be under {PREFIX}, such as /applications/app_xxx/placements, with no query, \"..\", or encoded \"/\". Got \"{raw}\"."
        ))
    };
    if !path.starts_with('/') || path == "/" {
        return Err(bad());
    }
    for seg in path[1..].split('/') {
        let dec = decode(seg);
        if dec.is_empty()
            || dec == "."
            || dec == ".."
            || dec.bytes().any(|b| {
                b == b'/'
                    || b == b'\\'
                    || b == b'?'
                    || b == b'#'
                    || b == b'%'
                    || b <= 32
                    || b == 127
            })
        {
            return Err(bad());
        }
    }
    Ok(format!("{PREFIX}{path}"))
}

/// TS `writePlan`: pretty JSON, mode 0600 when the file is created.
pub fn write_plan(path: &Path, plan: &Value) -> Result<(), CliError> {
    use std::io::Write;
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options
        .open(path)
        .and_then(|mut f| {
            f.write_all(
                format!("{}\n", json::stringify_pretty(plan).unwrap_or_default()).as_bytes(),
            )
        })
        .map_err(|e| CliError::new(EXIT::GENERIC, "FILE_ERROR", vec![e.to_string()]))
}

pub fn append_plan(file: &str, entry: &Value) -> Result<usize, CliError> {
    let path = Path::new(file);
    let mut plan = if path.exists() {
        read_plan(file)?
    } else {
        Value::Object(vec![
            ("version".into(), Value::Number(1.)),
            ("requests".into(), Value::Array(vec![])),
        ])
    };
    let Some(Value::Array(reqs)) = get_mut(&mut plan, "requests") else {
        return Err(usage(format!(
            "{file} is not a plan file: it needs \"version\": 1 and a \"requests\" array."
        )));
    };
    reqs.push(entry.clone());
    let n = reqs.len();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    write_plan(path, &plan)?;
    Ok(n)
}
pub fn read_plan(file: &str) -> Result<Value, CliError> {
    let s = fs::read_to_string(file)
        .map_err(|_| usage(format!("{file} is not a readable plan file (JSON).")))?;
    let p = json::parse(&s)
        .map_err(|_| usage(format!("{file} is not a readable plan file (JSON).")))?;
    if !matches!(get(&p,"version"),Some(Value::Number(n)) if *n==1.)
        || !matches!(get(&p, "requests"), Some(Value::Array(_)))
    {
        return Err(usage(format!(
            "{file} is not a plan file: it needs \"version\": 1 and a \"requests\" array."
        )));
    }
    Ok(p)
}
fn get_mut<'a>(v: &'a mut Value, k: &str) -> Option<&'a mut Value> {
    if let Value::Object(o) = v {
        o.iter_mut().find(|(n, _)| n == k).map(|(_, v)| v)
    } else {
        None
    }
}
fn plan_entry(name: &str, env: &str, r: &Built) -> Value {
    let mut v = vec![
        ("command".into(), Value::from(name.to_owned())),
        ("env".into(), Value::from(env.to_owned())),
        ("method".into(), Value::from(r.op.method.clone())),
        ("url".into(), Value::from(r.url.clone())),
        ("pathTemplate".into(), Value::from(r.op.path.clone())),
    ];
    if let Some(b) = &r.body {
        v.push(("body".into(), Value::from(b.clone())))
    }
    if let Some(c) = &r.op.confirm {
        v.push(("confirm".into(), Value::from(c.clone())))
    }
    Value::Object(v)
}

pub fn timestamp(rt: &Runtime) -> String {
    time::iso_datetime(rt.d.host.now())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_api_path_escape() {
        assert_eq!(
            validate_api_path("/applications/%2e%2e/admin")
                .unwrap_err()
                .code,
            "USAGE"
        );
    }
    #[test]
    fn accepts_management_prefix() {
        assert_eq!(
            validate_api_path("/management/v1/applications/a").unwrap(),
            "/management/v1/applications/a"
        );
    }
}
