//! AI plugin commands.
use crate::{
    cli::Command,
    core::{
        ai::{Verb, command_result},
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
                Ok(())
            });
        ai.commands.push(leaf);
    }
    program.commands.push(ai);
}

#[cfg(test)]
mod tests {
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
    fn test_deps() -> crate::core::deps::Deps {
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
