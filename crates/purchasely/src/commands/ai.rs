//! AI plugin commands.
use crate::{
    cli::Command,
    core::{
        ai::{Verb, command_result},
        errors::{CliError, EXIT},
        json::Value,
        output::{EmitOptions, emit},
        runtime::{Runtime, add_globals},
    },
};

const VERBS: [(&str, &str); 4] = [
    (
        "install",
        "Install the plugin in every coding agent found on this machine",
    ),
    (
        "update",
        "Update the plugin in the agents that ai install registered",
    ),
    (
        "status",
        "Show the agents, the plugin version and how each one follows releases",
    ),
    (
        "uninstall",
        "Remove the plugin from the agents that ai install registered",
    ),
];

const MORE: [(&str, &str); 4] = [
    (
        "install",
        "Agents: claude, codex, copilot, gemini, cursor, opencode (default all). With --dry-run it runs read-only probes and prints the plan. PURCHASELY_OFFLINE=1 blocks it.",
    ),
    (
        "status",
        "Per agent: detected, managed by ai install, installed by hand (true, false or unknown), plugin version, how it follows releases, last error. It changes nothing.",
    ),
    (
        "update",
        "A copy agent (Cursor, OpenCode) updates only when a newer release exists.",
    ),
    ("uninstall", "A marketplace is never removed."),
];

fn field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    if let Value::Object(fields) = value {
        fields
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    } else {
        None
    }
}
fn text_field(value: &Value, key: &str) -> Option<String> {
    field(value, key).and_then(Value::as_str)
}

/// Adds the four leaf commands and reserves their generated command names.
pub fn register(program: &mut Command, rt: &mut Runtime) {
    let mut ai =
        Command::new("ai").description("Install the Purchasely AI plugin in your coding agents");
    for (verb, description) in VERBS {
        rt.taken.insert(format!("ai {verb}"));
        let mut leaf = Command::new(verb).description(description);
        add_globals(&mut leaf);
        leaf.options.push(crate::cli::OptionDef::value(
            "--agent <name>",
            "claude, codex, copilot, gemini, cursor, opencode or all (default: all)",
        ));
        let detail = MORE
            .iter()
            .find(|(name, _)| *name == verb)
            .map(|(_, text)| *text)
            .unwrap_or("");
        let kind = match verb {
            "install" => Verb::Install,
            "update" => Verb::Update,
            "status" => Verb::Status,
            _ => Verb::Uninstall,
        };
        leaf = leaf
            .after_help(format!("\n{detail}\n"))
            .action(move |p, rt| {
                let env = rt.env(p.opt("env"))?;
                let rows = command_result(rt, kind, p.opt("agent"), p.flag("dry-run"))?;
                let failed = if let Value::Array(ref agents) = rows {
                    agents
                        .iter()
                        .filter(|r| {
                            matches!(
                                text_field(r, "action").as_deref(),
                                Some("failed" | "refused")
                            )
                        })
                        .filter_map(|r| text_field(r, "message"))
                        .collect::<Vec<_>>()
                } else {
                    vec![]
                };
                let installed = if let Value::Array(ref agents) = rows {
                    agents
                        .iter()
                        .filter(|r| {
                            text_field(r, "action").as_deref() == Some("installed")
                                || text_field(r, "action").as_deref() == Some("updated")
                        })
                        .map(|r| text_field(r, "agent").unwrap_or_default())
                        .collect::<Vec<_>>()
                } else {
                    vec![]
                };
                let value = Value::Object(vec![("agents".into(), rows)]);
                emit(
                    &rt.d,
                    &value,
                    &EmitOptions {
                        table: p.opt("output") == Some("table"),
                        out: p.opt("out").map(str::to_owned),
                        env,
                        name: format!("ai-{verb}"),
                        ..EmitOptions::default()
                    },
                );
                if !p.flag("dry-run") && installed.iter().any(|a| a == "claude") {
                    rt.d.host
                        .stderr("Restart Claude Code to load the new plugin.\n");
                }
                if !p.flag("dry-run") && installed.iter().any(|a| a == "cursor") {
                    rt.d.host.stderr("Reload the Cursor window.\n");
                }
                if !failed.is_empty() {
                    return Err(CliError::new(EXIT::GENERIC, "AI_FAILED", failed));
                }
                Ok(())
            });
        ai.commands.push(leaf);
    }
    program.commands.push(ai);
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    #[test]
    fn registers_four_ai_verbs() {
        let mut root = Command::new("purchasely");
        let mut rt = Runtime::new(test_deps());
        register(&mut root, &mut rt);
        let ai = root.commands.iter().find(|c| c.name == "ai").unwrap();
        assert_eq!(
            ai.commands
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
            ["install", "update", "status", "uninstall"]
        );
        assert_eq!(rt.taken.len(), 4);
    }
    pub(crate) fn test_deps() -> crate::core::deps::Deps {
        use crate::core::deps::*;
        use std::{collections::BTreeMap, path::PathBuf};
        struct H;
        impl Host for H {
            fn fetch(&self, _: Request) -> Result<Response, NetError> {
                Ok(Response::default())
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
                0
            }
            fn wall_now(&self) -> i64 {
                0
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
        Deps {
            host: Box::new(H),
            env: BTreeMap::new(),
            is_tty: false,
            config_dir: PathBuf::new(),
            state_dir: PathBuf::new(),
            cache_dir: PathBuf::new(),
            version: "test".into(),
            pid: 1,
            name: "purchasely".into(),
            host_url: String::new(),
            ext: None,
            self_exe: PathBuf::new(),
            platform: "linux",
            stdout_tty: false,
            columns: None,
        }
    }
}
