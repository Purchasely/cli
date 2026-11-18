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

#[derive(Clone, Debug, Default)]
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
    )?;
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
            rate_limited: None,
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

/// TS `checkAllowlist`: when the env has a list, the app in the pathname of the URL must be in it.
pub fn allowlist(rt: &Runtime, env: &str, url: &str) -> Result<(), CliError> {
    let Some(apps) = rt.cfg.write_allowlist.as_ref().and_then(|v| get(v, env)) else {
        return Ok(());
    };
    if matches!(apps, Value::Null | Value::Bool(false)) {
        return Ok(());
    }
    let refused = |m: String| CliError::new(EXIT::REFUSED, "REFUSED", vec![m]);
    // TS `/\/applications\/([^/?]+)/.exec(new URL(url).pathname)`.
    let href = fetch_url(url);
    let path = pathname(&href);
    let app = path
        .match_indices("/applications/")
        .map(|(i, m)| path[i + m.len()..].split('/').next().unwrap_or(""))
        .find(|a| !a.is_empty());
    let Some(app) = app else {
        return Err(refused(format!(
            "write_allowlist.{env} is set in the config, and this write targets no app. Writes go to the listed apps only."
        )));
    };
    let app = decode_component(app).ok_or_else(|| {
        CliError::new(
            EXIT::GENERIC,
            "INTERNAL_ERROR",
            vec!["URI malformed".into()],
        )
    })?;
    let allowed = if let Value::Array(a) = apps {
        a.iter().any(|v| v.as_str().as_deref() == Some(&app))
    } else {
        true
    };
    if !allowed {
        return Err(refused(format!(
            "{app} is not in write_allowlist.{env} of the config. Writes go to the listed apps only."
        )));
    }
    Ok(())
}

/// The URL that fetch sends (WHATWG URL parser, http and https only): tab and newline removed, `\\` read as
/// `/`, dot segments resolved, the path and the query percent-encoded, no fragment.
pub(crate) fn fetch_url(url: &str) -> String {
    let url: String = url
        .trim_matches(|c: char| c <= ' ')
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    let Some(after) = url.find("://").map(|i| i + 3) else {
        return url;
    };
    let rest = &url[after..];
    let auth = rest.find(['/', '\\', '?', '#']).unwrap_or(rest.len());
    let origin = &url[..after + auth];
    let rest = rest[auth..].split('#').next().unwrap_or("");
    let (path, query) = match rest.find('?') {
        Some(i) => (&rest[..i], Some(&rest[i + 1..])),
        None => (rest, None),
    };
    let segments: Vec<&str> = path.split(['/', '\\']).skip(1).collect();
    let mut out: Vec<String> = Vec::new();
    for (i, seg) in segments.iter().enumerate() {
        let last = i + 1 == segments.len();
        let lower = seg.to_ascii_lowercase();
        let dots = match lower.as_str() {
            "." | "%2e" => 1,
            ".." | ".%2e" | "%2e." | "%2e%2e" => 2,
            _ => 0,
        };
        if dots == 2 {
            out.pop();
        }
        if dots == 0 {
            out.push(percent_encode(seg, b"\"<>`{}"));
        } else if last {
            out.push(String::new());
        }
    }
    let mut href = format!("{origin}/{}", out.join("/"));
    if let Some(q) = query {
        href.push('?');
        href.push_str(&percent_encode(q, b"\"<>'"));
    }
    href
}

/// The pathname of an absolute URL: from the first `/` after the host to the query.
pub(crate) fn pathname(url: &str) -> &str {
    let rest = url.find("://").map_or(url, |i| &url[i + 3..]);
    let rest = rest.find('/').map_or("", |i| &rest[i..]);
    rest.split(['?', '#']).next().unwrap_or("")
}

/// Encode the C0 controls, the space, the bytes of `extra` and every byte above 0x7E. A `%` stays.
fn percent_encode(s: &str, extra: &[u8]) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b <= b' ' || b > 0x7e || extra.contains(&b) {
            out.push_str(&format!("%{b:02X}"));
        } else {
            out.push(b as char);
        }
    }
    out
}

/// TS `decodeURIComponent`: `None` where it throws (a bad `%` sequence, or bytes that are not UTF-8).
pub(crate) fn decode_component(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = s.get(i + 1..i + 3)?;
            if !hex.bytes().all(|h| h.is_ascii_hexdigit()) {
                return None;
            }
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
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
        let Some(dec) = decode_component(seg) else {
            return Err(bad());
        };
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
    /// Oracle: `new URL(u).href` in Node 22, without the fragment (fetch does not send it).
    #[test]
    fn fetch_url_matches_the_whatwg_parser() {
        for (raw, href) in [
            (
                "https://a.io/x/./y/../z?q=a b\"é#f",
                "https://a.io/x/z?q=a%20b%22%C3%A9",
            ),
            ("https://a.io/x/%2E%2e/y/.", "https://a.io/y/"),
            ("https://a.io/x\\y/{c}é", "https://a.io/x/y/%7Bc%7D%C3%A9"),
            ("https://a.io", "https://a.io/"),
            ("https://a.io/a/..", "https://a.io/"),
            (" https://a.io/a/b\tc?x='1'", "https://a.io/a/bc?x=%271%27"),
        ] {
            assert_eq!(fetch_url(raw), href, "{raw}");
        }
        assert_eq!(decode_component("caf%C3%A9").as_deref(), Some("café"));
        assert_eq!(decode_component("%FF"), None);
        assert_eq!(decode_component("%+1"), None);
    }
    #[test]
    fn accepts_management_prefix() {
        assert_eq!(
            validate_api_path("/management/v1/applications/a").unwrap(),
            "/management/v1/applications/a"
        );
    }
}
