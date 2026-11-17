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

/// What a case directory tells about the machine that recorded it: the repo root of the replay (for `<REPO>`)
/// and the temp dirs of the TS run (the parents of its `purchasely-cli-` roots).
struct Recording {
    repo: Option<PathBuf>,
    ts_tmp: Vec<String>,
}
static RECORDING: std::sync::Mutex<Recording> = std::sync::Mutex::new(Recording {
    repo: None,
    ts_tmp: Vec::new(),
});

/// `<REPO>` (a repo path that the recorder wrote relative to its clone) becomes the repo root of the replay.
fn expand_repo(value: &str) -> String {
    match &RECORDING.lock().expect("recording lock").repo {
        Some(repo) if value.contains("<REPO>") => value.replace("<REPO>", &repo.to_string_lossy()),
        _ => value.to_owned(),
    }
}
/// The repo root of the replay becomes `<REPO>` in a comparison.
pub(crate) fn collapse_repo(value: &str) -> String {
    match &RECORDING.lock().expect("recording lock").repo {
        Some(repo) => value.replace(&*repo.to_string_lossy(), "<REPO>"),
        None => value.to_owned(),
    }
}
/// The parents of the `purchasely-cli-` roots that the recorded text names: the temp dirs of the TS run.
fn ts_tmp_dirs(text: &str) -> Vec<String> {
    let mut dirs = Vec::<String>::new();
    let path_char = |c: char| c.is_ascii_alphanumeric() || "/._-+".contains(c);
    let mut rest = text;
    while let Some(index) = rest.find("/purchasely-cli-") {
        let head = &rest[..index];
        let start = head
            .char_indices()
            .rev()
            .take_while(|(_, c)| path_char(*c))
            .last()
            .map_or(index, |(i, _)| i);
        let dir = &head[start..];
        if dir.starts_with('/') && dir.len() > 1 && !dirs.iter().any(|d| d == dir) {
            dirs.push(dir.to_owned());
        }
        rest = &rest[index + 1..];
    }
    dirs
}
/// A recorded input path that no root maps reaches a file of the recording machine: a replay elsewhere would
/// not find it. `tokenized` is the input after the root tokens replaced the recorded roots.
fn check_unmapped(tokenized: &str) -> Result<(), ReplayError> {
    let recording = RECORDING.lock().expect("recording lock");
    for dir in &recording.ts_tmp {
        if let Some(at) = tokenized.find(&format!("{dir}/")) {
            let path: String = tokenized[at..]
                .chars()
                .take_while(|c| !c.is_whitespace() && !"\"'`".contains(*c))
                .collect();
            return Err(ReplayError(format!(
                "unmapped TS temp path {path}: record its folder as a root (tmp_roots) or create it under the setup root"
            )));
        }
    }
    // A home path is a clone or a user folder of the recording machine; the recorder writes the repo as `<REPO>`.
    for home in ["/Users/", "/home/"] {
        if let Some(at) = tokenized.find(home) {
            let path: String = tokenized[at..]
                .chars()
                .take_while(|c| !c.is_whitespace() && !"\"'`".contains(*c))
                .collect();
            return Err(ReplayError(format!(
                "unmapped recording path {path}: record repo paths as <REPO>"
            )));
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReplayReport {
    pub run: usize,
    pub pending: usize,
    pub filtered: usize,
}

pub fn pending_count(cases: &[Value]) -> usize {
    cases.iter().filter(|c| c.get("pending").is_some()).count()
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
    // `read_dir` order differs between file systems (sorted on APFS, not on ext4): replay in one order.
    files.sort_by(|a, b| a.0.cmp(&b.0));
    // CONTRACT_ORDER=reverse replays the files last to first: a case that fails only then reads state
    // that an earlier case left in the process.
    if std::env::var("CONTRACT_ORDER").is_ok_and(|v| v == "reverse") {
        files.reverse();
    }
    let flat = files
        .iter()
        .flat_map(|(_, cases)| cases.iter().cloned())
        .collect::<Vec<_>>();
    {
        let mut recording = RECORDING.lock().expect("recording lock");
        // `contract/cases` sits in the repo root.
        recording.repo = directory.join("../..").canonicalize().ok();
        recording.ts_tmp = ts_tmp_dirs(&Value::Array(flat.clone()).to_string());
    }
    let selected = selected(&flat, filter)?;
    let mut report = ReplayReport {
        filtered: flat.len() - selected.len(),
        ..ReplayReport::default()
    };
    let mut updated = flat.clone();
    // CONTRACT_PENDING=1 also replays the pending cases, to find the ones that pass now.
    let with_pending = std::env::var("CONTRACT_PENDING").is_ok_and(|v| v == "1");
    let mut failures = Vec::<String>::new();
    for old in selected {
        let index = flat.iter().position(|c| std::ptr::eq(c, old)).unwrap();
        if old.get("pending").is_some() && !with_pending {
            report.pending += 1;
            continue;
        }
        let mut case = old.clone();
        let Some(steps) = case["steps"].as_array_mut() else {
            continue;
        };
        for (step_index, step) in steps.iter_mut().enumerate() {
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
            } else if let Err(e) = replay_step(data) {
                let tag = old
                    .get("pending")
                    .map_or(String::new(), |p| format!(" [pending {p}]"));
                failures.push(format!("{}{tag}: step {step_index}: {}", old["test"], e.0));
                break;
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
            let mut changed = false;
            for case in cases.iter_mut() {
                changed |= *case != updated[offset];
                *case = updated[offset].clone();
                offset += 1;
            }
            // Write only the files of the blessed cases: a rewrite of the others changes their key order only.
            if !changed {
                continue;
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
        "run {}, pending {}, filtered {}, failed {}",
        report.run,
        report.pending,
        report.filtered,
        failures.len()
    );
    // Every failing case, not only the first one.
    if !failures.is_empty() {
        return Err(failures.join("\n"));
    }
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
        let mut argv: Vec<String> =
            serde_json::from_value(step.get("argv").cloned().unwrap_or(json!([])))
                .unwrap_or_default();
        let mut tmp = ts_tmp_roots(step);
        // The longest path first: a root can hold another one.
        tmp.sort_by_key(|(path, _)| std::cmp::Reverse(path.len()));
        // The extra roots first: one can be the `purchasely-cli-` root of another setup, not this step's root.
        let to_replay = |s: &str| -> Result<String, ReplayError> {
            let mut s = s.to_owned();
            for (path, token) in &tmp {
                s = s.replace(path, token);
            }
            let s = crate::normalize::map_ts_root(&s, "<ROOT>");
            check_unmapped(&s)?;
            Ok(expand_tokens(&s, &roots))
        };
        // A restored text file can hold a TS temp path (the targets of ai.plugin.json): it gets the replay path.
        let mut files_before = step.get("files_before").cloned().unwrap_or(Value::Null);
        for entry in files_before
            .as_object_mut()
            .into_iter()
            .flat_map(|m| m.values_mut())
            .filter_map(Value::as_array_mut)
            .flatten()
        {
            let utf8 = entry["hex"]
                .as_str()
                .and_then(decode_hex)
                .is_some_and(|b| String::from_utf8(b).is_ok());
            if let (true, Some(text)) = (utf8, entry["text"].as_str()) {
                entry["text"] = json!(to_replay(text)?);
                if let Some(o) = entry.as_object_mut() {
                    o.remove("hex");
                }
            }
        }
        apply_snapshots(&files_before, &roots)?;
        for arg in &mut argv {
            *arg = to_replay(arg)?;
        }
        let mut replay_step = step.clone();
        if let Some(env) = replay_step.get_mut("env").and_then(Value::as_object_mut) {
            for value in env.values_mut() {
                if let Some(s) = value.as_str() {
                    *value = json!(to_replay(s)?);
                }
            }
        }
        let host = FakeHost::with_root(&Value::Object(replay_step), root.clone());
        let deps = host.deps(argv.clone());
        let code = purchasely::run::run(argv, deps);
        host.finish()
            .map_err(|e| ReplayError(format!("{}; stderr was {:?}", e.0, host.stderr_text())))?;
        let files_after = snapshot_roots(&roots)?;
        let actual = json!({"code":code,"out":host.stdout_text(),"err":host.stderr_text(),"t_after":host.now(),"files_after":files_after,"keychain":keychain_after(step,&host)});
        // stderr first: it holds the error that explains a wrong code or output.
        for key in ["err", "out", "code"] {
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
        if let Some(expected) = step.get("files_after") {
            let got = sort_files(normalize_transcript(&actual["files_after"], &roots, step));
            let want = sort_files(normalize_transcript(expected, &[], step));
            if got != want {
                return Err(ReplayError(format!(
                    "files_after differs: {}",
                    files_diff(&want, &got)
                )));
            }
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
/// The extra temp dirs of the TS run (`snapshotRoots` of `test/helpers.ts`): an absolute env value whose last
/// component is a `mkdtemp` name (`cli-ai-home-AbC123`) is the dir of `<TMPn>`, in env order.
pub(crate) fn ts_tmp_roots(step: &Map<String, Value>) -> Vec<(String, String)> {
    // A newer recording names each extra root (`tmp_roots`): no guess.
    if let Some(recorded) = step.get("tmp_roots").and_then(Value::as_object) {
        return recorded
            .iter()
            .filter_map(|(token, path)| Some((path.as_str()?.to_owned(), token.clone())))
            .collect();
    }
    let count = step
        .get("files_before")
        .and_then(Value::as_object)
        .map_or(0, |m| m.keys().filter(|k| k.starts_with("<TMP")).count());
    let mut out: Vec<(String, String)> = Vec::new();
    for value in step
        .get("env")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|m| m.values())
    {
        let Some(path) = value
            .as_str()
            .filter(|p| p.starts_with('/') && !p.contains("purchasely-cli-"))
        else {
            continue;
        };
        let last = path.rsplit('/').next().unwrap_or_default();
        let mkdtemp = last.rsplit_once('-').is_some_and(|(_, tail)| {
            tail.len() == 6 && tail.chars().all(|c| c.is_ascii_alphanumeric())
        });
        if mkdtemp && out.len() < count && !out.iter().any(|(p, _)| p == path) {
            out.push((path.to_owned(), format!("<TMP{}>", out.len())));
        }
    }
    out
}
pub(crate) fn expand_tokens(value: &str, roots: &[(String, PathBuf)]) -> String {
    let mut result = expand_repo(value);
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
    // The longest path first: the root path is a prefix of the `<TMPn>` paths.
    let mut roots = roots.to_vec();
    roots.sort_by_key(|(_, path)| std::cmp::Reverse(path.as_os_str().len()));
    for (token, path) in &roots {
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
    for (path, token) in ts_tmp_roots(step) {
        serialized = serialized.replace(&path, &token);
    }
    let serialized = collapse_repo(&serialized);
    let normalized = crate::normalize::normalize(&serialized, "__NO_ROOT_MATCH__", pid, version)
        .replace("purchasely-cli-<MKDTEMP>", "<ROOT>");
    serde_json::from_str(&normalized).unwrap_or(value.clone())
}
/// A snapshot lists entries in walk order; TS and Rust walk differently, so compare by path.
/// `hex` is dropped when it is UTF-8: `text` then holds the same bytes, normalized (a UUID becomes `<UUIDn>`).
fn sort_files(mut v: Value) -> Value {
    if let Some(roots) = v.as_object_mut() {
        for entries in roots.values_mut() {
            if let Some(list) = entries.as_array_mut() {
                for entry in list.iter_mut() {
                    let utf8 = entry["hex"]
                        .as_str()
                        .and_then(decode_hex)
                        .is_some_and(|b| String::from_utf8(b).is_ok());
                    if utf8 && let Some(o) = entry.as_object_mut() {
                        o.remove("hex");
                    }
                }
                list.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
            }
        }
    }
    v
}
/// The entries that only one side has, or that differ, by root and path.
fn files_diff(want: &Value, got: &Value) -> String {
    let entries = |v: &Value| -> Vec<(String, Value)> {
        let mut out = Vec::new();
        for (root, list) in v.as_object().into_iter().flatten() {
            for e in list.as_array().into_iter().flatten().cloned() {
                out.push((
                    format!("{root}/{}", e["path"].as_str().unwrap_or_default()),
                    e,
                ));
            }
        }
        out
    };
    let (want, got) = (entries(want), entries(got));
    let mut out = Vec::new();
    for (path, w) in &want {
        match got.iter().find(|(p, _)| p == path) {
            None => out.push(format!("missing {path}")),
            Some((_, g)) if g != w => {
                let (wt, gt) = (
                    w["text"].as_str().unwrap_or(""),
                    g["text"].as_str().unwrap_or(""),
                );
                let at = wt
                    .chars()
                    .zip(gt.chars())
                    .take_while(|(a, b)| a == b)
                    .count();
                // CONTRACT_FULL=1 prints the whole texts.
                let full = std::env::var("CONTRACT_FULL").is_ok_and(|v| v == "1");
                let around = |t: &str| {
                    if full {
                        return t.to_owned();
                    }
                    t.chars()
                        .skip(at.saturating_sub(60))
                        .take(160)
                        .collect::<String>()
                };
                out.push(format!(
                    "{path}: mode {} vs {}; text differs at char {at}: expected {:?}, got {:?}",
                    w["mode"],
                    g["mode"],
                    around(wt),
                    around(gt)
                ))
            }
            _ => {}
        }
    }
    for (path, _) in got
        .iter()
        .filter(|(p, _)| !want.iter().any(|(w, _)| w == p))
    {
        out.push(format!("extra {path}"));
    }
    if out.is_empty() {
        out.push(format!(
            "same entries, other roots or order: {want:?} vs {got:?}"
        ));
    }
    out.join("; ")
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

thread_local! {
    /// The commands of the recorded extension modules (`ext.commands` of `test/helpers.ts`), for `register_recorded`.
    static RECORDED_COMMANDS: std::cell::RefCell<Value> = const { std::cell::RefCell::new(Value::Null) };
}

static EXTENSION: std::sync::OnceLock<fn(&Value) -> Extension> = std::sync::OnceLock::new();

/// Installs the extension that a step with a recorded `ext` replays with. A workspace with its own extension calls it
/// once with its own builder; without a call, the replay uses the `acme` test extension.
pub fn set_extension(build: fn(&Value) -> Extension) {
    let _ = EXTENSION.set(build);
}

/// The extension of a recorded step: the installed builder, else `acme`.
pub fn recorded_extension(recorded: &Value) -> Extension {
    EXTENSION
        .get()
        .map_or_else(|| acme_extension(recorded), |build| build(recorded))
}

pub fn acme_extension(recorded: &Value) -> Extension {
    let commands = recorded["commands"].clone();
    let modules: Vec<fn(&mut Command, &mut Runtime)> = if commands.is_array() {
        RECORDED_COMMANDS.with(|c| *c.borrow_mut() = commands);
        vec![register_recorded]
    } else {
        vec![register_acme]
    };
    Extension {
        modules,
        surface: "extra",
        surfaces: vec![("extra", "extra")],
        surface_defs: vec![],
        identity: true,
        section: (
            leak(
                recorded["section"]["title"]
                    .as_str()
                    .unwrap_or("More commands"),
            ),
            leak(
                recorded["section"]["note"]
                    .as_str()
                    .unwrap_or("Only in `acme`."),
            ),
        ),
        latest: (!recorded["latest"].is_null())
            .then_some(acme_latest as purchasely::core::ext::LatestFn),
        // The harness lives for one test run: leaking the recorded text gives the &'static str of the type.
        npm_update: recorded["npm_update"].as_str().map(leak),
    }
}
fn leak(s: &str) -> &'static str {
    Box::leak(s.to_owned().into_boxed_str())
}

/// `ACME.latest` of `test/update.test.ts`, the only recorded extension with a channel: `gh release view`
/// (replayed from the recorded execs), then the stable version of its tag.
fn acme_latest(
    d: &purchasely::core::deps::Deps,
) -> Result<Option<String>, purchasely::core::errors::CliError> {
    use purchasely::core::{
        deps::ExecOpts,
        errors::{CliError, EXIT},
    };
    let fail = || {
        CliError::new(
            EXIT::AUTH,
            "UPDATE_CHECK_FAILED",
            vec!["could not check the latest version".into()],
        )
    };
    let r = d
        .host
        .exec(
            "gh",
            &["release".into(), "view".into()],
            ExecOpts {
                timeout_ms: 15_000,
                ..ExecOpts::default()
            },
        )
        .map_err(|_| fail())?;
    if r.code != 0 {
        return Err(fail());
    }
    let tag = r.stdout.trim();
    Ok(purchasely::core::update::stable_version(
        tag.strip_prefix('v').unwrap_or(tag),
    ))
}
/// Build the commands that the TS extension modules added, as the recorder saw them.
fn register_recorded(parent: &mut Command, _: &mut Runtime) {
    fn build(v: &Value) -> Command {
        let text = |k: &str| v[k].as_str().unwrap_or_default().to_owned();
        let mut c = Command::new(text("name")).description(text("description"));
        for o in v["options"].as_array().into_iter().flatten() {
            let flags = o["flags"].as_str().unwrap_or_default();
            let description = o["description"].as_str().unwrap_or_default();
            let mut option = if flags.contains(['<', '[']) {
                purchasely::cli::OptionDef::value(flags, description)
            } else {
                purchasely::cli::OptionDef::boolean(flags, description)
            };
            option.hidden = o["hidden"].as_bool().unwrap_or(false);
            c = c.option(option);
        }
        for sub in v["commands"].as_array().into_iter().flatten() {
            c = c.command(build(sub));
        }
        if v["hidden"].as_bool().unwrap_or(false) {
            c = c.hide();
        }
        if v["action"].as_bool().unwrap_or(false) {
            c = c.action(|_, _| Ok(()));
        }
        c
    }
    RECORDED_COMMANDS.with(|recorded| {
        for v in recorded.borrow().as_array().into_iter().flatten() {
            parent.commands.push(build(v));
        }
    });
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
    fn ts_root_maps_in_argv_and_json() {
        assert_eq!(
            crate::normalize::map_ts_root(
                "--plan=/var/folders/x/T/purchasely-cli-nfbwWw/plan.json",
                "/r"
            ),
            "--plan=/r/plan.json"
        );
        assert_eq!(
            crate::normalize::map_ts_root(
                r#"{"p":"/tmp/purchasely-cli-ab12/x","q":"purchasely-cli-<MKDTEMP>"}"#,
                "<ROOT>"
            ),
            r#"{"p":"<ROOT>/x","q":"purchasely-cli-<MKDTEMP>"}"#
        );
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
    fn ts_tmp_dirs_are_the_parents_of_the_recorded_roots() {
        assert_eq!(
            ts_tmp_dirs(
                r#"["/var/t/purchasely-cli-ab12/x","/var/t/purchasely-cli-cd34","purchasely-cli-<MKDTEMP>"]"#
            ),
            vec!["/var/t".to_owned()]
        );
    }
    #[test]
    fn an_unmapped_recording_path_fails_the_step() {
        RECORDING.lock().unwrap().ts_tmp = vec!["/rec-tmp".into()];
        let err = check_unmapped("--in=/rec-tmp/dump-Ab12Cd/x.jsonl").unwrap_err();
        assert!(err.0.contains("/rec-tmp/dump-Ab12Cd/x.jsonl"), "{}", err.0);
        assert!(check_unmapped("--in=<TMP0>/x.jsonl").is_ok());
        assert!(check_unmapped("/Users/me/clone/test/fixtures").is_err());
        assert!(check_unmapped("<REPO>/test/fixtures").is_ok());
        let step =
            json!({"argv":["--in","/rec-tmp/dump-Ab12Cd"],"env":{},"code":0,"out":"","err":""});
        assert!(
            replay_step(step.as_object().unwrap())
                .unwrap_err()
                .0
                .contains("unmapped TS temp path")
        );
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
}
