//! Owner: P1b
use purchasely::{
    Deps,
    core::deps::{
        ExecError, ExecOpts, ExecResult, Host, Io, Keychain, KeychainError, NetError, Request,
        Response,
    },
};
use serde_json::{Map, Value, json};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, VecDeque},
    path::{Path, PathBuf},
    rc::Rc,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayError(pub String);

#[derive(Clone)]
pub struct FakeHost {
    inner: Rc<Inner>,
}

struct Inner {
    step: Value,
    calls: RefCell<VecDeque<Value>>,
    execs: RefCell<VecDeque<Value>>,
    answers: RefCell<VecDeque<Value>>,
    probes: RefCell<VecDeque<Value>>,
    /// The keys of a recorded terminal (`io.keys`), in order, across every menu of the step.
    keys: RefCell<VecDeque<Value>>,
    now: Cell<i64>,
    wall: Cell<i64>,
    stdout: RefCell<String>,
    stderr: RefCell<String>,
    prompts: RefCell<Vec<Value>>,
    keychain: Option<FakeKeychain>,
    error: RefCell<Option<ReplayError>>,
    child_runs: Cell<usize>,
    root: PathBuf,
}

#[derive(Clone)]
struct FakeKeychain {
    expected: Rc<RefCell<VecDeque<Value>>>,
    store: Rc<RefCell<BTreeMap<String, String>>>,
    observed: Rc<RefCell<Vec<Value>>>,
    error: Rc<RefCell<Option<ReplayError>>>,
}

impl FakeHost {
    pub fn from_step(step: &Value) -> Self {
        Self::with_root(
            step,
            std::env::temp_dir().join(format!("contract-root-{}", std::process::id())),
        )
    }
    pub fn with_root(step: &Value, root: PathBuf) -> Self {
        let keychain = if step.get("keychain").is_none() || step["keychain"].is_null() {
            None
        } else {
            let initial = step["keychain"]["before"]
                .as_object()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|(k, v)| v.as_str().map(|v| (k, v.to_owned())))
                .collect();
            Some(FakeKeychain {
                expected: Rc::new(RefCell::new(
                    step["keychain"]["ops"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default()
                        .into(),
                )),
                store: Rc::new(RefCell::new(initial)),
                observed: Rc::new(RefCell::new(Vec::new())),
                error: Rc::new(RefCell::new(None)),
            })
        };
        Self {
            inner: Rc::new(Inner {
                step: step.clone(),
                calls: RefCell::new(array(step, "calls").into()),
                execs: RefCell::new(array(step, "execs").into()),
                answers: RefCell::new(array(step, "prompts").into()),
                probes: RefCell::new(array(step, "pid_probes").into()),
                keys: RefCell::new(
                    step["io"]["keys"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default()
                        .into(),
                ),
                now: Cell::new(step["t_before"].as_i64().unwrap_or(0)),
                wall: Cell::new(parse_wall(step["wall"].as_str().unwrap_or_default()).unwrap_or(0)),
                stdout: RefCell::new(String::new()),
                stderr: RefCell::new(String::new()),
                prompts: RefCell::new(Vec::new()),
                keychain,
                error: RefCell::new(None),
                child_runs: Cell::new(0),
                root,
            }),
        }
    }
    fn fail(&self, msg: impl Into<String>) {
        *self.inner.error.borrow_mut() = Some(ReplayError(msg.into()));
    }
    pub fn child_runs(&self) -> usize {
        self.inner.child_runs.get()
    }
    pub fn stdout_text(&self) -> String {
        self.inner.stdout.borrow().clone()
    }
    pub fn stderr_text(&self) -> String {
        self.inner.stderr.borrow().clone()
    }
    pub fn keychain_state(&self) -> Value {
        self.inner.keychain.as_ref().map(|k|json!({"before":self.inner.step["keychain"]["before"],"after":&*k.store.borrow(),"ops":&*k.observed.borrow()})).unwrap_or(Value::Null)
    }
    pub fn finish(&self) -> Result<(), ReplayError> {
        if let Some(error) = self.inner.error.borrow().clone() {
            return Err(error);
        }
        if let Some(next) = self.inner.calls.borrow().front() {
            return Err(ReplayError(format!(
                "{} fetch call(s) were not consumed, the next is {} {}",
                self.inner.calls.borrow().len(),
                next["method"].as_str().unwrap_or("?"),
                next["url"].as_str().unwrap_or("?")
            )));
        }
        for (name, remaining) in [
            ("fetch", self.inner.calls.borrow().len()),
            ("exec", self.inner.execs.borrow().len()),
            ("pid probe", self.inner.probes.borrow().len()),
            ("terminal key", self.inner.keys.borrow().len()),
        ] {
            if remaining != 0 {
                return Err(ReplayError(format!(
                    "{remaining} {name} call(s) were not consumed"
                )));
            }
        }
        if !self.inner.answers.borrow().is_empty() {
            return Err(ReplayError(format!(
                "{} prompt(s) were not consumed",
                self.inner.answers.borrow().len()
            )));
        }
        if let Some(keychain) = &self.inner.keychain {
            if let Some(error) = keychain.error.borrow().clone() {
                return Err(error);
            }
            if !keychain.expected.borrow().is_empty() {
                return Err(ReplayError(format!(
                    "{} keychain operation(s) were not consumed",
                    keychain.expected.borrow().len()
                )));
            }
            if *keychain.observed.borrow()
                != self.inner.step["keychain"]["ops"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
            {
                return Err(ReplayError("keychain operations differ".into()));
            }
            let actual = serde_json::to_value(&*keychain.store.borrow()).unwrap_or(Value::Null);
            if actual != self.inner.step["keychain"]["after"] {
                return Err(ReplayError(format!(
                    "keychain final state differs: {actual}"
                )));
            }
        }
        let prompts = self.inner.prompts.borrow().clone();
        if !self.inner.step["prompts"].is_null() && json!(prompts) != self.inner.step["prompts"] {
            return Err(ReplayError("prompt transcript differs".into()));
        }
        Ok(())
    }
    pub fn deps(&self, _argv: Vec<String>) -> Deps {
        let step = &self.inner.step;
        let roots = crate::replay::execution_roots(step, &self.inner.root).unwrap_or_default();
        let env = step["env"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(k, v)| {
                v.as_str()
                    .map(|v| (k.clone(), crate::replay::expand_tokens(v, &roots)))
            })
            .collect();
        Deps {
            host: Box::new(self.clone()),
            env,
            is_tty: step["is_tty"].as_bool().unwrap_or(false),
            config_dir: self.inner.root.join("config"),
            state_dir: self.inner.root.join("state"),
            cache_dir: self.inner.root.join("cache"),
            version: step["version"].as_str().unwrap_or("9.9.9").into(),
            pid: step["pid"].as_u64().unwrap_or(1) as u32,
            name: step["name"].as_str().unwrap_or("purchasely").into(),
            host_url: step["host"].as_str().unwrap_or("https://cli.test").into(),
            ext: (!step["ext"].is_null()).then(|| crate::replay::recorded_extension(&step["ext"])),
            self_exe: PathBuf::from("<SELF>"),
            platform: match step["platform"].as_str().unwrap_or("linux") {
                "darwin" => "darwin",
                _ => "linux",
            },
            stdout_tty: step["stdout_tty"].as_bool().unwrap_or(false),
            columns: step["columns"].as_u64().map(|n| n as u16),
        }
    }
    /// The files that the recorded command wrote in the step roots (a screenshot), with their mode.
    fn apply_writes(&self, writes: &Value) -> Result<(), ExecError> {
        let roots = crate::replay::execution_roots(&self.inner.step, &self.inner.root)
            .map_err(|e| ExecError::Other(e.0))?;
        for write in writes.as_array().into_iter().flatten() {
            let token = write["root"].as_str().unwrap_or("<ROOT>");
            let Some((_, root)) = roots.iter().find(|(t, _)| t == token) else {
                self.fail(format!("exec write to an unknown root {token}"));
                return Err(ExecError::Other("unknown write root".into()));
            };
            let path = root.join(write["path"].as_str().unwrap_or_default());
            let bytes = write["hex"]
                .as_str()
                .and_then(decode_hex)
                .unwrap_or_default();
            let io = |e: std::io::Error| ExecError::Other(e.to_string());
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(io)?;
            }
            std::fs::write(&path, bytes).map_err(io)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = write["mode"].as_u64().unwrap_or(0o644) as u32;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode))
                    .map_err(io)?;
            }
        }
        Ok(())
    }
    fn exec_once(
        &self,
        file: &str,
        args: &[String],
        opts: ExecOpts,
    ) -> Result<ExecResult, ExecError> {
        let Some(expected) = self.inner.execs.borrow_mut().pop_front() else {
            self.fail(format!("unexpected exec call {file} {args:?}"));
            return Err(ExecError::Other("unexpected exec call".into()));
        };
        let expected_file = expected["file"].as_str().unwrap_or_default();
        // TS runs `node <script> args` and records args without the script. Rust runs its own binary (the
        // `self_exe` of the fake deps, `<SELF>`) with the args alone.
        let actual_args = if expected_file == "<SELF>" && file != "<SELF>" {
            args.get(1..).unwrap_or_default()
        } else {
            args
        };
        let expected_args: Vec<String> =
            serde_json::from_value(expected["args"].clone()).unwrap_or_default();
        let recorded_opts = &expected["opts"];
        let cwd = opts.cwd.as_ref().map(|p| p.to_string_lossy().into_owned());
        let expected_cwd =
            expected_opts_string(recorded_opts, "cwd", &self.inner.step, &self.inner.root);
        let cwd_match = expected_cwd.as_ref().is_none_or(|v| {
            normalize_text(v, &self.inner.step, &self.inner.root)
                == cwd
                    .as_deref()
                    .map(|s| normalize_text(s, &self.inner.step, &self.inner.root))
                    .unwrap_or_default()
        });
        let input_match = recorded_opts["input"].as_str().is_none_or(|v| {
            normalize_text(v, &self.inner.step, &self.inner.root)
                == opts
                    .input
                    .as_deref()
                    .map(|s| normalize_text(s, &self.inner.step, &self.inner.root))
                    .unwrap_or_default()
        });
        let opts_match = recorded_opts.is_null()
            || (recorded_opts["timeoutMs"]
                .as_u64()
                .is_none_or(|v| v == opts.timeout_ms)
                && recorded_opts["detach"]
                    .as_bool()
                    .is_none_or(|v| v == opts.detach)
                && input_match
                && cwd_match);
        let expected_args_value =
            normalize_value(&json!(expected_args), &self.inner.step, &self.inner.root);
        let actual_args_value =
            normalize_value(&json!(actual_args), &self.inner.step, &self.inner.root);
        let expected_file = normalize_text(expected_file, &self.inner.step, &self.inner.root);
        let actual_file = normalize_text(file, &self.inner.step, &self.inner.root);
        if (expected_file != "<SELF>" && expected_file != actual_file)
            || expected_args_value != actual_args_value
            || !opts_match
        {
            self.fail(format!(
                "exec differs: {file} {args:?} opts={opts:?}; expected {expected_file} {expected_args_value} opts={recorded_opts}"
            ));
            return Err(ExecError::Other("exec transcript mismatch".into()));
        }
        if expected["run_child"] == true {
            self.inner.child_runs.set(self.inner.child_runs.get() + 1);
            let child_args = expected_args;
            let result = purchasely::run::run(child_args.clone(), self.deps(child_args));
            if result != expected["result"]["code"].as_i64().unwrap_or(0) as i32 {
                self.fail("child exit code differs");
            }
        }
        self.apply_writes(&expected["writes"])?;
        if let Some(error) = expected["error"].as_str() {
            return Err(if error == "ENOENT" {
                ExecError::NotFound
            } else {
                ExecError::Other(error.into())
            });
        }
        let result = &expected["result"];
        Ok(ExecResult {
            code: result["code"].as_i64().unwrap_or(0) as i32,
            stdout: result["stdout"].as_str().unwrap_or_default().into(),
            stderr: result["stderr"].as_str().unwrap_or_default().into(),
            timed_out: result["timed_out"].as_bool().unwrap_or(false),
        })
    }
}

impl Host for FakeHost {
    fn fetch(&self, req: Request) -> Result<Response, NetError> {
        let Some(expected) = self.inner.calls.borrow_mut().pop_front() else {
            self.fail(format!("unexpected fetch call {} {}", req.method, req.url));
            return Err(NetError {
                message: "unexpected fetch call".into(),
                ..NetError::default()
            });
        };
        let expected_call = json!({"method":expected["method"],"url":expected["url"],"headers":lower_headers(&expected["headers"]),"body":expected["body"],"refuse_redirect":expected["refuse_redirect"],"at":expected["at"]});
        let actual_headers: Map<String, Value> = req
            .headers
            .iter()
            .map(|(k, v)| (k.to_ascii_lowercase(), json!(v)))
            .collect();
        let actual_call = json!({"method":req.method,"url":req.url,"headers":actual_headers,"body":req.body.as_ref().map(|b|String::from_utf8_lossy(b).into_owned()),"refuse_redirect":req.refuse_redirect,"at":self.now()});
        let expected_call = normalize_value(&expected_call, &self.inner.step, &self.inner.root);
        let actual_call = normalize_value(&actual_call, &self.inner.step, &self.inner.root);
        if expected_call != actual_call {
            // Name each field that differs, with both values.
            let fields = ["method", "url", "headers", "body", "refuse_redirect", "at"]
                .iter()
                .filter(|k| expected_call[**k] != actual_call[**k])
                .map(|k| {
                    format!(
                        "{k}: expected {}, got {}",
                        expected_call[*k], actual_call[*k]
                    )
                })
                .collect::<Vec<_>>();
            self.fail(format!(
                "fetch transcript mismatch for {} {}: {}",
                req.method,
                req.url,
                fields.join("; ")
            ));
            return Err(NetError {
                message: "fetch transcript mismatch".into(),
                ..NetError::default()
            });
        }
        let reply = &expected["reply"];
        if let Some(message) = reply["error"].as_str() {
            // The recorder keeps the error name when there is no cause code: TS prints `cause.code ?? name`.
            let name = reply["name"].as_str();
            return Err(NetError {
                message: message.into(),
                timed_out: message.to_lowercase().contains("timed out")
                    || name == Some("TimeoutError"),
                code: reply["code"].as_str().or(name).map(str::to_owned),
            });
        }
        let body = if let Some(hex) = reply["hex"].as_str() {
            decode_hex(hex).unwrap_or_default()
        } else {
            reply["text"]
                .as_str()
                .map(str::as_bytes)
                .map(<[u8]>::to_vec)
                .or_else(|| {
                    reply["bytes"].as_array().map(|a| {
                        a.iter()
                            .filter_map(Value::as_u64)
                            .map(|n| n as u8)
                            .collect()
                    })
                })
                .unwrap_or_default()
        };
        Ok(Response {
            status: reply["status"].as_u64().unwrap_or(200) as u16,
            headers: headers_vec(&reply["headers"]),
            body,
        })
    }
    fn exec(&self, file: &str, args: &[String], opts: ExecOpts) -> Result<ExecResult, ExecError> {
        self.exec_once(file, args, opts)
    }
    fn prompt(&self, question: &str, hidden: bool) -> String {
        let Some(expected) = self.inner.answers.borrow_mut().pop_front() else {
            self.fail("unexpected prompt");
            return String::new();
        };
        if expected["question"].as_str().is_some_and(|v| v != question)
            || expected["hidden"].as_bool().is_some_and(|v| v != hidden)
        {
            self.fail("prompt differs");
        }
        self.inner
            .prompts
            .borrow_mut()
            .push(json!({"question":question,"hidden":hidden,"answer":expected["answer"]}));
        expected["answer"].as_str().unwrap_or_default().into()
    }
    fn read_stdin(&self) -> String {
        self.inner.step["stdin"].as_str().unwrap_or_default().into()
    }
    fn now(&self) -> i64 {
        self.inner.now.get()
    }
    fn wall_now(&self) -> i64 {
        self.inner.wall.get()
    }
    fn pid_alive(&self, pid: u32) -> bool {
        let Some(expected) = self.inner.probes.borrow_mut().pop_front() else {
            self.fail("unexpected pid_alive probe");
            return false;
        };
        let expected_pid = &expected["pid"];
        if expected_pid.as_str().is_some_and(|v| v != "<PID>")
            || expected_pid.as_u64().is_some_and(|v| v != pid as u64)
        {
            self.fail("pid_alive pid differs");
        }
        expected["alive"].as_bool().unwrap_or(false)
    }
    fn sleep(&self, ms: i64) {
        self.inner.now.set(self.inner.now.get() + ms);
        self.inner.wall.set(self.inner.wall.get() + ms);
    }
    fn stdout(&self, s: &str) {
        self.inner.stdout.borrow_mut().push_str(s);
    }
    fn stderr(&self, s: &str) {
        self.inner.stderr.borrow_mut().push_str(s);
    }
    fn keychain(&self) -> Option<&dyn Keychain> {
        self.inner.keychain.as_ref().map(|k| k as &dyn Keychain)
    }
    fn io(&self) -> Option<&dyn Io> {
        self.inner.step["io"].is_object().then_some(self as &dyn Io)
    }
}

/// The recorded terminal: its keys go to the game until it is over; no key left quits.
impl Io for FakeHost {
    fn play(&self, game: &mut dyn purchasely::games::term::Game) -> Option<Result<String, String>> {
        while game.over().is_none() {
            let Some(key) = self.inner.keys.borrow_mut().pop_front() else {
                return Some(Ok("Bye.".into()));
            };
            game.key(key.as_str().unwrap_or_default());
        }
        Some(Ok(game.over().unwrap_or_default().to_owned()))
    }
    fn dimensions(&self) -> (usize, usize) {
        let io = &self.inner.step["io"];
        let size = |key: &str, default: u64| io[key].as_u64().unwrap_or(default) as usize;
        (size("columns", 80), size("rows", 24))
    }
    fn is_tty(&self) -> bool {
        self.inner.step["io"]["tty"].as_bool().unwrap_or(false)
    }
}

impl Keychain for FakeKeychain {
    fn get(&self, service: &str, account: &str) -> Result<Option<String>, KeychainError> {
        let op = self.next("get", service, account, None)?;
        if let Some(err) = op["error"].as_str() {
            self.observed
                .borrow_mut()
                .push(json!({"op":"get","args":[service,account],"error":err}));
            return Err(KeychainError(err.into()));
        }
        let result = if op.get("result").is_some() {
            op["result"].as_str().map(str::to_owned)
        } else {
            self.store
                .borrow()
                .get(&format!("{service}/{account}"))
                .cloned()
        };
        self.observed
            .borrow_mut()
            .push(json!({"op":"get","args":[service,account],"result":result}));
        Ok(result)
    }
    fn set(&self, service: &str, account: &str, secret: &str) -> Result<(), KeychainError> {
        let op = self.next("set", service, account, Some(secret))?;
        if let Some(err) = op["error"].as_str() {
            self.observed
                .borrow_mut()
                .push(json!({"op":"set","args":[service,account,secret],"error":err}));
            return Err(KeychainError(err.into()));
        }
        self.store
            .borrow_mut()
            .insert(format!("{service}/{account}"), secret.into());
        self.observed
            .borrow_mut()
            .push(json!({"op":"set","args":[service,account,secret],"result":null}));
        Ok(())
    }
    fn delete(&self, service: &str, account: &str) -> Result<(), KeychainError> {
        let op = self.next("delete", service, account, None)?;
        if let Some(err) = op["error"].as_str() {
            self.observed
                .borrow_mut()
                .push(json!({"op":"delete","args":[service,account],"error":err}));
            return Err(KeychainError(err.into()));
        }
        self.store
            .borrow_mut()
            .remove(&format!("{service}/{account}"));
        self.observed
            .borrow_mut()
            .push(json!({"op":"delete","args":[service,account],"result":null}));
        Ok(())
    }
}
impl FakeKeychain {
    fn next(
        &self,
        kind: &str,
        service: &str,
        account: &str,
        secret: Option<&str>,
    ) -> Result<Value, KeychainError> {
        let Some(op) = self.expected.borrow_mut().pop_front() else {
            *self.error.borrow_mut() = Some(ReplayError("unexpected keychain operation".into()));
            return Err(KeychainError("unexpected keychain operation".into()));
        };
        let mut args = vec![json!(service), json!(account)];
        if let Some(secret) = secret {
            args.push(json!(secret));
        }
        if op["op"].as_str() != Some(kind) || op["args"] != json!(args) {
            *self.error.borrow_mut() =
                Some(ReplayError(format!("keychain {kind} arguments differ")));
            return Err(KeychainError("keychain transcript mismatch".into()));
        }
        Ok(op)
    }
}

fn array(v: &Value, key: &str) -> Vec<Value> {
    v[key].as_array().cloned().unwrap_or_default()
}
fn normalize_text(text: &str, step: &Value, root: &Path) -> String {
    let mut roots = crate::replay::execution_roots(step, root).unwrap_or_default();
    // The longest path first: the root path is a prefix of the `<TMPn>` paths.
    roots.sort_by_key(|(_, path)| std::cmp::Reverse(path.as_os_str().len()));
    let mut mapped = text.to_owned();
    for (token, path) in roots {
        mapped = mapped.replace(&path.to_string_lossy().to_string(), &token);
    }
    for (path, token) in step
        .as_object()
        .map(crate::replay::ts_tmp_roots)
        .unwrap_or_default()
    {
        mapped = mapped.replace(&path, &token);
    }
    crate::normalize::normalize(
        &crate::replay::collapse_repo(&mapped),
        &root.to_string_lossy(),
        step["pid"].as_u64().unwrap_or(u32::MAX as u64) as u32,
        step["version"]
            .as_str()
            .filter(|v| !v.is_empty())
            .unwrap_or("__NO_VERSION_MATCH__"),
    )
    .replace("purchasely-cli-<MKDTEMP>", "<ROOT>")
}
fn normalize_value(value: &Value, step: &Value, root: &Path) -> Value {
    let text = value.to_string();
    let text = normalize_text(&text, step, root);
    serde_json::from_str(&text).unwrap_or_else(|_| value.clone())
}
fn expected_opts_string(opts: &Value, key: &str, step: &Value, root: &Path) -> Option<String> {
    let value = opts.get(key)?.as_str()?;
    let roots = crate::replay::execution_roots(step, root).ok()?;
    Some(crate::replay::expand_tokens(value, &roots))
}
fn headers_vec(v: &Value) -> Vec<(String, String)> {
    if let Some(m) = v.as_object() {
        m.iter()
            .filter_map(|(k, v)| v.as_str().map(|v| (k.clone(), v.into())))
            .collect()
    } else {
        vec![]
    }
}
fn lower_headers(value: &Value) -> Value {
    Value::Object(
        value
            .as_object()
            .into_iter()
            .flatten()
            .map(|(k, v)| (k.to_ascii_lowercase(), v.clone()))
            .collect(),
    )
}
fn decode_hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).ok()?, 16).ok())
        .collect()
}
fn parse_wall(s: &str) -> Option<i64> {
    let (date, time) = s.split_once('T')?;
    let mut d = date.split('-').map(|v| v.parse::<i64>().ok());
    let (y, m, day) = (d.next()??, d.next()??, d.next()??);
    let t = time.trim_end_matches('Z');
    let mut t = t.split(':');
    let h = t.next()?.parse::<i64>().ok()?;
    let min = t.next()?.parse::<i64>().ok()?;
    let seconds = t.next()?;
    let (sec, millis) = if let Some((whole, fraction)) = seconds.split_once('.') {
        let digits = format!("{fraction:0<3}");
        (whole.parse::<i64>().ok()?, digits[..3].parse::<i64>().ok()?)
    } else {
        (seconds.parse::<i64>().ok()?, 0)
    };
    let y = y - i64::from(m <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(((era * 146097 + doe - 719468) * 86400 + h * 3600 + min * 60 + sec) * 1000 + millis)
}

#[cfg(test)]
mod tests {
    use super::*;
    use purchasely::core::deps::KeychainError;
    #[test]
    fn recorded_terminal_keys_drive_the_menus() {
        let host = FakeHost::from_step(
            &json!({"io":{"tty":true,"columns":100,"rows":24,"keys":["down","return","return","down"]}}),
        );
        let io = host.io().unwrap();
        assert!(io.is_tty());
        assert_eq!(io.dimensions(), (100, 24));
        let lines = ["a".to_owned(), "b".to_owned()];
        let pick = || purchasely::games::term::choose(&lines, "T", "run", host.io(), &mut |_| {});
        assert_eq!(pick(), Some(1));
        assert_eq!(pick(), Some(0));
        assert!(host.finish().unwrap_err().0.contains("1 terminal key"));
        assert!(FakeHost::from_step(&json!({})).io().is_none());
    }
    #[test]
    fn extra_call_fails() {
        let host = FakeHost::from_step(&json!({"calls":[]}));
        let e = host.fetch(Request::default()).unwrap_err();
        assert_eq!(e.message, "unexpected fetch call");
        assert!(host.finish().unwrap_err().0.contains("unexpected fetch"));
    }
    #[test]
    fn missing_call_fails() {
        let host = FakeHost::from_step(&json!({"calls":[{"method":"GET","url":"/expected"}]}));
        assert!(host.finish().unwrap_err().0.contains("1 fetch call"));
    }
    #[test]
    fn keychain_failure_replays() {
        let host = FakeHost::from_step(
            &json!({"keychain":{"before":{},"after":{},"ops":[{"op":"get","args":["svc","acct"],"error":"no keychain"}]}}),
        );
        assert_eq!(
            host.keychain().unwrap().get("svc", "acct").unwrap_err(),
            KeychainError("no keychain".into())
        );
        host.finish().unwrap();
        assert!(
            FakeHost::from_step(&json!({"keychain":null}))
                .keychain()
                .is_none()
        );
    }
    #[test]
    fn keychain_operations_replay_in_order() {
        let host = FakeHost::from_step(
            &json!({"keychain":{"before":{},"after":{},"ops":[{"op":"set","args":["svc","acct","secret"],"result":null},{"op":"get","args":["svc","acct"],"result":"secret"},{"op":"delete","args":["svc","acct"],"result":null}]}}),
        );
        let keychain = host.keychain().unwrap();
        keychain.set("svc", "acct", "secret").unwrap();
        assert_eq!(
            keychain.get("svc", "acct").unwrap().as_deref(),
            Some("secret")
        );
        keychain.delete("svc", "acct").unwrap();
        host.finish().unwrap();
    }
    #[test]
    fn prompt_replays_hidden_answer() {
        let host = FakeHost::from_step(
            &json!({"prompts":[{"question":"Password","hidden":true,"answer":"secret"}]}),
        );
        assert_eq!(host.prompt("Password", true), "secret");
        host.finish().unwrap();
    }
    #[test]
    fn run_child_reruns() {
        let host = FakeHost::from_step(
            &json!({"name":"purchasely","version":"9.9.9","host":"https://cli.test","execs":[{"file":"<SELF>","args":["--version"],"opts":{"detach":true},"run_child":true,"result":{"code":0}}]}),
        );
        let result = host
            .exec(
                "/node",
                &["runner.js".into(), "--version".into()],
                ExecOpts {
                    detach: true,
                    ..ExecOpts::default()
                },
            )
            .unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(host.child_runs(), 1);
        host.finish().unwrap();
    }
    #[test]
    fn pid_alive_replays() {
        let host = FakeHost::from_step(
            &json!({"pid_probes":[{"pid":"<PID>","alive":true},{"pid":7,"alive":false}]}),
        );
        assert!(host.pid_alive(42));
        assert!(!host.pid_alive(7));
        assert!(host.finish().is_ok());
    }
    #[test]
    fn sleep_advances_wall() {
        let host = FakeHost::from_step(&json!({"wall":"2026-01-15T12:00:00Z","t_before":10}));
        host.sleep(25);
        assert_eq!(host.wall_now(), 1_768_478_400_025);
        assert_eq!(host.now(), 35);
    }
    #[test]
    fn fetch_checks_ordered_request_and_replays_reply() {
        let host = FakeHost::from_step(
            &json!({"calls":[{"at":0,"method":"GET","url":"/x","headers":{"x-id":"1"},"body":null,"refuse_redirect":false,"reply":{"status":201,"text":"ok"}}]}),
        );
        let r = Request {
            method: "GET".into(),
            url: "/x".into(),
            headers: vec![("X-ID".into(), "1".into())],
            ..Request::default()
        };
        let response = host.fetch(r).unwrap();
        assert_eq!(response.status, 201);
        assert_eq!(response.body, b"ok");
        host.finish().unwrap();
    }
    #[test]
    fn exec_replays_options_and_result() {
        let host = FakeHost::from_step(
            &json!({"execs":[{"file":"cmd","args":["a"],"opts":{"timeoutMs":50,"detach":false},"result":{"code":3,"stdout":"out","stderr":"err","timed_out":true}}]}),
        );
        let result = host
            .exec(
                "cmd",
                &["a".into()],
                ExecOpts {
                    timeout_ms: 50,
                    ..ExecOpts::default()
                },
            )
            .unwrap();
        assert_eq!(
            result,
            ExecResult {
                code: 3,
                stdout: "out".into(),
                stderr: "err".into(),
                timed_out: true
            }
        );
        host.finish().unwrap();
    }
    #[test]
    fn exec_writes_its_recorded_files() {
        let root = std::env::temp_dir().join(format!("contract-writes-{}", std::process::id()));
        let host = FakeHost::with_root(
            &json!({"execs":[{"file":"cmd","args":[],"writes":[{"root":"<ROOT>","path":"out/shot.png","mode":420,"hex":"6f6b"}],"result":{"code":0}}]}),
            root.clone(),
        );
        host.exec("cmd", &[], ExecOpts::default()).unwrap();
        assert_eq!(std::fs::read(root.join("out/shot.png")).unwrap(), b"ok");
        let _ = std::fs::remove_dir_all(root);
        host.finish().unwrap();
    }
    #[test]
    fn fetch_error_name_is_the_cause() {
        let host = FakeHost::from_step(
            &json!({"calls":[{"method":"GET","url":"u","headers":{},"body":null,"refuse_redirect":false,"at":0,"reply":{"error":"no answer","name":"TimeoutError"}}]}),
        );
        let request = Request {
            method: "GET".into(),
            url: "u".into(),
            ..Request::default()
        };
        let error = host.fetch(request).unwrap_err();
        assert_eq!(error.code.as_deref(), Some("TimeoutError"));
        assert!(error.timed_out);
    }
}
