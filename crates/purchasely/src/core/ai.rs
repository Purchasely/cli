//! Purchasely AI plugin helpers shared by the command and daily updater.
use super::{
    deps::{Deps, Request},
    errors::{CliError, EXIT, usage},
    json::{self, Value},
    runtime::Runtime,
};

pub const AGENTS: [&str; 6] = ["claude", "codex", "copilot", "gemini", "cursor", "opencode"];
const API: &str = "https://api.github.com/repos/Purchasely/Purchasely-AI-Plugin/releases/latest";
const REPO: &str = "Purchasely/Purchasely-AI-Plugin";

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
    super::state::StateStore::new(d, "ai").read(
        "plugin",
        Value::Object(vec![("agents".into(), Value::Object(vec![]))]),
    )
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
fn write_registration(d: &Deps, agent: &str, value: Option<Value>, latest: Option<&str>) {
    let old = plugin(d);
    let mut entries = if let Value::Object(xs) = agent_map(&old) {
        xs
    } else {
        vec![]
    };
    entries.retain(|(k, _)| k != agent);
    if let Some(v) = value {
        entries.push((agent.into(), v));
    }
    let mut root = if let Value::Object(xs) = old {
        xs
    } else {
        vec![]
    };
    root.retain(|(k, _)| k != "agents" && (latest.is_none() || k != "latest"));
    root.push(("agents".into(), Value::Object(entries)));
    if let Some(tag) = latest {
        root.push(("latest".into(), Value::from(tag.to_owned())));
    }
    let _ = super::state::StateStore::new(d, "ai").write("plugin", &Value::Object(root));
}
fn arr_str(v: &[String]) -> Value {
    Value::Array(v.iter().cloned().map(Value::from).collect())
}
fn version_tag(tag: &str) -> &str {
    tag.strip_prefix('v').unwrap_or(tag)
}
fn newer(a: &str, b: &str) -> bool {
    let nums = |s: &str| {
        version_tag(s)
            .split('.')
            .map(|n| n.parse::<u64>().unwrap_or(0))
            .collect::<Vec<_>>()
    };
    nums(a) > nums(b)
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
fn list(agent: &str) -> Vec<String> {
    match agent {
        "claude" | "copilot" => ["plugin", "list", "--json"].map(str::to_owned).to_vec(),
        "codex" => ["plugin", "list", "--json", "-m", "purchasely"]
            .map(str::to_owned)
            .to_vec(),
        "gemini" => ["extensions", "list"].map(str::to_owned).to_vec(),
        _ => vec![],
    }
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
fn managed_status(d: &Deps, agent: &str) -> Value {
    let reg = registration(d, agent);
    let detected = detect(d, agent);
    let hand = if reg.is_some() || !detected {
        Value::Bool(false)
    } else if agent == "opencode" {
        Value::from("unknown".to_owned())
    } else if agent == "cursor" {
        let p = d
            .env
            .get("HOME")
            .map(|h| std::path::Path::new(h).join(".cursor/plugins/local/purchasely"));
        Value::Bool(p.is_some_and(|p| p.exists() && !p.join(".purchasely-cli").exists()))
    } else {
        match is_good(d, agent, &list(agent)) {
            Some(r) => {
                let parsed = json::parse(&r.stdout).ok();
                Value::Bool(parsed.is_some_and(|v| !array(&v).is_empty()))
            }
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
    let selected = validate_request(&rt.d, verb, agent, dry_run)?;
    if verb == Verb::Status {
        return Ok(Value::Array(
            selected.iter().map(|a| managed_status(&rt.d, a)).collect(),
        ));
    }
    if matches!(verb, Verb::Install | Verb::Update) && !dry_run {
        let tag = latest_tag(&rt.d)?;
        if !valid_tag(&tag) {
            return Err(CliError::new(
                EXIT::GENERIC,
                "AI_RELEASE_UNREADABLE",
                vec!["could not read the latest plugin release. Check your network.".into()],
            ));
        }
    }
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
    let mut rows = Vec::new();
    for a in agents {
        if dry_run {
            rows.push(process_agent(&rt.d, a, verb, true));
        } else {
            let store = super::state::StateStore::new(&rt.d, "ai");
            let result = store.with_lock_timeout(300_000, || process_agent(&rt.d, a, verb, false));
            rows.push(match result {
                Ok(value) => value,
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
    Ok(Value::Array(rows))
}

fn agent_row(
    agent: &str,
    action: &str,
    version: Option<String>,
    message: Option<String>,
    ops: Vec<String>,
) -> Value {
    Value::Object(vec![
        ("agent".into(), Value::from(agent.to_owned())),
        ("action".into(), Value::from(action.to_owned())),
        (
            "version".into(),
            version.map(Value::from).unwrap_or(Value::Null),
        ),
        ("follows".into(), Value::from(follows(agent).to_owned())),
        (
            "message".into(),
            message.map(Value::from).unwrap_or(Value::Null),
        ),
        ("ops".into(), arr_str(&ops)),
    ])
}

fn process_agent(d: &Deps, agent: &str, verb: Verb, dry_run: bool) -> Value {
    if verb == Verb::Install && !dry_run && matches!(agent, "claude" | "codex" | "copilot") {
        let marketplace = ["plugin", "marketplace", "add", REPO];
        let args = marketplace
            .iter()
            .map(|x| (*x).to_owned())
            .collect::<Vec<_>>();
        if is_good(d, agent, &args).is_none() {
            return agent_row(
                agent,
                "failed",
                None,
                Some(format!(
                    "{agent}: the plugin marketplace add failed (exit 1). Run \"{}\" yourself to see why.",
                    marketplace.join(" ")
                )),
                vec![],
            );
        }
    }
    let steps = agent_args(agent, verb);
    if dry_run {
        let ops = steps
            .iter()
            .map(|x| format!("{agent} {}", x.join(" ")))
            .collect();
        return agent_row(agent, "planned", None, None, ops);
    }
    for args in &steps {
        if is_good(d, agent, args).is_none() {
            return agent_row(
                agent,
                "failed",
                None,
                Some(format!("{agent}: the plugin operation failed (exit 1).")),
                vec![],
            );
        }
    }
    agent_row(
        agent,
        match verb {
            Verb::Install => "installed",
            Verb::Update => "updated",
            Verb::Uninstall => "removed",
            Verb::Status => "status",
        },
        None,
        None,
        vec![],
    )
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
pub fn update_plugins(rt: &Runtime, background: bool) {
    if offline(&rt.d)
        || !matches!(agent_map(&plugin(&rt.d)), Value::Object(ref values) if !values.is_empty())
    {
        return;
    }
    let store = super::state::StateStore::new(&rt.d, "ai");
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
            fields.retain(|(key, _)| key != "checked_at");
            fields.push(("checked_at".into(), Value::Number(rt.d.host.now() as f64)));
            store.write("plugin", &Value::Object(fields)).is_ok()
        });
        if !reserved.unwrap_or(false) {
            return;
        }
    }
    let Ok(raw_tag) = latest_tag(&rt.d) else {
        return;
    };
    let tag = version_tag(&raw_tag);
    if store
        .with_lock_timeout(timeout, || {
            let current = plugin(&rt.d);
            let mut fields = if let Value::Object(fields) = current {
                fields
            } else {
                vec![]
            };
            fields.retain(|(key, _)| key != "latest" && key != "checked_at");
            fields.push(("latest".into(), Value::from(tag.to_owned())));
            fields.push(("checked_at".into(), Value::Number(rt.d.host.now() as f64)));
            store.write("plugin", &Value::Object(fields)).is_ok()
        })
        .unwrap_or(false)
        == false
    {
        return;
    }
    for agent in AGENTS {
        let Some(mut record) = registration(&rt.d, agent) else {
            continue;
        };
        let Some(old_version) = get_string(&record, "version") else {
            continue;
        };
        if !newer(tag, &old_version) || !detect(&rt.d, agent) {
            continue;
        }
        if !matches!(agent, "claude" | "codex" | "copilot" | "gemini") {
            continue;
        }
        let steps = agent_args(agent, Verb::Update);
        if steps
            .iter()
            .all(|args| is_good(&rt.d, agent, args).is_some())
        {
            if let Value::Object(ref mut fields) = record {
                fields.retain(|(key, _)| {
                    key != "version" && key != "updated_at" && key != "last_error"
                });
                fields.push(("version".into(), Value::from(tag.to_owned())));
                fields.push(("updated_at".into(), Value::Number(rt.d.host.now() as f64)));
            }
            write_registration(&rt.d, agent, Some(record), Some(tag));
            if !background {
                rt.d.host
                    .stderr(&format!("{agent}: plugin updated to {tag}\n"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
