//! HTTP request pipeline: credentials, shared limits, headers, retry, usage, and redirects.
use super::{
    breaker::{Breaker, pause_from_headers},
    credentials::{Surface, hint, key_hash, what},
    deps::{Deps, Request, Response},
    errors::{CliError, EXIT},
    limiter::Limiter,
    state::StateStore,
    usage::{self, UsageLine},
};
pub const REQUEST_TIMEOUT_MS: u64 = 120_000;
#[derive(Clone)]
pub struct Ctx<'a> {
    pub d: &'a Deps,
    pub env: String,
    pub surface: Surface,
    pub key: Option<String>,
    pub identity: Option<String>,
    pub reason: Option<String>,
    pub command: Option<String>,
    pub rate: f64,
    pub headers: Vec<(String, String)>,
    /// Refuse every write before it is sent (`PURCHASELY_READ_ONLY=1` or `read_only` in the config).
    pub read_only: bool,
}
#[derive(Clone, Debug)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub path_template: String,
    pub body: Option<String>,
    pub write: Option<bool>,
    pub refuse_redirect: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub text: String,
}
pub fn user_agent(
    version: &str,
    identity: Option<&str>,
    reason: &str,
    command: &str,
    runner: &str,
) -> String {
    // TS `clean`: each run of `\r\n();` becomes one space, then trim; an empty value is `-`.
    fn clean(s: &str) -> String {
        let mut out = String::new();
        let mut in_run = false;
        for c in s.chars() {
            if "\r\n();".contains(c) {
                if !in_run {
                    out.push(' ');
                }
                in_run = true;
            } else {
                out.push(c);
                in_run = false;
            }
        }
        let out = out.trim();
        if out.is_empty() {
            "-".into()
        } else {
            out.into()
        }
    }
    format!(
        "purchasely-cli/{version} ({}; {}; {}; {})",
        clean(identity.unwrap_or("")),
        clean(reason),
        clean(command),
        runner
    )
}

/// TS `readOnlyError`.
pub fn read_only_error(d: &Deps) -> CliError {
    let why = if d.env.get("PURCHASELY_READ_ONLY").is_some_and(|v| v == "1") {
        "PURCHASELY_READ_ONLY=1 is set"
    } else {
        "read_only is true in the config"
    };
    CliError::new(
        EXIT::REFUSED,
        "REFUSED",
        vec![format!(
            "Read-only mode: {why}. This command writes, so the CLI sent nothing."
        )],
    )
}

/// TS `userAgent(ctx)`: only an extension with `identity` sends the email.
pub fn ctx_user_agent(ctx: &Ctx<'_>) -> String {
    user_agent(
        &ctx.d.version,
        if ctx.d.ext.as_ref().is_some_and(|e| e.identity) {
            ctx.identity.as_deref()
        } else {
            None
        },
        ctx.reason.as_deref().unwrap_or(""),
        ctx.command.as_deref().unwrap_or(""),
        &runner(ctx.d),
    )
}
pub fn refused_redirect_exit() -> i32 {
    EXIT::REFUSED
}
fn uuid() -> String {
    let mut b = [0u8; 16];
    let _ = getrandom::fill(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0],
        b[1],
        b[2],
        b[3],
        b[4],
        b[5],
        b[6],
        b[7],
        b[8],
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15]
    )
}
fn header<'a>(r: &'a Response, k: &str) -> Option<&'a str> {
    r.headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(k))
        .map(|(_, v)| v.as_str())
}
pub fn send(ctx: &Ctx<'_>, req: &HttpRequest) -> Result<HttpResponse, CliError> {
    let key = ctx.key.as_deref().ok_or_else(|| {
        CliError::new(
            EXIT::AUTH,
            "UNAUTHORIZED",
            vec![format!(
                "No {}. {}",
                what(&ctx.surface).unwrap_or(ctx.surface.clone()),
                hint(&ctx.surface, &ctx.d.name).unwrap_or_default()
            )],
        )
    })?;
    let write = req.write.unwrap_or_else(|| req.method != "GET");
    let hash = key_hash(key);
    let state_key = format!("{}-{}-{hash}", ctx.env, ctx.surface);
    let store = StateStore::new(ctx.d, &state_key);
    let limiter = Limiter::new(ctx.d, &store);
    let breaker = Breaker::new(ctx.d, &store);
    if write && ctx.read_only {
        return Err(read_only_error(ctx.d));
    }
    for attempt in 0..2 {
        let factor = breaker.gate()?;
        let permit = limiter
            .acquire(ctx.rate * factor, write)
            .map_err(|e| CliError::new(EXIT::GENERIC, "STATE_ERROR", vec![e]))?;
        let id = uuid();
        let mut headers = Vec::new();
        if !ctx
            .headers
            .iter()
            .any(|(n, _)| n.eq_ignore_ascii_case("Accept"))
        {
            headers.push(("Accept".into(), "application/json".into()));
        }
        if ctx.headers.is_empty() {
            headers.push(("Authorization".into(), format!("Bearer {key}")));
        } else {
            headers.extend(ctx.headers.clone());
        }
        headers.extend([
            ("User-Agent".into(), ctx_user_agent(ctx)),
            ("X-Request-Id".into(), id.clone()),
        ]);
        if req.body.is_some() {
            headers.push(("Content-Type".into(), "application/json".into()))
        }
        let response = ctx.d.host.fetch(Request {
            method: req.method.clone(),
            url: req.url.clone(),
            headers,
            body: req.body.as_ref().map(|s| s.as_bytes().to_vec()),
            timeout_ms: REQUEST_TIMEOUT_MS,
            refuse_redirect: req.refuse_redirect,
        });
        let _ = permit.release();
        let response = response.map_err(|e| {
            // TS: `cause.code`, else the error name. The message is never echoed.
            let cause = match &e.code {
                Some(code) => code.as_str(),
                None if e.timed_out => "Timeout",
                None => "Error",
            };
            CliError::new(
                EXIT::GENERIC,
                "NETWORK_ERROR",
                vec![format!("The request failed before a response ({cause}).")],
            )
        })?;
        let text = String::from_utf8_lossy(&response.body).into_owned();
        let pause = if response.status == 429 {
            Some(pause_from_headers(
                header(&response, "retry-after"),
                header(&response, "x-ratelimit-reset"),
                ctx.d.host.now(),
            ))
        } else {
            None
        };
        let line = UsageLine {
            ts: String::new(),
            env: ctx.env.clone(),
            surface: ctx.surface.clone(),
            key: hash.clone(),
            method: req.method.clone(),
            path: req.path_template.clone(),
            status: response.status,
            rate_limited: pause.is_some(),
            request_id: id,
        };
        let _ = usage::log_usage(ctx.d, line);
        if req.refuse_redirect && (300..400).contains(&response.status) {
            let mut e = CliError::new(
                EXIT::REFUSED,
                "REFUSED",
                vec![format!(
                    "The server answered with a redirect (HTTP {}). The CLI does not follow it.",
                    response.status
                )],
            );
            e.status = Some(response.status);
            return Err(e);
        }
        if let Some(ms) = pause {
            breaker.record(ms)?;
            if attempt == 0 {
                continue;
            }
        }
        return Ok(HttpResponse {
            status: response.status,
            headers: response.headers,
            text,
        });
    }
    unreachable!()
}
pub fn runner(d: &Deps) -> String {
    for (v, n) in [
        ("CLAUDECODE", "claude-code"),
        ("CODEX_THREAD_ID", "codex"),
        ("GEMINI_CLI", "gemini-cli"),
        ("CURSOR_AGENT", "cursor"),
    ] {
        if d.env.get(v).is_some_and(|value| !value.is_empty()) {
            return format!("agent:{n}");
        }
    }
    if d.env.get("CI").is_some_and(|value| !value.is_empty()) {
        "ci".into()
    } else if d.is_tty {
        "tty".into()
    } else {
        "script".into()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::deps::{ExecError, ExecOpts, ExecResult, Host, Io, Keychain, NetError};
    use std::{
        collections::{BTreeMap, VecDeque},
        path::PathBuf,
        sync::{
            Arc, Mutex,
            atomic::{AtomicI64, Ordering},
        },
    };
    struct H {
        responses: Mutex<VecDeque<Response>>,
        requests: Mutex<Vec<Request>>,
        now: AtomicI64,
    }
    impl Host for H {
        fn fetch(&self, r: Request) -> Result<Response, NetError> {
            self.requests.lock().unwrap().push(r);
            Ok(self
                .responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_default())
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
            self.now.load(Ordering::SeqCst)
        }
        fn wall_now(&self) -> i64 {
            self.now()
        }
        fn pid_alive(&self, _: u32) -> bool {
            false
        }
        fn sleep(&self, n: i64) {
            self.now.fetch_add(n, Ordering::SeqCst);
        }
        fn stdout(&self, _: &str) {}
        fn stderr(&self, _: &str) {}
        fn keychain(&self) -> Option<&dyn Keychain> {
            None
        }
        fn io(&self) -> Option<&dyn Io> {
            None
        }
    }
    fn ctx(statuses: &[u16]) -> (Ctx<'static>, Arc<H>) {
        let id = uuid();
        let h = Arc::new(H {
            responses: Mutex::new(
                statuses
                    .iter()
                    .map(|status| Response {
                        status: *status,
                        headers: vec![],
                        body: vec![],
                    })
                    .collect(),
            ),
            requests: Mutex::new(vec![]),
            now: AtomicI64::new(0),
        });
        let d = Box::leak(Box::new(Deps {
            host: Box::new(Proxy(h.clone())),
            env: BTreeMap::new(),
            is_tty: false,
            config_dir: std::env::temp_dir().join(format!("http-{id}")),
            state_dir: std::env::temp_dir().join(format!("http-state-{id}")),
            cache_dir: PathBuf::new(),
            version: "0.7.2".into(),
            pid: std::process::id(),
            name: "purchasely".into(),
            host_url: String::new(),
            ext: None,
            self_exe: PathBuf::new(),
            platform: "linux",
            stdout_tty: false,
            columns: None,
        }));
        (
            Ctx {
                d,
                env: "prod".into(),
                surface: "management".into(),
                key: Some("test-key".into()),
                identity: None,
                reason: Some("test".into()),
                command: Some("apps list".into()),
                rate: 1.0,
                headers: vec![],
                read_only: false,
            },
            h,
        )
    }
    struct Proxy(Arc<H>);
    impl Host for Proxy {
        fn fetch(&self, r: Request) -> Result<Response, NetError> {
            self.0.fetch(r)
        }
        fn exec(&self, a: &str, b: &[String], c: ExecOpts) -> Result<ExecResult, ExecError> {
            self.0.exec(a, b, c)
        }
        fn prompt(&self, a: &str, b: bool) -> String {
            self.0.prompt(a, b)
        }
        fn read_stdin(&self) -> String {
            self.0.read_stdin()
        }
        fn now(&self) -> i64 {
            self.0.now()
        }
        fn wall_now(&self) -> i64 {
            self.0.wall_now()
        }
        fn pid_alive(&self, p: u32) -> bool {
            self.0.pid_alive(p)
        }
        fn sleep(&self, n: i64) {
            self.0.sleep(n)
        }
        fn stdout(&self, s: &str) {
            self.0.stdout(s)
        }
        fn stderr(&self, s: &str) {
            self.0.stderr(s)
        }
        fn keychain(&self) -> Option<&dyn Keychain> {
            None
        }
        fn io(&self) -> Option<&dyn Io> {
            None
        }
    }
    fn req(refuse: bool) -> HttpRequest {
        HttpRequest {
            method: "GET".into(),
            url: "https://example.invalid".into(),
            path_template: "/v1".into(),
            body: None,
            write: Some(false),
            refuse_redirect: refuse,
        }
    }
    #[test]
    fn user_agent() {
        assert_eq!(
            super::user_agent("1", Some("person"), "hello\nworld", "apps list", "ci"),
            "purchasely-cli/1 (person; hello world; apps list; ci)"
        )
    }
    #[test]
    fn refused_redirect_exit_10() {
        let (c, _) = ctx(&[302]);
        let e = send(&c, &req(true)).unwrap_err();
        assert_eq!(e.exit, 10);
        assert_eq!(e.status, Some(302))
    }
    #[test]
    fn timeout_is_120000() {
        let (c, h) = ctx(&[200]);
        let _ = send(&c, &req(false)).unwrap();
        let requests = h.requests.lock().unwrap();
        assert_eq!(requests[0].timeout_ms, 120000);
        assert!(
            requests[0]
                .headers
                .iter()
                .any(|(k, v)| k == "Authorization" && v == "Bearer test-key")
        );
        assert!(
            requests[0]
                .headers
                .iter()
                .any(|(k, v)| k == "X-Request-Id" && v.len() == 36)
        );
    }
    #[test]
    fn retries_once_after_429_pause() {
        let (c, h) = ctx(&[429, 200]);
        let r = send(&c, &req(false)).unwrap();
        assert_eq!(r.status, 200);
        assert_eq!(h.requests.lock().unwrap().len(), 2);
        assert!(h.now() > 0)
    }
}
