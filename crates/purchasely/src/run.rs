//! Owner: P5a
use crate::{
    Program, Register,
    cli::{self, Command, Parsed},
    commands,
    core::{
        credentials::credential_display,
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

/// The built-in modules, in the order of `MODULES` in `run.ts`. A hand-written command registers before the
/// Management commands and takes a generated name.
const MODULES: [Register; 14] = [
    commands::auth::register,
    commands::usage::register,
    commands::search::register,
    commands::reference::register,
    commands::ai::register,
    commands::update::register,
    commands::games::register,
    commands::completion::register,
    commands::api::register,
    commands::apply::register,
    commands::doctor::register,
    commands::dashboard::register,
    commands::demo::register,
    crate::management::register::register,
];

/// Tops that need no key, no update check and no status line (`run.ts` preAction).
const OFFLINE: [&str; 6] = ["games", "play", "completion", "reference", "demo", "grep"];

/// The root with the built-in modules, then the modules of the extension. Returns the root and the number of
/// built-in top-level commands: the ones after it come from the extension and use its surface.
pub fn build(rt: &mut Runtime) -> (Command, usize) {
    let mut root = root_command(&rt.d.name, &rt.d.version);
    // TS defineSurface() of an extension: its surfaces exist before any command runs.
    if let Some(ext) = &rt.d.ext {
        for (id, def) in &ext.surface_defs {
            crate::core::credentials::define_surface(*id, *def);
        }
    }
    for register in MODULES {
        register(&mut root, rt);
    }
    let builtin = root.commands.len();
    let ext =
        rt.d.ext
            .as_ref()
            .map(|ext| ext.modules.clone())
            .unwrap_or_default();
    for register in ext {
        register(&mut root, rt);
    }
    (root, builtin)
}

fn root_command(name: &str, version: &str) -> Command {
    let mut root = Command::new(name)
        .description("Purchasely command line tool")
        .version(version);
    add_globals(&mut root);
    root.after_help(format!("\nConventions:\n  JSON goes to stdout. An error goes to stderr as one JSON object.\n  A command is <resource> <action>.\n  --env is mandatory on every write.\n  --dry-run prints the request without sending it.\n\nExamples:\n  {name} reference --format man"))
}

fn fail(rt: &Runtime, error: &CliError) -> i32 {
    rt.d.host.stderr(&format!("{}\n", error.to_json()));
    error.exit
}

pub fn run(argv: Vec<String>, d: Deps) -> i32 {
    let mut rt = Runtime::new(d);
    let (root, builtin) = build(&mut rt);
    let width = if rt.d.is_tty {
        rt.d.columns.unwrap_or(80) as usize
    } else {
        80
    };
    let argv = rewrite(argv);
    let parsed = match cli::parse(&root, &argv) {
        Ok(parsed) => parsed,
        Err(error) if error.0.starts_with("HELP:") => {
            // The parse error names the leaf only: walk argv to the command that asked for help.
            let mut command = &root;
            for arg in &argv {
                if let Some(sub) = command
                    .commands
                    .iter()
                    .find(|c| c.name == *arg || c.aliases.contains(arg))
                {
                    command = sub;
                }
            }
            rt.d.host.stdout(&cli::help::help(command, width));
            return EXIT::OK;
        }
        Err(error) if error.0 == rt.d.version => {
            rt.d.host.stdout(&format!("{}\n", rt.d.version));
            return EXIT::OK;
        }
        Err(error) => {
            let message = error.0.strip_prefix("error: ").unwrap_or(&error.0);
            return fail(
                &rt,
                &CliError::new(EXIT::USAGE, "USAGE", vec![message.to_owned()]),
            );
        }
    };
    if !parsed.command.is_empty() {
        return match dispatch(&root, builtin, &parsed, &mut rt, width) {
            Ok(()) => EXIT::OK,
            Err(error) => fail(&rt, &error),
        };
    }
    if let Some(output) = parsed.opt("output")
        && output != "json"
        && output != "table"
    {
        return fail(
            &rt,
            &CliError::new(
                EXIT::USAGE,
                "USAGE",
                vec!["--output must be json or table.".into()],
            ),
        );
    }
    if let Some(env) = parsed.opt("env")
        && let Err(error) = rt.env(Some(env))
    {
        return fail(&rt, &error);
    }
    if parsed.arguments.is_empty() && parsed.options.is_empty() {
        rt.d.host.stdout(&cli::help::help(&root, width));
    } else if !parsed.arguments.is_empty() {
        return fail(
            &rt,
            &CliError::new(EXIT::USAGE, "USAGE", vec!["too many arguments".into()]),
        );
    }
    EXIT::OK
}

/// Runs the action of the parsed command after the preAction rules of `run.ts`.
fn dispatch(
    root: &Command,
    builtin: usize,
    parsed: &Parsed,
    rt: &mut Runtime,
    width: usize,
) -> Result<(), CliError> {
    let path = parsed.command.join(" ");
    let command = root.find(&parsed.command).ok_or_else(|| {
        CliError::new(
            EXIT::GENERIC,
            "INTERNAL_ERROR",
            vec![format!("no command {path}")],
        )
    })?;
    rt.command = Some(path.clone());
    let Some(action) = command.action.clone() else {
        // A group with no subcommand prints its help on stdout and exits 0.
        if !command.commands.is_empty() {
            rt.d.host.stdout(&cli::help::help(command, width));
            return Ok(());
        }
        return Err(CliError::new(
            EXIT::GENERIC,
            "NOT_IMPLEMENTED",
            vec![format!("{path} is not ported yet.")],
        ));
    };
    let top = parsed.command[0].as_str();
    if !OFFLINE.contains(&top) {
        let env = rt.env(parsed.opt("env"))?;
        if let Some(output) = parsed.opt("output")
            && output != "json"
            && output != "table"
        {
            return Err(CliError::new(
                EXIT::USAGE,
                "USAGE",
                vec!["--output must be json or table.".into()],
            ));
        }
        // shortcut: no autoUpdate call (run.ts) while core/update.rs is a stub; P5b adds it here.
        let from_ext = root
            .commands
            .iter()
            .position(|c| c.name == top)
            .is_some_and(|i| i >= builtin);
        let surface = rt.surface(top, from_ext);
        let credential = rt
            .credential(&env, &surface)
            .map_err(|m| CliError::new(EXIT::GENERIC, "INTERNAL_ERROR", vec![m]))?;
        let identity = match &rt.d.ext {
            Some(ext) if ext.identity => rt.cfg.identity_email.as_deref().unwrap_or("-"),
            _ => "-",
        };
        let demo = if rt.d.env.get("PURCHASELY_DEMO").is_some_and(|v| v == "1") {
            " demo=on (sample data, not real)"
        } else {
            ""
        };
        rt.d.host.stderr(&format!(
            "{} env={env} surface={surface} identity={identity} key={}{demo}\n",
            rt.d.name,
            credential_display(&surface, credential.as_ref().map(|c| c.key.as_str()))
        ));
    }
    (action.0)(parsed, rt)
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
    fn dispatch_runs_the_action_and_maps_its_error_to_stderr_json() {
        use crate::core::deps::*;
        use std::{cell::RefCell, rc::Rc};
        type Out = Rc<RefCell<(String, String)>>;
        struct H(Out);
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
            fn stdout(&self, s: &str) {
                self.0.borrow_mut().0.push_str(s);
            }
            fn stderr(&self, s: &str) {
                self.0.borrow_mut().1.push_str(s);
            }
            fn keychain(&self) -> Option<&dyn Keychain> {
                None
            }
            fn io(&self) -> Option<&dyn Io> {
                None
            }
        }
        fn boom(program: &mut Command, _: &mut Runtime) {
            program.commands.push(
                Command::new("boom")
                    .action(|p, rt| {
                        let line = format!(
                            "ran {} {:?}\n",
                            rt.command.as_deref().unwrap_or(""),
                            p.arguments
                        );
                        rt.d.host.stdout(&line);
                        Err(CliError::new(
                            EXIT::NOT_FOUND,
                            "NOT_FOUND",
                            vec!["nothing here".into()],
                        ))
                    })
                    .argument(cli::Argument::optional("[x]", "x")),
            );
        }
        let out: Out = Rc::default();
        let deps = Deps {
            host: Box::new(H(out.clone())),
            env: BTreeMap::from([("PURCHASELY_API_KEY".into(), "pk_mgmt_live_SECRET123".into())]),
            is_tty: false,
            config_dir: PathBuf::from("/nonexistent"),
            state_dir: PathBuf::from("/nonexistent"),
            cache_dir: PathBuf::from("/nonexistent"),
            version: "1.0.0".into(),
            pid: 1,
            name: "purchasely".into(),
            host_url: String::new(),
            ext: Some(crate::Extension {
                modules: vec![boom],
                surface: "management",
                surfaces: vec![],
                surface_defs: vec![],
                identity: false,
                section: ("", ""),
                latest: None,
                npm_update: None,
            }),
            self_exe: PathBuf::new(),
            platform: "linux",
            stdout_tty: false,
            columns: None,
        };
        let code = run(vec!["boom".into(), "a".into()], deps);
        let (stdout, stderr) = out.borrow().clone();
        assert_eq!(code, EXIT::NOT_FOUND);
        assert_eq!(stdout, "ran boom [\"a\"]\n");
        assert_eq!(
            stderr,
            "purchasely env=prod surface=management identity=- key=\u{2026}T123\n{\"error\":{\"code\":\"NOT_FOUND\",\"messages\":[\"nothing here\"]}}\n"
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
