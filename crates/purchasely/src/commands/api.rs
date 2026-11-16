//! Owner: P6b

use crate::{
    cli::{Argument, Command, OptionDef, Parsed},
    core::{
        errors::{CliError, usage},
        runtime::Runtime,
    },
    management::{
        exec,
        request::{self, Built},
        spec,
    },
};

const METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE"];
pub fn register(program: &mut Command, rt: &mut Runtime) {
    let mut c = Command::new("api")
        .description("Send a raw Management API request")
        .argument(Argument::required(
            "<method>",
            "GET, POST, PUT, PATCH, DELETE",
        ))
        .argument(Argument::required(
            "<path>",
            "path under /management/v1, such as /applications/app_xxx/placements",
        ));
    c.options.extend(globals());
    c.options.push(
        OptionDef::value("--query <k=v>", "query parameter (repeatable)")
            .repeat()
            .default("[]"),
    );
    c.options.push(OptionDef::value(
        "--data <json>",
        "request body: JSON, @file.json or - for stdin",
    ));
    c.options.push(OptionDef::value(
        "--confirm-token <token>",
        "token from a CONFIRMATION_REQUIRED response",
    ));
    c.options.push(OptionDef::value(
        "--plan <file>",
        "record the write in this plan file, send nothing. A person reads it, then runs `apply <file>`",
    ));
    c = c.after_help(format!("\nThe path is relative to /management/v1, or starts with it. The CLI sends it only to the Management API of --env.\nA write (POST, PUT, PATCH, DELETE) needs --env prod or --env staging, also with --dry-run.\nA risky change returns CONFIRMATION_REQUIRED: add --yes, answer the prompt, or resend with --confirm-token <token>.\n\nExamples:\n  {} api GET /applications/app_xxx/placements --query limit=10\n  {} api PATCH /applications/app_xxx/placements/plc_xxx --data @placement.json --env prod --dry-run", rt.d.name, rt.d.name));
    c = c.action(run);
    program.commands.push(c);
}
pub fn globals() -> Vec<OptionDef> {
    vec![
        OptionDef::value("--output <format>", "json (default) or table"),
        OptionDef::value("--env <env>", "prod or staging. Mandatory for writes"),
        OptionDef::boolean("--dry-run", "print the request without sending it"),
        OptionDef::boolean("--yes", "accept the confirmation prompt of a risky change"),
        OptionDef::value(
            "--reason <text>",
            "why you run this command (sent in the User-Agent)",
        ),
        OptionDef::value("--rate <n>", "requests per second, lower than the default"),
        OptionDef::boolean("--i-asked-backend", "allow a rate above the hard cap"),
        OptionDef::value(
            "--out <file>",
            "write the JSON output to a file, or - for stdout",
        ),
    ]
}
fn run(p: &Parsed, rt: &mut Runtime) -> Result<(), CliError> {
    let method = p
        .arguments
        .first()
        .map(|s| s.to_ascii_uppercase())
        .unwrap_or_default();
    if !METHODS.contains(&method.as_str()) {
        return Err(usage(
            "The method must be one of GET, POST, PUT, PATCH, DELETE.",
        ));
    }
    let raw = p.arguments.get(1).ok_or_else(|| usage("missing path"))?;
    let full = exec::validate_api_path(raw)?;
    let path = &full["/management/v1".len()..];
    let mut query = Vec::new();
    for item in p.options.get("query").into_iter().flatten() {
        let Some((k, v)) = item.split_once('=') else {
            return Err(usage(format!("--query takes k=v, got \"{item}\".")));
        };
        if k.is_empty() {
            return Err(usage(format!("--query takes k=v, got \"{item}\".")));
        }
        query.push(format!("{}={}", encode(k), encode(v)));
    }
    if method == "GET" && p.opt("data").is_some() {
        return Err(usage("A GET takes no body. Do not pass --data."));
    }
    let body = if let Some(d) = p.opt("data") {
        Some(request::json_body(&request::read_data(
            d,
            &rt.d.host.read_stdin(),
        )?)?)
    } else {
        None
    };
    // The spec operation gives the confirm placement and the logged path template. No match: no confirm retry,
    // and the log keeps a segment only when it is a literal segment of the spec.
    let matched = find_op(&method, &full);
    if p.opt("confirm-token").is_some() && matched.is_none() {
        return Err(usage(
            "No operation of the API spec matches this method and path, so the token has no place. Send it with --query confirm_token=<token>, or in the --data body.",
        ));
    }
    let op = matched.unwrap_or_else(|| crate::management::spec::Op {
        method: method.clone(),
        path: template(path),
        tag: String::new(),
        summary: String::new(),
        description: String::new(),
        scope: String::new(),
        params: vec![],
        body: None,
        confirm: None,
        app: false,
    });
    let full = full.clone();
    let body = body.clone();
    let query = query.clone();
    let build = move |env: &str, t: Option<&str>| {
        let base = request::base_url(env);
        let mut u = format!(
            "{base}{full}{}",
            if query.is_empty() {
                String::new()
            } else {
                format!("?{}", query.join("&"))
            }
        );
        let mut b = body.clone();
        if let Some(tok) = t {
            let (nu, nb) = request::with_token(op.confirm.as_deref(), &u, b.as_deref(), Some(tok))?;
            u = nu;
            b = nb;
        }
        Ok(Built {
            op: op.clone(),
            url: u,
            body: b,
        })
    };
    exec::execute(rt, p, build, "api", true, exec::Meta::default())
}
fn find_op(method: &str, path: &str) -> Option<crate::management::spec::Op> {
    let (_, ops) = spec::embedded_spec();
    ops.into_iter()
        .filter(|o| o.method == method && matches(o.path.as_str(), path))
        .min_by_key(|o| o.path.matches('{').count())
}
fn matches(pattern: &str, path: &str) -> bool {
    let a: Vec<_> = pattern.split('/').collect();
    let b: Vec<_> = path.split('/').collect();
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| x.starts_with('{') && x.ends_with('}') || *x == y)
}
fn template(path: &str) -> String {
    let (_, ops) = spec::embedded_spec();
    let literals: std::collections::HashSet<String> = ops
        .iter()
        .flat_map(|o| {
            o.path
                .split('/')
                .filter(|s| !s.is_empty() && !s.starts_with('{'))
                .map(str::to_owned)
        })
        .collect();
    format!(
        "/management/v1{}",
        path.split('/')
            .map(|s| if s.is_empty() || literals.contains(s) {
                s.to_owned()
            } else {
                "{id}".into()
            })
            .collect::<Vec<_>>()
            .join("/")
    )
}
fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(b as char)
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::deps::*;
    use std::{cell::RefCell, collections::BTreeMap, path::PathBuf, rc::Rc};
    struct Capture(Rc<RefCell<Vec<Request>>>);
    impl Host for Capture {
        fn fetch(&self, r: Request) -> Result<Response, NetError> {
            self.0.borrow_mut().push(r);
            Ok(Response {
                status: 200,
                body: b"{\"placements\":[]}".to_vec(),
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
        fn stdout(&self, _: &str) {}
        fn stderr(&self, _: &str) {}
        fn keychain(&self) -> Option<&dyn Keychain> {
            None
        }
        fn io(&self) -> Option<&dyn Io> {
            None
        }
    }
    fn runtime(capture: Rc<RefCell<Vec<Request>>>) -> Runtime {
        let root = std::env::temp_dir().join(format!("p6b-api-{}", std::process::id()));
        let mut env = BTreeMap::new();
        env.insert("PURCHASELY_API_KEY".into(), "pk_mgmt_live_SECRET123".into());
        Runtime::new(Deps {
            host: Box::new(Capture(capture)),
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
    #[test]
    fn api_path_validation_blocks_encoded_traversal() {
        assert_eq!(
            exec::validate_api_path("/applications/%2e%2e/admin")
                .unwrap_err()
                .code,
            "USAGE"
        );
    }
    #[test]
    fn api_sends_checked_path_and_refuses_redirects() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let mut rt = runtime(calls.clone());
        let mut root = Command::new("purchasely");
        register(&mut root, &mut rt);
        let argv = [
            "api",
            "get",
            "/applications/app_1/placements",
            "--query",
            "fields=id,name",
        ]
        .map(str::to_owned);
        let p = crate::cli::parse(&root, &argv).unwrap();
        let leaf = root.find(&p.command).unwrap();
        (leaf.action.as_ref().unwrap().0)(&p, &mut rt).unwrap();
        let req = calls.borrow();
        assert_eq!(
            req[0].url,
            "https://api.purchasely.io/management/v1/applications/app_1/placements?fields=id%2Cname"
        );
        assert!(req[0].refuse_redirect);
        assert_eq!(req[0].method, "GET");
    }
}
