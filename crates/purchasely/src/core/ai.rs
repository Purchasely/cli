//! Purchasely AI plugin helpers shared by the command and daily updater.
use super::{
    deps::{Deps, Request},
    errors::{CliError, EXIT, usage},
    json::{self, Value},
    runtime::Runtime,
};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub const AGENTS: [&str; 6] = ["claude", "codex", "copilot", "gemini", "cursor", "opencode"];
const API: &str = "https://api.github.com/repos/Purchasely/Purchasely-AI-Plugin/releases/latest";
const REPO: &str = "Purchasely/Purchasely-AI-Plugin";
const TARBALL: &str =
    "https://codeload.github.com/Purchasely/Purchasely-AI-Plugin/tar.gz/refs/tags/";
const MARKER: &str = ".purchasely-cli";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    Install,
    Update,
    Uninstall,
    Status,
}

pub fn offline(d: &Deps) -> bool {
    d.env.get("PURCHASELY_OFFLINE").is_some_and(|x| x == "1")
}
pub fn follows(agent: &str) -> &'static str {
    if matches!(agent, "claude" | "codex" | "copilot" | "gemini") {
        "default branch"
    } else {
        "release tag"
    }
}
pub fn valid_tag(tag: &str) -> bool {
    let v = tag.strip_prefix('v').unwrap_or(tag);
    let p: Vec<_> = v.split('.').collect();
    p.len() == 3
        && p.iter()
            .all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}
fn version_tag(tag: &str) -> &str {
    tag.strip_prefix('v').unwrap_or(tag)
}
/// TS `ai.ts` imports `newer` from `update.ts`.
use super::update::newer;
pub fn latest_tag(d: &Deps) -> Result<String, CliError> {
    let r = d.host.fetch(Request {
        method: "GET".into(),
        url: API.into(),
        headers: vec![
            ("User-Agent".into(), format!("purchasely-cli/{}", d.version)),
            ("Accept".into(), "application/vnd.github+json".into()),
        ],
        timeout_ms: 15_000,
        ..Request::default()
    });
    let parsed = r
        .ok()
        .filter(|x| x.status == 200)
        .and_then(|x| String::from_utf8(x.body).ok())
        .and_then(|x| json::parse(&x).ok());
    let tag = parsed
        .as_ref()
        .and_then(|x| get_string(x, "tag_name"))
        .unwrap_or_default();
    if valid_tag(&tag) {
        Ok(tag)
    } else {
        Err(CliError::new(
            EXIT::GENERIC,
            "AI_RELEASE_UNREADABLE",
            vec!["could not read the latest plugin release. Check your network.".into()],
        ))
    }
}
pub fn detect(d: &Deps, agent: &str) -> bool {
    if agent == "cursor" {
        return d
            .env
            .get("HOME")
            .is_some_and(|h| std::path::Path::new(h).join(".cursor").is_dir());
    }
    d.host
        .exec(
            agent,
            &["--version".into()],
            super::deps::ExecOpts {
                timeout_ms: 120_000,
                ..Default::default()
            },
        )
        .is_ok_and(|x| x.code == 0 && !x.timed_out)
}
fn get_string(v: &Value, key: &str) -> Option<String> {
    if let Value::Object(xs) = v {
        xs.iter()
            .find(|(k, _)| k == key)
            .and_then(|(_, v)| v.as_str())
    } else {
        None
    }
}

fn prop(v: &Value, key: &str) -> Value {
    if let Value::Object(xs) = v {
        xs.iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .unwrap_or(Value::Null)
    } else {
        Value::Null
    }
}
fn array(v: &Value) -> Vec<Value> {
    if let Value::Array(xs) = v {
        xs.clone()
    } else {
        Vec::new()
    }
}
fn plugin(d: &Deps) -> Value {
    let store = state_store(d);
    store.read(
        "plugin",
        Value::Object(vec![("agents".into(), Value::Object(vec![]))]),
    )
}
fn state_store(d: &Deps) -> super::state::StateStore<'_> {
    super::state::StateStore::new(d, "ai")
}
fn agent_map(v: &Value) -> Value {
    prop(v, "agents")
}
fn registration(d: &Deps, agent: &str) -> Option<Value> {
    let map = agent_map(&plugin(d));
    if let Value::Object(xs) = map {
        xs.into_iter().find(|(k, _)| k == agent).map(|(_, v)| v)
    } else {
        None
    }
}
/// JS `{ ...o, key: value }`: an existing key keeps its place, a new key goes last.
fn set(fields: &mut Vec<(String, Value)>, key: &str, value: Value) {
    match fields.iter_mut().find(|(k, _)| k == key) {
        Some(slot) => slot.1 = value,
        None => fields.push((key.into(), value)),
    }
}
/// TS `Session.put`: re-read, change the one agent, set `latest` when known, write.
fn write_registration(
    d: &Deps,
    agent: &str,
    value: Option<Value>,
    latest: Option<&str>,
) -> Result<(), CliError> {
    let old = plugin(d);
    let mut entries = if let Value::Object(xs) = agent_map(&old) {
        xs
    } else {
        vec![]
    };
    match value {
        Some(v) => set(&mut entries, agent, v),
        None => entries.retain(|(k, _)| k != agent),
    }
    let mut root = if let Value::Object(xs) = old {
        xs
    } else {
        vec![]
    };
    set(&mut root, "agents", Value::Object(entries));
    if let Some(tag) = latest {
        set(&mut root, "latest", Value::from(tag.to_owned()));
    }
    // TS `Session.put` lets the write throw: the command fails instead of reporting a lost record.
    state_store(d)
        .write("plugin", &Value::Object(root))
        .map_err(|e| super::breaker::state_error(e.to_string()))
}
fn arr_str(v: &[String]) -> Value {
    Value::Array(v.iter().cloned().map(Value::from).collect())
}
fn agent_args(agent: &str, verb: Verb) -> Vec<Vec<String>> {
    let id = match agent {
        "claude" | "copilot" => "purchasely@Purchasely-AI-Plugin",
        "codex" => "purchasely@purchasely",
        _ => "purchasely",
    };
    let steps: Vec<Vec<&str>> = match (agent, verb) {
        ("claude", Verb::Install) => vec![vec!["plugin", "install", id, "-s", "user"]],
        ("claude", Verb::Update) => vec![
            vec!["plugin", "marketplace", "update", "Purchasely-AI-Plugin"],
            vec!["plugin", "update", id, "-s", "user"],
        ],
        ("claude", Verb::Uninstall) => vec![vec!["plugin", "uninstall", id, "-s", "user"]],
        ("codex", Verb::Install) => vec![vec!["plugin", "add", id]],
        ("codex", Verb::Update) => vec![vec!["plugin", "marketplace", "upgrade", "purchasely"]],
        ("codex", Verb::Uninstall) => vec![vec!["plugin", "remove", id]],
        ("copilot", Verb::Install) => vec![vec!["plugin", "install", id]],
        ("copilot", Verb::Update) => vec![vec!["plugin", "update", id]],
        ("copilot", Verb::Uninstall) => vec![vec!["plugin", "uninstall", id]],
        ("gemini", Verb::Install) => vec![vec![
            "extensions",
            "install",
            "https://github.com/Purchasely/Purchasely-AI-Plugin",
            "--consent",
        ]],
        ("gemini", Verb::Update) => vec![vec!["extensions", "update", "purchasely"]],
        ("gemini", Verb::Uninstall) => vec![vec!["extensions", "uninstall", "purchasely"]],
        _ => vec![],
    };
    steps
        .iter()
        .map(|s| s.iter().map(|x| (*x).to_owned()).collect())
        .collect()
}
fn is_good(d: &Deps, agent: &str, args: &[String]) -> Option<super::deps::ExecResult> {
    d.host
        .exec(
            agent,
            args,
            super::deps::ExecOpts {
                timeout_ms: 120_000,
                ..Default::default()
            },
        )
        .ok()
        .filter(|r| r.code == 0 && !r.timed_out)
}
fn native_list(agent: &str) -> Vec<String> {
    match agent {
        "claude" | "copilot" => ["plugin", "list", "--json"].map(str::to_owned).to_vec(),
        "codex" => ["plugin", "list", "--json", "-m", "purchasely"]
            .map(str::to_owned)
            .to_vec(),
        "gemini" => ["extensions", "list"].map(str::to_owned).to_vec(),
        _ => vec![],
    }
}
fn native_entries(agent: &str, output: &str) -> Option<Vec<Value>> {
    if agent == "gemini" {
        return Some(
            output
                .lines()
                .filter(|line| {
                    let s = line.trim_start_matches(|c: char| !c.is_ascii_alphanumeric());
                    s == "purchasely"
                        || s.starts_with("purchasely ")
                        || s.starts_with("purchasely(")
                })
                .map(|_| Value::Object(vec![("name".into(), Value::from("purchasely".to_owned()))]))
                .collect(),
        );
    }
    let value = json::parse(output).ok()?;
    let xs = if agent == "codex" {
        array(&prop(&value, "installed"))
    } else {
        array(&value)
    };
    let name = match agent {
        "claude" => "id",
        "codex" => "pluginId",
        _ => "name",
    };
    let wanted = match agent {
        "claude" => "purchasely@Purchasely-AI-Plugin",
        "copilot" => "purchasely",
        "codex" => "purchasely@purchasely",
        _ => "purchasely",
    };
    Some(
        xs.into_iter()
            .filter(|entry| {
                get_string(entry, name).as_deref() == Some(wanted)
                    && (agent != "codex" || prop(entry, "installed") != Value::Bool(false))
                    && (agent != "copilot"
                        || get_string(entry, "marketplace").as_deref()
                            == Some("Purchasely-AI-Plugin"))
            })
            .map(|entry| {
                let keys: &[&str] = match agent {
                    "claude" => &["id", "scope", "installedAt"],
                    "codex" => &["pluginId", "marketplaceName"],
                    _ => &["name", "marketplace"],
                };
                Value::Object(
                    keys.iter()
                        .filter_map(|key| {
                            let v = prop(&entry, key);
                            (v != Value::Null).then(|| ((*key).to_owned(), v))
                        })
                        .collect(),
                )
            })
            .collect(),
    )
}
fn identity(d: &Deps, agent: &str, label: &str) -> Result<Value, String> {
    let result = is_good(d, agent, &native_list(agent));
    let entries = result
        .as_ref()
        .and_then(|r| native_entries(agent, &r.stdout));
    if entries.as_ref().is_some_and(|v| v.len() == 1) {
        return Ok(entries.unwrap().remove(0));
    }
    Err(format!(
        "{agent}: the plugin {label} check failed (exit {}). Run \"{}\" yourself to see why.",
        result.map(|r| r.code).unwrap_or(127),
        std::iter::once(agent.to_owned())
            .chain(native_list(agent))
            .collect::<Vec<_>>()
            .join(" ")
    ))
}
fn refuse(
    d: &Deps,
    agent: &str,
    reg: &Value,
    reason: &str,
    drop: bool,
    dry_run: bool,
) -> Result<Value, CliError> {
    let version = get_string(reg, "version");
    let message = if drop {
        format!("{agent}: the plugin is gone from the agent. The record was dropped.")
    } else {
        format!("{agent}: {reason}. Nothing was changed.")
    };
    // TS `refused`: a dry run changes no state.
    if dry_run {
    } else if drop {
        write_registration(d, agent, None, None)?;
    } else {
        let mut updated = if let Value::Object(xs) = reg.clone() {
            xs
        } else {
            vec![]
        };
        updated.retain(|(k, _)| k != "last_error");
        updated.push(("last_error".into(), Value::from(message.clone())));
        write_registration(d, agent, Some(Value::Object(updated)), None)?;
    }
    Ok(agent_row(agent, "refused", version, Some(message), vec![]))
}
fn marker(path: &Path) -> Option<String> {
    fs::read_to_string(path.join(MARKER))
        .ok()
        .map(|s| s.trim().to_owned())
}
fn cursor_target(d: &Deps) -> PathBuf {
    d.env
        .get("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join(".cursor/plugins/local/purchasely")
}
fn opencode_root(d: &Deps) -> PathBuf {
    d.env
        .get("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join(".config/opencode")
}
fn targets(d: &Deps, agent: &str) -> Vec<PathBuf> {
    if agent == "cursor" {
        vec![cursor_target(d)]
    } else {
        let root = opencode_root(d);
        vec![
            root.join("skills/purchasely-integrate"),
            root.join("references"),
        ]
    }
}
fn copy_tree(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &to)?;
        } else {
            fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}
fn copy_to(src: &Path, dst: &Path, version: &str, pid: u32) -> io::Result<()> {
    let parent = dst.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let tmp = PathBuf::from(format!("{}.tmp-{pid}", dst.display()));
    let old = PathBuf::from(format!("{}.old-{pid}", dst.display()));
    let _ = fs::remove_dir_all(&tmp);
    copy_tree(src, &tmp)?;
    if dst.exists() {
        fs::rename(dst, &old)?;
    }
    fs::rename(&tmp, dst)?;
    let _ = fs::remove_dir_all(old);
    fs::write(dst.join(MARKER), format!("{version}\n"))?;
    Ok(())
}
fn release_payload(d: &Deps, raw_tag: &str) -> Result<(PathBuf, PathBuf), CliError> {
    let cache = d.cache_dir.join("ai");
    let _ = fs::create_dir_all(&cache);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&d.cache_dir, fs::Permissions::from_mode(0o700));
        let _ = fs::set_permissions(&cache, fs::Permissions::from_mode(0o700));
    }
    // TS `mkdtempSync(join(cache, "ai", "release-"))`: six random characters, a new folder, mode 0700.
    let dir = (0..100)
        .find_map(|_| {
            const ALNUM: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
            let mut bytes = [0u8; 6];
            getrandom::fill(&mut bytes).ok()?;
            let name: String = bytes
                .iter()
                .map(|b| ALNUM[*b as usize % ALNUM.len()] as char)
                .collect();
            let dir = cache.join(format!("release-{name}"));
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
            builder.create(&dir).ok().map(|_| dir)
        })
        .ok_or_else(download_error)?;
    let response = d
        .host
        .fetch(Request {
            method: "GET".into(),
            url: format!("{TARBALL}{raw_tag}"),
            headers: vec![
                ("User-Agent".into(), format!("purchasely-cli/{}", d.version)),
                ("Accept".into(), "application/vnd.github+json".into()),
            ],
            timeout_ms: 60_000,
            ..Request::default()
        })
        .ok()
        .filter(|r| r.status == 200)
        .ok_or_else(download_error)?;
    let archive = dir.join("plugin.tgz");
    fs::write(&archive, response.body).map_err(|_| download_error())?;
    let args = vec![
        "-xzf".into(),
        archive.to_string_lossy().into_owned(),
        "-C".into(),
        dir.to_string_lossy().into_owned(),
    ];
    if is_good(d, "tar", &args).is_none() {
        let _ = fs::remove_dir_all(&dir);
        return Err(download_error());
    }
    let src = dir.join(format!(
        "Purchasely-AI-Plugin-{}/purchasely",
        version_tag(raw_tag)
    ));
    if !src.exists() {
        let _ = fs::remove_dir_all(&dir);
        return Err(download_error());
    }
    Ok((dir, src))
}
fn download_error() -> CliError {
    CliError::new(
        EXIT::GENERIC,
        "AI_FAILED",
        vec!["could not download the plugin release. Check your network.".into()],
    )
}
/// TS `layout(d, a, src)`: every (source, destination) dir of a copy agent, read from the payload.
fn layout(d: &Deps, agent: &str, src: &Path) -> Vec<(PathBuf, PathBuf)> {
    if agent == "cursor" {
        vec![(src.to_path_buf(), cursor_target(d))]
    } else {
        let mut out = Vec::new();
        let skills = src.join("skills");
        if let Ok(entries) = fs::read_dir(&skills) {
            for entry in entries
                .flatten()
                .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            {
                out.push((
                    entry.path(),
                    opencode_root(d).join("skills").join(entry.file_name()),
                ));
            }
        }
        out.push((src.join("references"), opencode_root(d).join("references")));
        out
    }
}
fn copy_layout(d: &Deps, agent: &str, src: &Path, version: &str) -> Result<Vec<PathBuf>, CliError> {
    let dirs = layout(d, agent, src);
    let mut written = Vec::new();
    for (from, to) in dirs {
        copy_to(&from, &to, version, d.pid).map_err(|_| download_error())?;
        written.push(to);
    }
    Ok(written)
}
fn managed_status(d: &Deps, agent: &str) -> Value {
    let reg = registration(d, agent);
    let detected = detect(d, agent);
    let hand = if reg.is_some() || !detected {
        Value::Bool(false)
    } else if agent == "cursor" {
        Value::Bool(
            targets(d, agent)
                .iter()
                .any(|p| p.exists() && marker(p).is_none()),
        )
    } else if agent == "opencode" {
        Value::from("unknown".to_owned())
    } else {
        match is_good(d, agent, &native_list(agent)) {
            Some(r) => Value::Bool(native_entries(agent, &r.stdout).is_some_and(|v| !v.is_empty())),
            None => Value::from("unknown".to_owned()),
        }
    };
    let val = |key: &str| reg.as_ref().map(|r| prop(r, key)).unwrap_or(Value::Null);
    Value::Object(vec![
        ("agent".into(), Value::from(agent.to_owned())),
        ("detected".into(), Value::Bool(detected)),
        ("managed".into(), Value::Bool(reg.is_some())),
        ("installed_by_hand".into(), hand),
        ("version".into(), val("version")),
        ("follows".into(), Value::from(follows(agent).to_owned())),
        ("last_error".into(), val("last_error")),
    ])
}

/// Constructs the command result. The command-tree owner supplies the stdout wrapper.
pub fn command_result(
    rt: &Runtime,
    verb: Verb,
    agent: Option<&str>,
    dry_run: bool,
) -> Result<Value, CliError> {
    // TS `new Session(...)` comes first: its StateStore creates the state dir, also before a refusal.
    state_store(&rt.d);
    let selected = validate_request(&rt.d, verb, agent, dry_run)?;
    if verb == Verb::Status {
        return Ok(Value::Array(
            selected.iter().map(|a| managed_status(&rt.d, a)).collect(),
        ));
    }
    let stored_latest = get_string(&plugin(&rt.d), "latest");
    let raw_tag = if matches!(verb, Verb::Install | Verb::Update) && !dry_run {
        Some(latest_tag(&rt.d)?)
    } else {
        None
    };
    let agents = if verb == Verb::Install {
        selected
            .into_iter()
            .filter(|a| detect(&rt.d, a))
            .collect::<Vec<_>>()
    } else {
        selected
            .into_iter()
            .filter(|a| registration(&rt.d, a).is_some())
            .collect()
    };
    if verb == Verb::Install && agents.is_empty() {
        return Err(usage(format!(
            "No coding agent found. Supported agents: {}.",
            AGENTS.join(", ")
        )));
    }
    let tag = raw_tag
        .as_deref()
        .map(version_tag)
        .or(stored_latest.as_deref());
    let mut payload = None;
    let mut rows = Vec::new();
    for a in agents {
        if dry_run {
            rows.push(process_agent(
                &rt.d,
                a,
                verb,
                true,
                tag,
                raw_tag.as_deref(),
                &mut payload,
            )?);
        } else {
            let store = state_store(&rt.d);
            let result = store.with_lock_timeout(300_000, || {
                process_agent(&rt.d, a, verb, false, tag, raw_tag.as_deref(), &mut payload)
            });
            rows.push(match result {
                Ok(value) => value?,
                Err(_) => agent_row(
                    a,
                    "refused",
                    None,
                    Some(format!(
                        "{a}: another ai command is running. Nothing was changed."
                    )),
                    vec![],
                ),
            });
        }
    }
    if let Some((dir, _)) = payload {
        let _ = fs::remove_dir_all(dir);
    }
    Ok(Value::Array(rows))
}

fn agent_row(
    agent: &str,
    action: &str,
    version: Option<String>,
    message: Option<String>,
    ops: Vec<String>,
) -> Value {
    let mut fields = vec![
        ("agent".into(), Value::from(agent.to_owned())),
        ("action".into(), Value::from(action.to_owned())),
        (
            "version".into(),
            version.map(Value::from).unwrap_or(Value::Null),
        ),
        ("follows".into(), Value::from(follows(agent).to_owned())),
    ];
    if let Some(message) = message {
        fields.push(("message".into(), Value::from(message)));
    }
    if !ops.is_empty() {
        fields.push(("ops".into(), arr_str(&ops)));
    }
    Value::Object(fields)
}

fn process_agent(
    d: &Deps,
    agent: &str,
    verb: Verb,
    dry_run: bool,
    tag: Option<&str>,
    raw_tag: Option<&str>,
    payload: &mut Option<(PathBuf, PathBuf)>,
) -> Result<Value, CliError> {
    let current = registration(d, agent);
    let version = current.as_ref().and_then(|r| get_string(r, "version"));
    if verb == Verb::Install && current.is_none() {
        if matches!(agent, "claude" | "codex" | "copilot" | "gemini") {
            let found = is_good(d, agent, &native_list(agent))
                .and_then(|r| native_entries(agent, &r.stdout));
            let Some(found) = found else {
                return Ok(agent_row(
                    agent,
                    "refused",
                    None,
                    Some(format!(
                        "{agent}: the list of installed plugins could not be read. Nothing was changed."
                    )),
                    vec![],
                ));
            };
            if !found.is_empty() {
                return Ok(agent_row(
                    agent,
                    "installed_by_hand",
                    None,
                    Some(format!(
                        "{agent}: a plugin is already here that ai install did not install. Nothing was changed."
                    )),
                    vec![],
                ));
            }
            if dry_run {
                return Ok(planned_native(d, agent, verb, tag));
            }
            if agent != "gemini" {
                let add = vec![
                    "plugin".into(),
                    "marketplace".into(),
                    "add".into(),
                    REPO.into(),
                ];
                let added = d.host.exec(
                    agent,
                    &add,
                    super::deps::ExecOpts {
                        timeout_ms: 120_000,
                        ..Default::default()
                    },
                );
                if !added.is_ok_and(|r| r.code == 0 && !r.timed_out) {
                    let list_args = if agent == "copilot" {
                        vec!["plugin".into(), "marketplace".into(), "list".into()]
                    } else {
                        vec![
                            "plugin".into(),
                            "marketplace".into(),
                            "list".into(),
                            "--json".into(),
                        ]
                    };
                    let l = is_good(d, agent, &list_args);
                    let needle = if agent == "codex" {
                        "purchasely"
                    } else {
                        "Purchasely-AI-Plugin"
                    };
                    if !l.is_some_and(|r| r.stdout.contains(needle)) {
                        return Ok(failed_step(agent, "marketplace add", &add, 1));
                    }
                }
            }
            for args in agent_args(agent, Verb::Install) {
                if is_good(d, agent, &args).is_none() {
                    return Ok(failed_step(agent, "install", &args, 1));
                }
            }
            let probe = match identity(d, agent, "install") {
                Ok(v) => v,
                Err(e) => return Ok(agent_row(agent, "failed", None, Some(e), vec![])),
            };
            let mut fields = vec![
                ("method".into(), Value::from("native".to_owned())),
                (
                    "id".into(),
                    Value::from(
                        match agent {
                            "claude" | "copilot" => "purchasely@Purchasely-AI-Plugin",
                            "codex" => "purchasely@purchasely",
                            _ => "purchasely",
                        }
                        .to_owned(),
                    ),
                ),
            ];
            if let Some(scope) = get_string(&probe, "scope") {
                fields.push(("scope".into(), Value::from(scope)));
            }
            fields.push(("probe".into(), probe));
            fields.push((
                "version".into(),
                Value::from(tag.unwrap_or_default().to_owned()),
            ));
            fields.push(("updated_at".into(), Value::Number(d.host.now() as f64)));
            write_registration(d, agent, Some(Value::Object(fields)), tag)?;
            return Ok(agent_row(
                agent,
                "installed",
                tag.map(str::to_owned),
                None,
                vec![],
            ));
        }
        if dry_run {
            if targets(d, agent)
                .iter()
                .any(|p| p.exists() && marker(p).is_none())
            {
                return Ok(hand_row(agent));
            }
            return Ok(planned_copy(d, agent, verb, tag));
        }
        let (dir, src) = match payload {
            Some(p) => p.clone(),
            None => match release_payload(d, raw_tag.unwrap_or_default()) {
                Ok(p) => {
                    *payload = Some(p.clone());
                    p
                }
                Err(e) => {
                    return Ok(agent_row(
                        agent,
                        "failed",
                        None,
                        e.messages.first().cloned(),
                        vec![],
                    ));
                }
            },
        };
        let _ = dir;
        // TS `install`: the ownership marker of every destination is checked before any copy.
        if layout(d, agent, &src)
            .iter()
            .any(|(_, p)| p.exists() && marker(p).is_none())
        {
            return Ok(hand_row(agent));
        }
        let written = match copy_layout(d, agent, &src, tag.unwrap_or_default()) {
            Ok(v) => v,
            Err(e) => {
                return Ok(agent_row(
                    agent,
                    "failed",
                    None,
                    e.messages.first().cloned(),
                    vec![],
                ));
            }
        };
        write_registration(
            d,
            agent,
            Some(copy_record(&written, tag.unwrap_or_default(), d.host.now())),
            tag,
        )?;
        return Ok(agent_row(
            agent,
            "installed",
            tag.map(str::to_owned),
            None,
            vec![],
        ));
    }
    if verb == Verb::Install && current.is_some() {
        return process_agent(d, agent, Verb::Update, dry_run, tag, raw_tag, payload);
    }
    let Some(reg) = current else {
        return Ok(agent_row(agent, "not_detected", None, None, vec![]));
    };
    if matches!(agent, "gemini") && matches!(verb, Verb::Update | Verb::Uninstall) {
        let args = agent_args(agent, verb)[0].clone();
        return refuse(
            d,
            agent,
            &reg,
            &format!(
                "the gemini list cannot show who owns the extension. Run \"{}\" yourself",
                std::iter::once(agent.to_owned())
                    .chain(args)
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            false,
            dry_run,
        );
    }
    let own = owns(d, agent, &reg);
    if let Err((reason, drop)) = own {
        return refuse(d, agent, &reg, &reason, drop, dry_run);
    }
    if verb == Verb::Uninstall {
        if dry_run {
            return Ok(planned_agent(d, agent, verb, tag, Some(&reg)));
        }
        if matches!(agent, "claude" | "codex" | "copilot") {
            for args in agent_args(agent, verb) {
                if is_good(d, agent, &args).is_none() {
                    return failed_registered_step(d, agent, &reg, "uninstall", &args, 1);
                }
            }
        } else {
            for p in record_targets(d, &reg) {
                let _ = fs::remove_dir_all(p);
            }
        }
        write_registration(d, agent, None, None)?;
        return Ok(agent_row(agent, "removed", version, None, vec![]));
    }
    if matches!(agent, "cursor" | "opencode")
        && tag.is_some_and(|t| !newer(t, version.as_deref().unwrap_or("0.0.0")))
    {
        return Ok(agent_row(agent, "up_to_date", version, None, vec![]));
    }
    if dry_run {
        return Ok(planned_agent(d, agent, Verb::Update, tag, Some(&reg)));
    }
    if matches!(agent, "claude" | "codex" | "copilot") {
        for args in agent_args(agent, Verb::Update) {
            if is_good(d, agent, &args).is_none() {
                return failed_registered_step(d, agent, &reg, "update", &args, 1);
            }
        }
        let probe = match identity(d, agent, "update") {
            Ok(v) => v,
            Err(e) => return failed_registered(d, agent, &reg, e),
        };
        let mut fields = if let Value::Object(xs) = reg.clone() {
            xs
        } else {
            vec![]
        };
        // TS `{ ...reg, scope: probe.scope, probe, version, updated_at, last_error: undefined }`.
        fields.retain(|(k, _)| k != "last_error");
        match get_string(&probe, "scope") {
            Some(scope) => set(&mut fields, "scope", Value::from(scope)),
            None => fields.retain(|(k, _)| k != "scope"),
        }
        set(&mut fields, "probe", probe);
        set(
            &mut fields,
            "version",
            Value::from(tag.unwrap_or_default().to_owned()),
        );
        set(
            &mut fields,
            "updated_at",
            Value::Number(d.host.now() as f64),
        );
        write_registration(d, agent, Some(Value::Object(fields)), tag)?;
    } else {
        let (dir, src) = match payload {
            Some(p) => p.clone(),
            None => match release_payload(d, raw_tag.unwrap_or_default()) {
                Ok(p) => {
                    *payload = Some(p.clone());
                    p
                }
                Err(e) => {
                    return failed_registered(
                        d,
                        agent,
                        &reg,
                        e.messages.first().cloned().unwrap_or_default(),
                    );
                }
            },
        };
        let _ = dir;
        let written = match copy_layout(d, agent, &src, tag.unwrap_or_default()) {
            Ok(v) => v,
            Err(e) => {
                return failed_registered(
                    d,
                    agent,
                    &reg,
                    e.messages.first().cloned().unwrap_or_default(),
                );
            }
        };
        for old in record_targets(d, &reg) {
            if !written.contains(&old) {
                let _ = fs::remove_dir_all(old);
            }
        }
        write_registration(
            d,
            agent,
            Some(copy_record(&written, tag.unwrap_or_default(), d.host.now())),
            tag,
        )?;
    }
    Ok(agent_row(
        agent,
        "updated",
        tag.map(str::to_owned),
        None,
        vec![],
    ))
}

fn failed_step(agent: &str, step: &str, args: &[String], code: i32) -> Value {
    agent_row(
        agent,
        "failed",
        None,
        Some(format!(
            "{agent}: the plugin {step} failed (exit {code}). Run \"{}\" yourself to see why.",
            std::iter::once(agent.to_owned())
                .chain(args.iter().cloned())
                .collect::<Vec<_>>()
                .join(" ")
        )),
        vec![],
    )
}
fn failed_registered(
    d: &Deps,
    agent: &str,
    reg: &Value,
    message: String,
) -> Result<Value, CliError> {
    let mut fields = if let Value::Object(fields) = reg.clone() {
        fields
    } else {
        vec![]
    };
    fields.retain(|(key, _)| key != "last_error");
    fields.push(("last_error".into(), Value::from(message.clone())));
    write_registration(d, agent, Some(Value::Object(fields)), None)?;
    Ok(agent_row(
        agent,
        "failed",
        get_string(reg, "version"),
        Some(message),
        vec![],
    ))
}
fn failed_registered_step(
    d: &Deps,
    agent: &str,
    reg: &Value,
    step: &str,
    args: &[String],
    code: i32,
) -> Result<Value, CliError> {
    let failed = failed_step(agent, step, args, code);
    failed_registered(
        d,
        agent,
        reg,
        get_string(&failed, "message").unwrap_or_default(),
    )
}
fn hand_row(agent: &str) -> Value {
    agent_row(
        agent,
        "installed_by_hand",
        None,
        Some(format!(
            "{agent}: a plugin is already here that ai install did not install. Nothing was changed."
        )),
        vec![],
    )
}
fn planned_native(d: &Deps, a: &str, v: Verb, tag: Option<&str>) -> Value {
    planned_agent(d, a, v, tag, None)
}
fn planned_copy(d: &Deps, a: &str, _v: Verb, tag: Option<&str>) -> Value {
    let targets = targets(d, a);
    let mut ops = vec![format!("fetch {TARBALL}{}", tag.unwrap_or("latest"))];
    ops.extend(
        targets
            .iter()
            .map(|p| format!("copy to {} and write {MARKER}", p.display())),
    );
    agent_row(a, "planned", tag.map(str::to_owned), None, ops)
}
fn planned_agent(d: &Deps, a: &str, v: Verb, tag: Option<&str>, reg: Option<&Value>) -> Value {
    let mut ops = vec![];
    if matches!(a, "claude" | "codex" | "copilot" | "gemini") {
        for args in agent_args(a, v) {
            ops.push(format!("{a} {}", args.join(" ")));
        }
        if v == Verb::Install && matches!(a, "claude" | "codex" | "copilot") {
            ops.insert(0, format!("{a} plugin marketplace add {REPO}"));
        }
    } else if v == Verb::Uninstall {
        ops.extend(
            reg.map(|r| record_targets(d, r))
                .unwrap_or_default()
                .iter()
                .map(|p| format!("delete {}", p.display())),
        );
    } else {
        ops.extend(
            targets(d, a)
                .iter()
                .map(|p| format!("copy to {} and write {MARKER}", p.display())),
        );
    }
    agent_row(
        a,
        "planned",
        reg.and_then(|r| get_string(r, "version"))
            .or_else(|| tag.map(str::to_owned)),
        None,
        ops,
    )
}
fn record_targets(d: &Deps, reg: &Value) -> Vec<PathBuf> {
    array(&prop(reg, "targets"))
        .iter()
        .filter_map(Value::as_str)
        .map(PathBuf::from)
        .collect::<Vec<_>>()
        .into_iter()
        .map(|p| {
            if p.is_absolute() {
                p
            } else {
                d.state_dir.join(p)
            }
        })
        .collect()
}
fn copy_record(paths: &[PathBuf], version: &str, now: i64) -> Value {
    Value::Object(vec![
        ("method".into(), Value::from("copy".to_owned())),
        (
            "targets".into(),
            Value::Array(
                paths
                    .iter()
                    .map(|p| Value::from(p.to_string_lossy().into_owned()))
                    .collect(),
            ),
        ),
        ("version".into(), Value::from(version.to_owned())),
        ("updated_at".into(), Value::Number(now as f64)),
    ])
}
fn owns(d: &Deps, agent: &str, reg: &Value) -> Result<(), (String, bool)> {
    if matches!(agent, "claude" | "codex" | "copilot" | "gemini") {
        let listed = is_good(d, agent, &native_list(agent))
            .and_then(|r| native_entries(agent, &r.stdout))
            .ok_or_else(|| {
                (
                    "the list of installed plugins could not be read".into(),
                    false,
                )
            })?;
        if listed.is_empty() {
            return Err(("the plugin is gone from the agent".into(), true));
        }
        if listed.len() > 1 {
            return Err(("the plugin is installed more than once (for example in the user and the project scope)".into(),false));
        }
        if listed[0] != prop(reg, "probe") {
            return Err(("the plugin was replaced since ai install".into(), false));
        }
        Ok(())
    } else {
        let paths = record_targets(d, reg);
        if !paths.is_empty() && paths.iter().all(|p| !p.exists()) {
            return Err(("the plugin is gone from the agent".into(), true));
        }
        if !paths.is_empty()
            && paths
                .iter()
                .all(|p| marker(p).as_deref() == get_string(reg, "version").as_deref())
        {
            Ok(())
        } else {
            Err(("the installed files changed since ai install".into(), false))
        }
    }
}

/// Validates the agent selection and network policy before any mutation.
pub fn validate_request(
    d: &Deps,
    verb: Verb,
    agent: Option<&str>,
    dry_run: bool,
) -> Result<Vec<&'static str>, CliError> {
    let selected: Vec<&'static str> = match agent {
        None | Some("all") => AGENTS.to_vec(),
        Some(a) if AGENTS.contains(&a) => {
            vec![AGENTS[AGENTS.iter().position(|x| *x == a).unwrap()]]
        }
        _ => {
            return Err(usage(format!(
                "--agent must be one of {} or all.",
                AGENTS.join(", ")
            )));
        }
    };
    if matches!(verb, Verb::Install | Verb::Update) && !dry_run && offline(d) {
        return Err(usage(
            "PURCHASELY_OFFLINE=1 is set: the ai commands need the network.",
        ));
    }
    Ok(selected)
}

/// Update plugins that have a newer release. Native agent updates do not need the tarball.
pub fn update_plugins(rt: &Runtime, background: bool) -> Result<(), CliError> {
    if offline(&rt.d)
        || !matches!(agent_map(&plugin(&rt.d)), Value::Object(ref values) if !values.is_empty())
    {
        return Ok(());
    }
    let store = state_store(&rt.d);
    let timeout = if background { -1 } else { 300_000 };
    if background {
        let reserved = store.with_lock_timeout(timeout, || {
            let current = plugin(&rt.d);
            let checked = match prop(&current, "checked_at") {
                Value::Number(value) => value as i64,
                _ => 0,
            };
            if rt.d.host.now() - checked < 24 * 60 * 60 * 1000 {
                return false;
            }
            let mut fields = if let Value::Object(fields) = current {
                fields
            } else {
                vec![]
            };
            set(
                &mut fields,
                "checked_at",
                Value::Number(rt.d.host.now() as f64),
            );
            store.write("plugin", &Value::Object(fields)).is_ok()
        });
        if !reserved.unwrap_or(false) {
            return Ok(());
        }
    }
    let Ok(raw_tag) = latest_tag(&rt.d) else {
        return Ok(());
    };
    let tag = version_tag(&raw_tag);
    if !store
        .with_lock_timeout(timeout, || {
            let current = plugin(&rt.d);
            let mut fields = if let Value::Object(fields) = current {
                fields
            } else {
                vec![]
            };
            set(&mut fields, "latest", Value::from(tag.to_owned()));
            set(
                &mut fields,
                "checked_at",
                Value::Number(rt.d.host.now() as f64),
            );
            store.write("plugin", &Value::Object(fields)).is_ok()
        })
        .unwrap_or(false)
    {
        return Ok(());
    }
    let mut payload = None;
    for agent in AGENTS {
        let Some(record) = registration(&rt.d, agent) else {
            continue;
        };
        let Some(old_version) = get_string(&record, "version") else {
            continue;
        };
        if !newer(tag, &old_version) || !detect(&rt.d, agent) {
            continue;
        }
        let result = store.with_lock_timeout(timeout, || {
            process_agent(
                &rt.d,
                agent,
                Verb::Update,
                false,
                Some(tag),
                Some(&raw_tag),
                &mut payload,
            )
        });
        // TS: a failed state write throws out of `updatePlugins`.
        let row = match result {
            Ok(Ok(row)) => Some(row),
            Ok(Err(e)) => {
                if let Some((dir, _)) = payload {
                    let _ = fs::remove_dir_all(dir);
                }
                return Err(e);
            }
            Err(_) => None,
        };
        if let Some(row) = row
            && !background
        {
            let action = get_string(&row, "action").unwrap_or_default();
            if action == "updated" {
                rt.d.host
                    .stderr(&format!("{agent}: plugin updated to {tag}\n"));
            } else if let Some(message) = get_string(&row, "message") {
                rt.d.host.stderr(&format!("{message}\n"));
            }
        }
    }
    if let Some((dir, _)) = payload {
        let _ = fs::remove_dir_all(dir);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::deps::*;
    use std::{collections::BTreeMap, process::Command};
    /// GitHub and the agents, faked: the release is v2.3.0, its tarball is `tgz`, only opencode is detected.
    struct H {
        tgz: Vec<u8>,
    }
    impl Host for H {
        fn fetch(&self, r: Request) -> Result<Response, NetError> {
            let body = if r.url == API {
                b"{\"tag_name\":\"v2.3.0\"}".to_vec()
            } else {
                self.tgz.clone()
            };
            Ok(Response {
                status: 200,
                body,
                ..Response::default()
            })
        }
        fn exec(&self, file: &str, args: &[String], _: ExecOpts) -> Result<ExecResult, ExecError> {
            let code = match file {
                "tar" => Command::new("tar")
                    .args(args)
                    .status()
                    .map_or(1, |s| s.code().unwrap_or(1)),
                "opencode" => 0,
                _ => 1,
            };
            Ok(ExecResult {
                code,
                ..ExecResult::default()
            })
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
    /// A sandbox HOME, state and cache, and a two-skill plugin release.
    fn sandbox(name: &str) -> (PathBuf, Runtime) {
        let root = std::env::temp_dir().join(format!("ai-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let pkg = root.join("pkg/Purchasely-AI-Plugin-2.3.0/purchasely");
        for dir in [
            "skills/purchasely-integrate",
            "skills/purchasely-extra",
            "references",
        ] {
            fs::create_dir_all(pkg.join(dir)).unwrap();
            fs::write(pkg.join(dir).join("file.md"), "release").unwrap();
        }
        let tgz = root.join("plugin.tgz");
        let packed = Command::new("tar")
            .arg("-czf")
            .arg(&tgz)
            .arg("-C")
            .arg(root.join("pkg"))
            .arg("Purchasely-AI-Plugin-2.3.0")
            .status()
            .unwrap();
        assert!(packed.success());
        let home = root.join("home");
        fs::create_dir_all(&home).unwrap();
        let mut env = BTreeMap::new();
        env.insert("HOME".into(), home.to_string_lossy().into_owned());
        let rt = Runtime::new(Deps {
            host: Box::new(H {
                tgz: fs::read(&tgz).unwrap(),
            }),
            env,
            is_tty: false,
            config_dir: root.join("config"),
            state_dir: root.join("state"),
            cache_dir: root.join("cache"),
            version: "9.9.9".into(),
            pid: std::process::id(),
            name: "purchasely".into(),
            host_url: String::new(),
            ext: None,
            self_exe: PathBuf::new(),
            platform: "darwin",
            stdout_tty: false,
            columns: None,
        });
        (root, rt)
    }
    /// TS `install` checks the marker of every destination of `layout(d, a, src)` before any copy.
    #[test]
    fn opencode_install_keeps_an_unmarked_skill_of_the_payload() {
        let (root, rt) = sandbox("hand-skill");
        let mine = root.join("home/.config/opencode/skills/purchasely-extra");
        fs::create_dir_all(&mine).unwrap();
        fs::write(mine.join("my-content.md"), "mine").unwrap();
        let rows = command_result(&rt, Verb::Install, Some("opencode"), false).unwrap();
        let Value::Array(rows) = rows else { panic!() };
        assert_eq!(
            get_string(&rows[0], "action").as_deref(),
            Some("installed_by_hand")
        );
        assert_eq!(
            fs::read_to_string(mine.join("my-content.md")).unwrap(),
            "mine"
        );
        assert!(
            !root
                .join("home/.config/opencode/skills/purchasely-integrate")
                .exists()
        );
        assert!(registration(&rt.d, "opencode").is_none());
        let _ = fs::remove_dir_all(root);
    }
    /// TS `refused` changes the record only when `!s.dry`.
    #[test]
    fn dry_run_refusal_leaves_the_record() {
        let (root, rt) = sandbox("dry-refusal");
        fs::create_dir_all(root.join("home/.cursor")).unwrap();
        fs::create_dir_all(root.join("state")).unwrap();
        let state = root.join("state/ai.plugin.json");
        let gone = root.join("home/.cursor/plugins/local/purchasely");
        let text = format!(
            r#"{{"agents":{{"cursor":{{"method":"copy","targets":["{}"],"version":"2.2.0","updated_at":1}},"opencode":{{"method":"copy","targets":["{}"],"version":"2.2.0","updated_at":1}}}}}}"#,
            gone.display(),
            root.join("state").display()
        );
        fs::write(&state, &text).unwrap();
        for agent in ["cursor", "opencode"] {
            let rows = command_result(&rt, Verb::Update, Some(agent), true).unwrap();
            let Value::Array(rows) = rows else { panic!() };
            assert_eq!(get_string(&rows[0], "action").as_deref(), Some("refused"));
        }
        assert_eq!(fs::read_to_string(&state).unwrap(), text);
        let _ = fs::remove_dir_all(root);
    }
    /// TS `Session.put` lets the state write throw: no "installed" without its record.
    #[test]
    fn failed_registration_write_fails_the_install() {
        let (root, rt) = sandbox("record-write");
        fs::create_dir_all(root.join("state/ai.plugin.json")).unwrap();
        let e = command_result(&rt, Verb::Install, Some("opencode"), false).unwrap_err();
        assert_eq!((e.exit, e.code.as_str()), (EXIT::GENERIC, "INTERNAL_ERROR"));
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn agent_table_has_six_agents() {
        assert_eq!(AGENTS.len(), 6);
    }
    #[test]
    fn tag_validation_rejects_path_input() {
        assert!(valid_tag("v2.3.0"));
        assert!(!valid_tag("../latest"));
        assert!(!valid_tag("v2.3"));
    }
    #[test]
    fn native_agents_follow_branch() {
        assert_eq!(follows("claude"), "default branch");
        assert_eq!(follows("cursor"), "release tag");
    }
}
