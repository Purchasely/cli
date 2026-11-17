//! Update channel selection, daily state and installer invocation.
use super::{
    deps::{Deps, ExecOpts, Request},
    errors::{CliError, EXIT},
    json::{self, Value},
    runtime::Runtime,
    state::StateStore,
};
use std::path::{Path, PathBuf};

pub const DAY_MS: i64 = 86_400_000;
pub const LATEST_TIMEOUT_MS: u64 = 15_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallKind {
    Managed,
    Brew,
    Npm,
}

pub fn install_kind(executable: &Path, managed_root: Option<&str>) -> InstallKind {
    let canonical = executable
        .canonicalize()
        .unwrap_or_else(|_| executable.to_path_buf());
    if canonical.to_string_lossy().contains("/Cellar/") {
        InstallKind::Brew
    } else if managed_root.is_some_and(|root| !root.is_empty()) {
        InstallKind::Managed
    } else {
        InstallKind::Npm
    }
}

pub fn install_kind_for(d: &Deps) -> InstallKind {
    install_kind(
        &d.self_exe,
        d.env
            .get("PURCHASELY_MANAGED_INSTALL_ROOT")
            .map(String::as_str),
    )
}

pub fn newer(a: &str, b: &str) -> bool {
    let parse = |s: &str| {
        s.strip_prefix('v')
            .unwrap_or(s)
            .split('.')
            .take(3)
            .map(|part| part.parse::<u64>().unwrap_or(0))
            .collect::<Vec<_>>()
    };
    let (a, b) = (parse(a), parse(b));
    (0..3).any(|i| {
        a.get(i).copied().unwrap_or(0) > b.get(i).copied().unwrap_or(0)
            && (0..i).all(|j| a.get(j) == b.get(j))
    })
}

pub fn stable_version(value: &str) -> Option<String> {
    let value = value.strip_prefix('v').unwrap_or(value);
    let parts: Vec<_> = value.split('.').collect();
    (parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit())))
    .then(|| value.to_owned())
}

pub fn latest_from_json(body: &str) -> Option<String> {
    let value = json::parse(body).ok()?;
    let field = |key: &str| match &value {
        Value::Object(values) => values.iter().find(|(name, _)| name == key)?.1.as_str(),
        _ => None,
    };
    stable_version(&field("native").or_else(|| field("version"))?)
}

pub fn latest_request() -> Request {
    Request {
        method: "GET".into(),
        url: String::new(),
        timeout_ms: LATEST_TIMEOUT_MS,
        ..Request::default()
    }
}

pub fn latest_version(d: &Deps) -> Result<Option<String>, CliError> {
    if let Some(latest) = d.ext.as_ref().and_then(|extension| extension.latest) {
        return latest(d);
    }
    let fail = || {
        CliError::new(
            EXIT::GENERIC,
            "UPDATE_CHECK_FAILED",
            vec!["could not check the latest version. Check your network, then try again.".into()],
        )
    };
    let mut request = latest_request();
    request.url = format!("{}/latest.json", d.host_url.trim_end_matches('/'));
    let response = d.host.fetch(request).map_err(|_| fail())?;
    if response.status != 200 {
        return Err(fail());
    }
    let body = String::from_utf8(response.body).map_err(|_| fail())?;
    let value = json::parse(&body).map_err(|_| fail())?;
    let field = |key: &str| match &value {
        Value::Object(values) => values.iter().find(|(name, _)| name == key)?.1.as_str(),
        _ => None,
    };
    let latest = field("native")
        .or_else(|| field("version"))
        .ok_or_else(fail)?;
    Ok(stable_version(&latest))
}

pub fn update_store(d: &Deps) -> StateStore<'_> {
    StateStore::new(d, format!("update.{}", d.name))
}

pub fn read_record(store: &StateStore<'_>) -> Value {
    let record = store.read("check", Value::Object(vec![]));
    if matches!(record, Value::Object(_)) {
        record
    } else {
        Value::Object(vec![])
    }
}

pub fn merge_record(store: &StateStore<'_>, patch: Vec<(String, Value)>) -> Result<(), String> {
    let mut record = match read_record(store) {
        Value::Object(record) => record,
        _ => vec![],
    };
    for (key, value) in patch {
        if let Some(existing) = record.iter_mut().find(|(name, _)| *name == key) {
            existing.1 = value;
        } else {
            record.push((key, value));
        }
    }
    store
        .write("check", &Value::Object(record))
        .map_err(|error| error.to_string())
}

pub fn should_run_installer(kind: InstallKind, latest: &str, current: &str) -> bool {
    kind == InstallKind::Managed && newer(latest, current)
}

pub fn notice_line(d: &Deps, kind: InstallKind, latest: &str) -> String {
    match kind {
        InstallKind::Brew => format!("{} {latest} is available: brew upgrade {}", d.name, d.name),
        InstallKind::Npm => format!(
            "{} {latest} is available: {}",
            d.name,
            d.ext
                .as_ref()
                .and_then(|ext| ext.npm_update)
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    format!(
                        "curl -fsSL {}/install.sh | sh",
                        d.host_url.trim_end_matches('/')
                    )
                })
        ),
        InstallKind::Managed => String::new(),
    }
}

pub fn run_installer(
    d: &Deps,
    root: &str,
    version: &str,
    log: Option<&Path>,
) -> Result<super::deps::ExecResult, String> {
    let script = PathBuf::from(root)
        .join("releases")
        .join(&d.version)
        .join("install.sh");
    let mut args = vec![
        script.to_string_lossy().into_owned(),
        "--version".into(),
        version.into(),
        "--keep".into(),
        d.version.clone(),
        "--root".into(),
        root.into(),
        "--upgrade-only".into(),
    ];
    if let Some(log) = log {
        args.extend(["--log".into(), log.to_string_lossy().into_owned()]);
    }
    d.host
        .exec(
            "sh",
            &args,
            ExecOpts {
                timeout_ms: 600_000,
                ..ExecOpts::default()
            },
        )
        .map_err(|error| match error {
            super::deps::ExecError::NotFound => "sh was not found".to_owned(),
            super::deps::ExecError::Other(message) => message,
        })
}

/// Best-effort daily update check run by the parent action hook. P5a owns the hook invocation.
pub fn auto_update(rt: &mut Runtime, top_level: &str) {
    let d = &rt.d;
    if d.env.contains_key("CI")
        || d.env
            .get("PURCHASELY_DISABLE_AUTOUPDATE")
            .is_some_and(|value| value == "1")
        || d.env
            .get("PURCHASELY_OFFLINE")
            .is_some_and(|value| value == "1")
        || d.env
            .get("PURCHASELY_DEMO")
            .is_some_and(|value| value == "1")
        || rt.cfg.auto_update == Some(false)
        || top_level == "update"
        || top_level == "ai"
    {
        return;
    }
    let store = update_store(d);
    let now = d.host.now();
    let record = read_record(&store);
    let checked = get_number(&record, "checked_at").unwrap_or(0);
    let kind = install_kind_for(d);
    if kind != InstallKind::Managed
        && let Some(latest) = get_string(&record, "latest")
        && newer(&latest, &d.version)
        && now - get_number(&record, "notified_at").unwrap_or(0) >= DAY_MS
        && store
            .with_lock_timeout(-1, || {
                let _ = merge_record(
                    &store,
                    vec![("notified_at".into(), Value::Number(now as f64))],
                );
            })
            .is_ok()
    {
        d.host
            .stderr(&format!("{}\n", notice_line(d, kind, &latest)));
    }
    if now - checked >= DAY_MS || plugin_check_due(d, now) {
        let self_exe = d.self_exe.to_string_lossy().into_owned();
        let _ = d.host.exec(
            &self_exe,
            &["update".into(), "--background".into()],
            ExecOpts {
                timeout_ms: 0,
                detach: true,
                ..ExecOpts::default()
            },
        );
    }
}

pub fn run_update(rt: &mut Runtime, background: bool) -> Result<(), CliError> {
    let kind = install_kind_for(&rt.d);
    let store = update_store(&rt.d);
    let now = rt.d.host.now();
    if background {
        let due = store
            .with_lock_timeout(5000, || {
                let record = read_record(&store);
                if now - get_number(&record, "checked_at").unwrap_or(0) < DAY_MS {
                    false
                } else {
                    let _ = merge_record(
                        &store,
                        vec![("checked_at".into(), Value::Number(now as f64))],
                    );
                    true
                }
            })
            .unwrap_or(false);
        if !due {
            return Ok(());
        }
    }
    let latest = match latest_version(&rt.d) {
        Ok(latest) => latest,
        Err(_error) if background => return Ok(()),
        Err(error) => return Err(error),
    };
    if let Some(version) = latest.as_ref() {
        let _ = merge_record(
            &store,
            vec![
                ("latest".into(), Value::from(version.clone())),
                ("checked_at".into(), Value::Number(now as f64)),
            ],
        );
    } else if !background {
        rt.d.host
            .stderr("No stable release found, nothing to compare with.\n");
    }
    let mut action = "up_to_date";
    if let Some(version) = latest
        .as_deref()
        .filter(|version| newer(version, &rt.d.version))
    {
        if should_run_installer(kind, version, &rt.d.version) {
            let root =
                rt.d.env
                    .get("PURCHASELY_MANAGED_INSTALL_ROOT")
                    .map(String::as_str)
                    .unwrap_or("");
            let log = background.then(|| rt.d.state_dir.join(format!("update.{}.log", rt.d.name)));
            let result = run_installer(&rt.d, root, version, log.as_deref())
                .map_err(|message| CliError::new(EXIT::GENERIC, "UPDATE_FAILED", vec![message]))?;
            if !background {
                rt.d.host.stderr(&result.stderr);
            }
            if result.code != 0 {
                if background {
                    return Ok(());
                }
                let messages = result
                    .stderr
                    .lines()
                    .rev()
                    .take(5)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .map(str::to_owned)
                    .collect();
                return Err(CliError::new(EXIT::GENERIC, "UPDATE_FAILED", messages));
            }
            action = "updated";
        } else {
            if !background {
                rt.d.host
                    .stderr(&format!("{}\n", notice_line(&rt.d, kind, version)));
            }
            action = "notice";
        }
    }
    if !background {
        let value = Value::Object(vec![
            (
                "kind".into(),
                Value::from(
                    match kind {
                        InstallKind::Managed => "managed",
                        InstallKind::Brew => "brew",
                        InstallKind::Npm => "npm",
                    }
                    .to_owned(),
                ),
            ),
            ("current".into(), Value::from(rt.d.version.clone())),
            (
                "latest".into(),
                latest.map(Value::from).unwrap_or(Value::Null),
            ),
            ("action".into(), Value::from(action.to_owned())),
        ]);
        super::output::emit(
            &rt.d,
            &value,
            &super::output::EmitOptions {
                env: "prod".into(),
                name: "update".into(),
                ..Default::default()
            },
        );
    }
    Ok(())
}

fn get_number(value: &Value, key: &str) -> Option<i64> {
    if let Value::Object(entries) = value {
        entries
            .iter()
            .find(|(name, _)| name == key)
            .and_then(|(_, v)| match v {
                Value::Number(n) => Some(*n as i64),
                _ => None,
            })
    } else {
        None
    }
}
fn get_string(value: &Value, key: &str) -> Option<String> {
    if let Value::Object(entries) = value {
        entries
            .iter()
            .find(|(name, _)| name == key)
            .and_then(|(_, v)| v.as_str())
    } else {
        None
    }
}

fn plugin_check_due(d: &Deps, now: i64) -> bool {
    let plugin = StateStore::new(d, "ai").read("plugin", Value::Null);
    let Some(agents) = (if let Value::Object(fields) = &plugin {
        fields
            .iter()
            .find(|(key, _)| key == "agents")
            .map(|(_, value)| value)
    } else {
        None
    }) else {
        return false;
    };
    matches!(agents, Value::Object(entries) if !entries.is_empty())
        && now - get_number(&plugin, "checked_at").unwrap_or(0) >= DAY_MS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::deps::{ExecError, ExecResult, Host, Io, Keychain, NetError, Response};
    use std::{
        collections::BTreeMap,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    struct UpdateHost(Arc<AtomicUsize>);
    impl Host for UpdateHost {
        fn fetch(&self, _: Request) -> Result<Response, NetError> {
            Ok(Response {
                status: 200,
                body: br#"{"version":"0.9.0","native":"1.0.0"}"#.to_vec(),
                ..Response::default()
            })
        }
        fn exec(&self, _: &str, _: &[String], _: ExecOpts) -> Result<ExecResult, ExecError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(ExecResult::default())
        }
        fn prompt(&self, _: &str, _: bool) -> String {
            String::new()
        }
        fn read_stdin(&self) -> String {
            String::new()
        }
        fn now(&self) -> i64 {
            DAY_MS * 3
        }
        fn wall_now(&self) -> i64 {
            DAY_MS * 3
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

    fn brew_runtime(dir: &Path, execs: Arc<AtomicUsize>) -> Runtime {
        let mut env = BTreeMap::new();
        env.insert(
            "PURCHASELY_MANAGED_INSTALL_ROOT".into(),
            "/managed/root".into(),
        );
        Runtime::new(Deps {
            host: Box::new(UpdateHost(execs)),
            env,
            is_tty: false,
            config_dir: dir.join("config"),
            state_dir: dir.join("state"),
            cache_dir: dir.join("cache"),
            version: "0.7.2".into(),
            pid: std::process::id(),
            name: "purchasely".into(),
            host_url: "https://cli.purchasely.io".into(),
            ext: None,
            self_exe: PathBuf::from("/opt/homebrew/Cellar/purchasely/0.7.2/bin/purchasely"),
            platform: "darwin",
            stdout_tty: false,
            columns: None,
        })
    }

    #[test]
    fn native_field_first() {
        assert_eq!(
            latest_from_json(r#"{"version":"0.9.0","native":"1.0.1"}"#),
            Some("1.0.1".into())
        );
        assert_eq!(
            latest_from_json(r#"{"version":"0.9.0"}"#),
            Some("0.9.0".into())
        );
    }

    /// The detached calls of `auto_update`: the host of `brew_runtime` that keeps the arguments.
    struct Spawns(Arc<std::sync::Mutex<Vec<Vec<String>>>>);
    impl Host for Spawns {
        fn fetch(&self, r: Request) -> Result<Response, NetError> {
            UpdateHost(Arc::default()).fetch(r)
        }
        fn exec(&self, _: &str, args: &[String], o: ExecOpts) -> Result<ExecResult, ExecError> {
            if o.detach {
                self.0.lock().unwrap().push(args.to_vec());
            }
            Ok(ExecResult::default())
        }
        fn prompt(&self, _: &str, _: bool) -> String {
            String::new()
        }
        fn read_stdin(&self) -> String {
            String::new()
        }
        fn now(&self) -> i64 {
            DAY_MS * 3
        }
        fn wall_now(&self) -> i64 {
            DAY_MS * 3
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
    fn spawn_runtime(
        tag: &str,
        env: &[(&str, &str)],
    ) -> (Runtime, Arc<std::sync::Mutex<Vec<Vec<String>>>>) {
        let dir =
            std::env::temp_dir().join(format!("purchasely-spawn-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut rt = brew_runtime(&dir, Arc::default());
        let spawned = Arc::<std::sync::Mutex<Vec<Vec<String>>>>::default();
        rt.d.host = Box::new(Spawns(spawned.clone()));
        rt.d.env = env
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect();
        (rt, spawned)
    }
    fn number(n: i64) -> Value {
        Value::Number(n as f64)
    }

    /// ai.test.ts "the parent spawns when only the plugin is due".
    #[test]
    fn the_parent_spawns_when_only_the_plugin_is_due() {
        let (mut rt, spawned) = spawn_runtime("plugin", &[]);
        let now = rt.d.host.now();
        merge_record(
            &update_store(&rt.d),
            vec![("checked_at".into(), number(now))],
        )
        .unwrap();
        let register = |rt: &Runtime, agents: bool, checked_at: i64| {
            let claude = Value::Object(vec![("method".into(), Value::from("native".to_owned()))]);
            let agents = Value::Object(if agents {
                vec![("claude".into(), claude)]
            } else {
                vec![]
            });
            StateStore::new(&rt.d, "ai")
                .write(
                    "plugin",
                    &Value::Object(vec![
                        ("latest".into(), Value::from("2.1.1".to_owned())),
                        ("agents".into(), agents),
                        ("checked_at".into(), number(checked_at)),
                    ]),
                )
                .unwrap();
        };
        let mut go = |rt: &mut Runtime, top: &str| {
            spawned.lock().unwrap().clear();
            auto_update(rt, top);
            spawned.lock().unwrap().clone()
        };
        register(&rt, true, now - 2 * DAY_MS);
        let calls = go(&mut rt, "usage");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0][calls[0].len() - 2..], ["update", "--background"]);
        // No spawn for the ai group itself: it does the work in the foreground.
        assert!(go(&mut rt, "ai").is_empty());
        register(&rt, true, now);
        assert!(go(&mut rt, "usage").is_empty());
        register(&rt, false, now - 2 * DAY_MS);
        assert!(go(&mut rt, "usage").is_empty());
        let _ = std::fs::remove_dir_all(&rt.d.state_dir);
    }

    /// demo.test.ts "demo mode > starts no update check": a check that is due does not start in demo mode.
    #[test]
    fn demo_mode_starts_no_update_check() {
        let (mut rt, spawned) = spawn_runtime("demo", &[("PURCHASELY_DEMO", "1")]);
        auto_update(&mut rt, "usage");
        assert!(spawned.lock().unwrap().is_empty());
        // The same runtime out of demo mode does start it: the test bites.
        rt.d.env.clear();
        auto_update(&mut rt, "usage");
        assert_eq!(spawned.lock().unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(&rt.d.state_dir);
    }

    /// update.test.ts "install kind > tells managed, brew and npm".
    #[test]
    fn tells_managed_brew_and_npm() {
        assert_eq!(
            install_kind(Path::new("/x/dist/core/update.js"), Some("/r")),
            InstallKind::Managed
        );
        assert_eq!(
            install_kind(
                Path::new("/opt/homebrew/Cellar/purchasely/0.2.0/libexec/dist/core/update.js"),
                None
            ),
            InstallKind::Brew
        );
        assert_eq!(
            install_kind(Path::new("/Users/k/cli/dist/core/update.js"), None),
            InstallKind::Npm
        );
    }

    /// update.test.ts "newer > compares x.y.z numerically".
    #[test]
    fn newer_compares_x_y_z_numerically() {
        assert!(newer("v0.2.0", "0.1.9"));
        assert!(!newer("0.1.0", "0.1.0"));
        assert!(!newer("0.1.0", "0.2.0"));
        assert!(newer("0.10.0", "0.9.0"));
    }

    /// update.test.ts "notice once a day > notice matrix per program and install kind".
    #[test]
    fn notice_matrix_per_program_and_install_kind() {
        let acme = || crate::core::ext::Extension {
            modules: vec![],
            surface: "extra",
            surfaces: vec![("extra", "extra")],
            surface_defs: vec![],
            identity: true,
            section: ("More commands", "Only in `acme`."),
            latest: None,
            npm_update: Some("npm install -g acme"),
        };
        let cases = [
            (
                "purchasely",
                InstallKind::Brew,
                "purchasely 10.0.0 is available: brew upgrade purchasely",
            ),
            (
                "acme",
                InstallKind::Brew,
                "acme 10.0.0 is available: brew upgrade acme",
            ),
            (
                "purchasely",
                InstallKind::Npm,
                "purchasely 10.0.0 is available: curl -fsSL https://cli.example.test/install.sh | sh",
            ),
            (
                "acme",
                InstallKind::Npm,
                "acme 10.0.0 is available: npm install -g acme",
            ),
        ];
        for (name, kind, line) in cases {
            let mut d = crate::commands::ai::tests::test_deps();
            d.name = name.into();
            d.host_url = "https://cli.example.test".into();
            d.ext = (name == "acme").then(acme);
            assert_eq!(notice_line(&d, kind, "10.0.0"), line);
        }
    }

    #[test]
    fn latest_timeout_is_15000() {
        assert_eq!(latest_request().timeout_ms, 15_000);
    }

    #[test]
    fn brew_install_kind_ignores_managed_root() {
        let exe = Path::new("/opt/homebrew/Cellar/purchasely/0.7.2/bin/purchasely");
        assert_eq!(install_kind(exe, Some("/managed/root")), InstallKind::Brew);
    }

    #[test]
    fn brew_updates_never_run_an_installer() {
        assert!(!should_run_installer(InstallKind::Brew, "1.0.0", "0.7.2"));
    }

    #[test]
    fn brew_foreground_and_background_updates_skip_installer_with_managed_root() {
        let dir =
            std::env::temp_dir().join(format!("purchasely-brew-update-{}", std::process::id()));
        let execs = Arc::new(AtomicUsize::new(0));
        let mut rt = brew_runtime(&dir, execs.clone());
        run_update(&mut rt, false).unwrap();
        let mut rt = brew_runtime(&dir.join("background"), execs.clone());
        run_update(&mut rt, true).unwrap();
        assert_eq!(execs.load(Ordering::SeqCst), 0);
        let _ = std::fs::remove_dir_all(dir);
    }
}
