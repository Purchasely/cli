//! Owner: P5a
use crate::{
    Program,
    cli::{self, Command},
    core::{
        deps::{Deps, Host},
        errors::{CliError, EXIT},
        real::RealHost,
        runtime::{Runtime, add_globals},
    },
};
use std::{
    collections::BTreeMap,
    io::{self, IsTerminal, Write},
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

pub struct RunResult {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

fn rewrite(argv: Vec<String>) -> Vec<String> {
    let args =
        if argv.first().is_some_and(|s| s == "help") && argv.get(1).is_some_and(|s| s == "--all") {
            let mut changed = vec!["reference".to_owned()];
            changed.extend_from_slice(&argv[2..]);
            changed
        } else {
            argv
        };
    args.into_iter()
        .map(|arg| {
            if arg == "--app-id" {
                "--app".into()
            } else if let Some(value) = arg.strip_prefix("--app-id=") {
                format!("--app={value}")
            } else {
                arg
            }
        })
        .collect()
}

pub fn build(program: Program) -> Command {
    root_command(program.name, program.version)
}

fn root_command(name: &str, version: &str) -> Command {
    let mut root = Command::new(name)
        .description("Purchasely command line tool")
        .version(version);
    add_globals(&mut root);
    root.after_help(format!("\nConventions:\n  JSON goes to stdout. An error goes to stderr as one JSON object.\n  A command is <resource> <action>.\n  --env is mandatory on every write.\n  --dry-run prints the request without sending it.\n\nExamples:\n  {name} reference --format man"))
}

pub fn run(argv: Vec<String>, d: Deps) -> i32 {
    let rt = Runtime::new(d);
    let root = root_command(&rt.d.name, &rt.d.version);
    let parsed = match cli::parse(&root, &rewrite(argv)) {
        Ok(parsed) => parsed,
        Err(error) if error.0.starts_with("HELP:") => {
            rt.d.host.stdout(&cli::help::help(
                &root,
                if rt.d.is_tty {
                    rt.d.columns.unwrap_or(80) as usize
                } else {
                    80
                },
            ));
            return EXIT::OK;
        }
        Err(error) if error.0 == rt.d.version => {
            rt.d.host.stdout(&format!("{}\n", rt.d.version));
            return EXIT::OK;
        }
        Err(error) => {
            let message = error.0.strip_prefix("error: ").unwrap_or(&error.0);
            rt.d.host
                .stderr(&CliError::new(EXIT::USAGE, "USAGE", vec![message.to_owned()]).to_json());
            rt.d.host.stderr("\n");
            return EXIT::USAGE;
        }
    };
    if let Some(output) = parsed.options.get("output").and_then(|v| v.first())
        && output != "json"
        && output != "table"
    {
        let error = CliError::new(
            EXIT::USAGE,
            "USAGE",
            vec!["--output must be json or table.".into()],
        );
        rt.d.host.stderr(&format!("{}\n", error.to_json()));
        return EXIT::USAGE;
    }
    if let Some(env) = parsed.options.get("env").and_then(|v| v.first())
        && let Err(error) = rt.env(Some(env))
    {
        rt.d.host.stderr(&format!("{}\n", error.to_json()));
        return error.exit;
    }
    if parsed.command.is_empty() && parsed.arguments.is_empty() && parsed.options.is_empty() {
        rt.d.host.stdout(&cli::help::help(
            &root,
            if rt.d.is_tty {
                rt.d.columns.unwrap_or(80) as usize
            } else {
                80
            },
        ));
    } else if !parsed.arguments.is_empty() {
        let error = CliError::new(EXIT::USAGE, "USAGE", vec!["too many arguments".into()]);
        rt.d.host.stderr(&format!("{}\n", error.to_json()));
        return EXIT::USAGE;
    }
    EXIT::OK
}

fn default_deps(program: Program) -> Deps {
    let env: BTreeMap<String, String> = std::env::vars().collect();
    let home = env
        .get("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let xdg = |name: &str, fallback: PathBuf| {
        env.get(name)
            .map(PathBuf::from)
            .unwrap_or(fallback)
            .join("purchasely")
    };
    let host: Box<dyn Host> = Box::new(RealHost::new(env.clone()));
    let columns = terminal_columns();
    let platform = if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    };
    Deps {
        host,
        env: env.clone(),
        is_tty: io::stdin().is_terminal() && io::stderr().is_terminal(),
        config_dir: xdg("XDG_CONFIG_HOME", home.join(".config")),
        state_dir: xdg("XDG_STATE_HOME", home.join(".local/state")),
        cache_dir: xdg("XDG_CACHE_HOME", home.join(".cache")),
        version: program.version.to_owned(),
        pid: std::process::id(),
        name: program.name.to_owned(),
        host_url: program.host.to_owned(),
        ext: program.ext,
        self_exe: std::env::current_exe().unwrap_or_default(),
        platform,
        stdout_tty: io::stdout().is_terminal(),
        columns,
    }
}

fn terminal_columns() -> Option<u16> {
    let mut size = std::mem::MaybeUninit::<libc::winsize>::zeroed();
    if unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, size.as_mut_ptr()) } != 0 {
        return None;
    }
    let columns = unsafe { size.assume_init().ws_col };
    (columns > 0).then_some(columns)
}

pub fn main(program: Program) {
    let deps = default_deps(program);
    let output = catch_run(|| run(std::env::args().skip(1).collect(), deps));
    if !output.stderr.is_empty() {
        let _ = io::stderr().write_all(output.stderr.as_bytes());
    }
    std::process::exit(output.code);
}

fn catch_run(f: impl FnOnce() -> i32) -> RunResult {
    static PANIC_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = PANIC_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let old_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    std::panic::set_hook(old_hook);
    match result {
        Ok(code) => RunResult {
            code,
            stdout: String::new(),
            stderr: String::new(),
        },
        Err(payload) => {
            let message = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                .unwrap_or_else(|| "unknown error".into());
            let error = CliError::new(EXIT::GENERIC, "INTERNAL_ERROR", vec![message]);
            RunResult {
                code: EXIT::GENERIC,
                stdout: String::new(),
                stderr: format!("{}\n", error.to_json()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{deps::ExecOpts, real::RealHost};

    #[test]
    fn plumbing_group_reuses_command_and_keeps_apps_alias() {
        let mut program = crate::cli::Command::new("purchasely");
        let first = crate::core::runtime::group(&mut program, "applications") as *const _;
        let second = crate::core::runtime::group(&mut program, "applications") as *const _;
        assert_eq!(first, second);
        assert_eq!(program.commands.len(), 1);
        assert_eq!(program.commands[0].aliases, ["apps"]);
    }

    #[test]
    fn plumbing_real_exec_handles_input_status_timeout_and_missing_binary() {
        let host = RealHost::default();
        let input = host
            .exec(
                "sh",
                &["-c".into(), "cat".into()],
                ExecOpts {
                    input: Some("hello".into()),
                    timeout_ms: 5_000,
                    ..ExecOpts::default()
                },
            )
            .unwrap();
        assert_eq!(
            (input.code, input.stdout.as_str(), input.timed_out),
            (0, "hello", false)
        );
        let status = host
            .exec("sh", &["-c".into(), "exit 3".into()], ExecOpts::default())
            .unwrap();
        assert_eq!((status.code, status.timed_out), (3, false));
        let timeout = host
            .exec(
                "sh",
                &["-c".into(), "sleep 5".into()],
                ExecOpts {
                    timeout_ms: 20,
                    ..ExecOpts::default()
                },
            )
            .unwrap();
        assert!(timeout.timed_out);
        assert_eq!(
            host.exec("no-such-binary-xyz", &[], ExecOpts::default()),
            Err(crate::core::deps::ExecError::NotFound)
        );
    }

    #[test]
    fn panic_maps_to_internal_error() {
        let result = catch_run(|| panic!("controlled panic"));
        assert_eq!(result.code, 1);
        assert_eq!(
            result.stderr,
            "{\"error\":{\"code\":\"INTERNAL_ERROR\",\"messages\":[\"controlled panic\"]}}\n"
        );
    }
}
