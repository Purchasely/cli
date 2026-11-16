//! Owner: P1b
use crate::{
    case::Case,
    fake::{FakeHost, ReplayError},
};
use purchasely::{Extension, Host, cli::Command, core::runtime::Runtime};
use serde_json::{Map, Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReplayReport {
    pub run: usize,
    pub pending: usize,
    pub filtered: usize,
}

pub fn pending_count(cases: &[Value]) -> usize {
    cases.iter().filter(|c| c.get("pending").is_some()).count()
}
pub fn normalize_case(case: &Value) -> Value {
    let expected = if case.get("intended_diff").is_some() {
        case.get("expected_ts").unwrap_or(case)
    } else {
        case
    };
    let mut normalized = expected.clone();
    if let Some(obj) = normalized.as_object_mut() {
        obj.remove("source");
        obj.remove("pending");
        obj.remove("unit");
        obj.remove("intended_diff");
        obj.remove("expected_ts");
    }
    if let Some(steps) = normalized.get_mut("steps").and_then(Value::as_array_mut) {
        for step in steps {
            let pid = step["pid"].as_u64().unwrap_or(u32::MAX as u64) as u32;
            let version = step["version"]
                .as_str()
                .filter(|v| !v.is_empty())
                .unwrap_or("__NO_VERSION_MATCH__");
            let version = version.to_owned();
            // The pid field goes first: "pid":<PID> is not valid JSON.
            if let Some(obj) = step.as_object_mut() {
                obj.remove("pid");
            }
            let text = step.to_string();
            let text = crate::normalize::normalize(&text, "__NO_ROOT_MATCH__", pid, &version)
                .replace("purchasely-cli-<MKDTEMP>", "<ROOT>");
            *step = serde_json::from_str::<Value>(&text).expect("normalized step is JSON");
        }
    }
    normalized
}
pub fn compare_drift_case(committed: &Value, fresh: &Value) -> Result<(), String> {
    let expected = normalize_case(committed);
    let actual = normalize_case(fresh);
    if expected == actual {
        Ok(())
    } else {
        Err(format!("contract drift: expected {expected}, got {actual}"))
    }
}
pub fn compare_dirs(committed: &Path, fresh: &Path) -> Result<(usize, usize, usize), String> {
    let (mut checked, mut skipped, mut new_files) = (0, 0, 0);
    if !fresh.exists() {
        return Err(format!(
            "fresh recordings directory is missing: {}",
            fresh.display()
        ));
    }
    if committed.exists() {
        for entry in fs::read_dir(committed).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let fresh_path = fresh.join(path.file_name().unwrap_or_default());
            let old: Value = serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            if !fresh_path.exists() {
                let all_unit = old["cases"]
                    .as_array()
                    .is_some_and(|a| !a.is_empty() && a.iter().all(|c| c.get("unit").is_some()));
                if all_unit {
                    skipped += 1;
                    continue;
                }
                return Err(format!("missing fresh recording file {}", path.display()));
            }
            let new: Value =
                serde_json::from_slice(&fs::read(&fresh_path).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            let old_cases = old["cases"].as_array().cloned().unwrap_or_default();
            let new_cases = new["cases"].as_array().cloned().unwrap_or_default();
            for expected in old_cases {
                let name = expected["test"].as_str().unwrap_or_default();
                let Some(actual) = new_cases.iter().find(|c| c["test"].as_str() == Some(name))
                else {
                    if expected.get("unit").is_some() {
                        skipped += 1;
                        continue;
                    }
                    return Err(format!("missing fresh case {name}"));
                };
                compare_drift_case(&expected, actual)?;
                checked += 1;
            }
        }
    }
    for entry in fs::read_dir(fresh).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json")
            && !committed
                .join(path.file_name().unwrap_or_default())
                .exists()
        {
            new_files += 1;
        }
    }
    Ok((checked, skipped, new_files))
}
pub fn selected<'a>(cases: &'a [Value], filter: Option<&str>) -> Result<Vec<&'a Value>, String> {
    let Some(filter) = filter.filter(|s| !s.is_empty()) else {
        return Ok(cases.iter().collect());
    };
    let prefixes: Vec<_> = filter.split(',').collect();
    let selected: Vec<_> = cases
        .iter()
        .filter(|c| {
            prefixes
                .iter()
                .any(|p| c["test"].as_str().unwrap_or_default().starts_with(p))
        })
        .collect();
    for prefix in prefixes {
        if !selected
            .iter()
            .any(|c| c["test"].as_str().unwrap_or_default().starts_with(prefix))
        {
            return Err(format!("CONTRACT_FILTER prefix matched no case: {prefix}"));
        }
    }
    Ok(selected)
}
pub fn run_cases(cases: &[Case], filter: Option<&str>) -> Result<ReplayReport, String> {
    let values: Vec<Value> = cases
        .iter()
        .map(|c| serde_json::to_value(c).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    let chosen = selected(&values, filter)?;
    let mut report = ReplayReport {
        filtered: values.len() - chosen.len(),
        ..ReplayReport::default()
    };
    for value in chosen {
        let case: Case = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
        if case.pending.is_some() {
            report.pending += 1;
            continue;
        }
        for (index, step) in case.steps.iter().enumerate() {
            replay_step(&step.data).map_err(|e| format!("{} step {index}: {}", case.test, e.0))?;
            report.run += 1;
        }
    }
    eprintln!(
        "run {}, pending {}, filtered {}",
        report.run, report.pending, report.filtered
    );
    Ok(report)
}
pub fn run_case_directory(
    directory: &Path,
    filter: Option<&str>,
    bless: bool,
) -> Result<ReplayReport, String> {
    let mut files = Vec::<(PathBuf, Vec<Value>)>::new();
    if directory.exists() {
        for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let file: Value = serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            files.push((path, file["cases"].as_array().cloned().unwrap_or_default()));
        }
    }
    let flat = files
        .iter()
        .flat_map(|(_, cases)| cases.iter().cloned())
        .collect::<Vec<_>>();
    let selected = selected(&flat, filter)?;
    let mut report = ReplayReport {
        filtered: flat.len() - selected.len(),
        ..ReplayReport::default()
    };
    let mut updated = flat.clone();
    for old in selected {
        let index = flat.iter().position(|c| std::ptr::eq(c, old)).unwrap();
        if old.get("pending").is_some() {
            report.pending += 1;
            continue;
        }
        let mut case = old.clone();
        let Some(steps) = case["steps"].as_array_mut() else {
            continue;
        };
        for step in steps {
            let Some(data) = step.as_object() else {
                continue;
            };
            if bless {
                let actual = replay_step(data).map_err(|e| format!("{}: {}", old["test"], e.0))?;
                for key in ["code", "out", "err", "t_after", "files_after"] {
                    if let Some(value) = actual.get(key) {
                        step[key] = value.clone();
                    }
                }
                if !step["keychain"].is_null() {
                    step["keychain"] = actual["keychain"].clone();
                }
            } else {
                replay_step(data).map_err(|e| format!("{}: {}", old["test"], e.0))?;
            }
            report.run += 1;
        }
        if bless {
            case["source"] = json!("bless");
            updated[index] = case;
        }
    }
    if bless {
        let mut offset = 0;
        for (path, cases) in &mut files {
            for case in cases.iter_mut() {
                *case = updated[offset].clone();
                offset += 1;
            }
            let file = json!({"cases":cases});
            fs::write(
                path,
                serde_json::to_vec_pretty(&file).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        }
    }
    eprintln!(
        "run {}, pending {}, filtered {}",
        report.run, report.pending, report.filtered
    );
    Ok(report)
}

pub fn replay_step(step: &Map<String, Value>) -> Result<Value, ReplayError> {
    let root = std::env::temp_dir().join(format!(
        "purchasely-cli-{}-{}",
        std::process::id(),
        NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        fs::create_dir_all(&root).map_err(io_error)?;
        let roots = make_roots(step, &root)?;
        apply_snapshots(step.get("files_before").unwrap_or(&Value::Null), &roots)?;
        let mut argv: Vec<String> =
            serde_json::from_value(step.get("argv").cloned().unwrap_or(json!([])))
                .unwrap_or_default();
        for arg in &mut argv {
            *arg = expand_tokens(arg, &roots);
        }
        let mut replay_step = step.clone();
        if let Some(env) = replay_step.get_mut("env").and_then(Value::as_object_mut) {
            for value in env.values_mut() {
                if let Some(s) = value.as_str() {
                    *value = json!(expand_tokens(s, &roots));
                }
            }
        }
        let host = FakeHost::with_root(&Value::Object(replay_step), root.clone());
        let deps = host.deps(argv.clone());
        let code = purchasely::run::run(argv, deps);
        host.finish()?;
        let files_after = snapshot_roots(&roots)?;
        let actual = json!({"code":code,"out":host.stdout_text(),"err":host.stderr_text(),"t_after":host.now(),"files_after":files_after,"keychain":keychain_after(step,&host)});
        for key in ["code", "out", "err"] {
            if let Some(expected) = step.get(key)
                && normalize_transcript(&actual[key], &roots, step)
                    != normalize_transcript(expected, &[], step)
            {
                return Err(ReplayError(format!(
                    "{key} differs: expected {expected}, got {}",
                    actual[key]
                )));
            }
        }
        if let Some(expected) = step.get("t_after").and_then(Value::as_i64)
            && host.now() != expected
        {
            return Err(ReplayError(format!(
                "t_after differs: expected {expected}, got {}",
                host.now()
            )));
        }
        if let Some(expected) = step.get("files_after")
            && normalize_transcript(&actual["files_after"], &roots, step)
                != normalize_transcript(expected, &[], step)
        {
            return Err(ReplayError("files_after differs".into()));
        }
        if step.get("keychain").is_some_and(|v| !v.is_null())
            && normalize_transcript(&actual["keychain"], &roots, step)
                != normalize_transcript(&step["keychain"], &[], step)
        {
            return Err(ReplayError("keychain state differs".into()));
        }
        Ok(normalize_transcript(&actual, &roots, step))
    })();
    let _ = fs::remove_dir_all(&root);
    for (_, path) in make_roots(step, &root)
        .unwrap_or_default()
        .into_iter()
        .skip(1)
    {
        let _ = fs::remove_dir_all(path);
    }
    result
}

pub(crate) fn execution_roots(
    step: &Value,
    root: &Path,
) -> Result<Vec<(String, PathBuf)>, ReplayError> {
    let map = step.as_object().cloned().unwrap_or_default();
    make_roots(&map, root)
}
fn make_roots(
    step: &Map<String, Value>,
    root: &Path,
) -> Result<Vec<(String, PathBuf)>, ReplayError> {
    let mut roots = vec![("<ROOT>".to_owned(), root.to_path_buf())];
    let files_before = step.get("files_before").unwrap_or(&Value::Null);
    let mut tokens = files_before
        .as_object()
        .into_iter()
        .flatten()
        .map(|(k, _)| k.clone())
        .filter(|k| k.starts_with("<TMP") && k.ends_with('>'))
        .collect::<Vec<_>>();
    tokens.sort_by_key(|s| s[4..s.len() - 1].parse::<usize>().unwrap_or(usize::MAX));
    for token in tokens {
        let base = root.file_name().unwrap_or_default().to_string_lossy();
        let p = root
            .parent()
            .unwrap_or_else(|| Path::new("/tmp"))
            .join(format!("{base}-{}", token.trim_matches(['<', '>'])));
        fs::create_dir_all(&p).map_err(io_error)?;
        roots.push((token, p));
    }
    Ok(roots)
}
fn apply_snapshots(snapshots: &Value, roots: &[(String, PathBuf)]) -> Result<(), ReplayError> {
    let Some(map) = snapshots.as_object() else {
        return Ok(());
    };
    for (token, entries) in map {
        let root = roots
            .iter()
            .find(|(t, _)| t == token)
            .map(|(_, p)| p)
            .ok_or_else(|| ReplayError(format!("unknown snapshot root {token}")))?;
        let Some(entries) = entries.as_array() else {
            continue;
        };
        for entry in entries {
            let path = root.join(entry["path"].as_str().unwrap_or_default());
            if entry["directory"].as_bool().unwrap_or(false) {
                fs::create_dir_all(&path).map_err(io_error)?;
            } else {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).map_err(io_error)?;
                }
                let bytes = entry["hex"]
                    .as_str()
                    .and_then(decode_hex)
                    .or_else(|| entry["text"].as_str().map(|s| s.as_bytes().to_vec()))
                    .unwrap_or_default();
                fs::write(&path, bytes).map_err(io_error)?;
            }
            set_mode(&path, entry["mode"].as_u64().unwrap_or(0o700) as u32)?;
        }
    }
    Ok(())
}
fn snapshot_roots(roots: &[(String, PathBuf)]) -> Result<Value, ReplayError> {
    let mut result = Map::new();
    for (token, path) in roots {
        result.insert(token.clone(), snapshot(path)?);
    }
    Ok(Value::Object(result))
}
fn snapshot(root: &Path) -> Result<Value, ReplayError> {
    let mut items = Vec::<Value>::new();
    fn walk(root: &Path, dir: &Path, out: &mut Vec<Value>) -> Result<(), ReplayError> {
        let mut entries = fs::read_dir(dir)
            .map_err(io_error)?
            .filter_map(Result::ok)
            .collect::<Vec<_>>();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let p = entry.path();
            let meta = fs::symlink_metadata(&p).map_err(io_error)?;
            let rel = p
                .strip_prefix(root)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/");
            let mode = mode(&meta);
            let is_dir = meta.is_dir();
            let mut item = json!({"path":rel,"mode":mode,"directory":is_dir});
            if !is_dir {
                let bytes = fs::read(&p).map_err(io_error)?;
                item["text"] = json!(String::from_utf8_lossy(&bytes));
                item["hex"] = json!(encode_hex(&bytes));
            }
            out.push(item);
            if is_dir {
                walk(root, &p, out)?;
            }
        }
        Ok(())
    }
    if root.exists() {
        walk(root, root, &mut items)?;
    }
    Ok(json!(items))
}
pub(crate) fn expand_tokens(value: &str, roots: &[(String, PathBuf)]) -> String {
    let mut result = value.to_owned();
    for (token, path) in roots {
        result = result.replace(token, &path.to_string_lossy());
    }
    result
}
fn normalize_transcript(
    value: &Value,
    roots: &[(String, PathBuf)],
    step: &Map<String, Value>,
) -> Value {
    let mut serialized = value.to_string();
    for (token, path) in roots {
        serialized = serialized.replace(&path.to_string_lossy().to_string(), token);
    }
    let pid = step
        .get("pid")
        .and_then(Value::as_u64)
        .unwrap_or(u32::MAX as u64) as u32;
    let version = step
        .get("version")
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty())
        .unwrap_or("__NO_VERSION_MATCH__");
    let normalized = crate::normalize::normalize(&serialized, "__NO_ROOT_MATCH__", pid, version)
        .replace("purchasely-cli-<MKDTEMP>", "<ROOT>");
    serde_json::from_str(&normalized).unwrap_or(value.clone())
}
fn keychain_after(step: &Map<String, Value>, host: &FakeHost) -> Value {
    if step.get("keychain").is_none_or(Value::is_null) {
        return Value::Null;
    }
    host.keychain_state()
}
fn io_error(e: std::io::Error) -> ReplayError {
    ReplayError(e.to_string())
}
#[cfg(unix)]
fn mode(p: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    p.permissions().mode() & 0o777
}
#[cfg(not(unix))]
fn mode(_: &fs::Metadata) -> u32 {
    0
}
#[cfg(unix)]
fn set_mode(p: &Path, mode: u32) -> Result<(), ReplayError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(p, fs::Permissions::from_mode(mode)).map_err(io_error)
}
#[cfg(not(unix))]
fn set_mode(_: &Path, _: u32) -> Result<(), ReplayError> {
    Ok(())
}
fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
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

pub fn bless_case(path: &Path, value: &Value) -> std::io::Result<()> {
    let mut case = value.clone();
    if let Some(steps) = case["steps"].as_array_mut() {
        for step in steps {
            if let Some(data) = step.as_object() {
                let actual = replay_step(data).map_err(|e| std::io::Error::other(e.0))?;
                for key in ["code", "out", "err", "t_after", "files_after"] {
                    if let Some(v) = actual.get(key) {
                        step[key] = v.clone();
                    }
                }
                if !step["keychain"].is_null() {
                    step["keychain"] = actual["keychain"].clone();
                }
            }
        }
    }
    case["source"] = json!("bless");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(&case).map_err(std::io::Error::other)?,
    )
}

pub fn acme_extension(_recorded: &Value) -> Extension {
    Extension {
        modules: vec![register_acme],
        surface: "extra",
        surfaces: vec![("extra", "extra")],
        surface_defs: vec![],
        identity: true,
        section: ("More commands", "Only in acme."),
        latest: None,
        npm_update: None,
    }
}
fn register_acme(parent: &mut Command, _: &mut Runtime) {
    parent.commands.push(
        Command::new("extra")
            .description("Added commands")
            .command(Command::new("run").description("Run it")),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    fn case(value: Value) -> Case {
        serde_json::from_value(value).unwrap()
    }
    #[test]
    fn pending_is_counted() {
        let cases = vec![
            case(json!({"test":"auth-status::wait","pending":"P5b","steps":[]})),
            case(
                json!({"test":"search::empty","steps":[{"argv":[],"env":{},"code":0,"out":"","err":"","calls":[],"execs":[],"prompts":[],"pid_probes":[],"files_before":{"<ROOT>":[]},"files_after":{"<ROOT>":[]}}]}),
            ),
        ];
        let report = run_cases(&cases, Some("auth-status::,search::")).unwrap();
        assert_eq!(report.pending, 1);
        assert_eq!(report.run, 1);
        assert_eq!(report.filtered, 0);
    }
    #[test]
    fn bless_writes_case() {
        let p = std::env::temp_dir().join(format!("contract-bless-{}", std::process::id()));
        bless_case(&p, &json!({"test":"x","steps":[]})).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&fs::read(&p).unwrap()).unwrap()["source"],
            "bless"
        );
        let _ = fs::remove_file(p);
    }
    #[test]
    fn filter_rejects_unmatched_prefix() {
        assert!(selected(&[json!({"test":"auth::x"})], Some("search::")).is_err());
    }
    #[test]
    fn acme_extension_registers_extra_run() {
        let ext = acme_extension(&Value::Null);
        let mut root = Command::new("acme");
        let host = crate::fake::FakeHost::from_step(&json!({"argv":[],"env":{}}));
        let mut rt = Runtime::new(host.deps(vec![]));
        (ext.modules[0])(&mut root, &mut rt);
        assert_eq!(root.commands[0].name, "extra");
        assert_eq!(root.commands[0].commands[0].name, "run");
    }
    #[test]
    fn replay_step_restores_and_compares_file_snapshot() {
        let bytes = b"initial";
        let step = json!({"argv":[],"env":{},"name":"purchasely","version":"9.9.9","host":"https://cli.test","code":0,"out":"","err":"","calls":[],"execs":[],"prompts":[],"pid_probes":[],"files_before":{"<ROOT>":[{"path":"sample.txt","mode":384,"directory":false,"text":"initial","hex":"696e697469616c"}]},"files_after":{"<ROOT>":[{"path":"sample.txt","mode":384,"directory":false,"text":"initial","hex":"696e697469616c"}]}});
        let _ = bytes;
        assert!(replay_step(step.as_object().unwrap()).is_ok());
    }
    #[test]
    fn drift_directory_replays_new_and_unit_cases() {
        let base = std::env::temp_dir().join(format!("contract-drift-test-{}", std::process::id()));
        let committed = base.join("committed");
        let fresh = base.join("fresh");
        fs::create_dir_all(&committed).unwrap();
        fs::create_dir_all(&fresh).unwrap();
        fs::write(
            committed.join("unit.json"),
            r#"{"cases":[{"test":"auth::migrated","steps":[],"unit":"auth::tests::ported"}]}"#,
        )
        .unwrap();
        fs::write(fresh.join("new.json"), r#"{"cases":[]}"#).unwrap();
        assert_eq!(compare_dirs(&committed, &fresh).unwrap(), (0, 1, 1));
        let _ = fs::remove_dir_all(base);
    }
}
