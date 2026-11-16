//! Owner: P5a
use super::{
    config::{self, Config},
    credentials::{self, Credential},
    deps::Deps,
    errors::{self, usage},
};
use crate::cli::{Command, OptionDef};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

pub struct Runtime {
    pub d: Deps,
    pub cfg: Config,
    pub taken: BTreeSet<String>,
    pub command: Option<String>,
    /// One resolution per `env:surface` and run (TS `Runtime.creds`), errors included.
    creds: RefCell<BTreeMap<String, Result<Option<Credential>, String>>>,
}

impl Runtime {
    pub fn new(d: Deps) -> Self {
        let raw = config::read_config(&d.config_dir);
        let mut cfg = Config::default();
        if let super::json::Value::Object(values) = raw {
            for (key, value) in values {
                match key.as_str() {
                    "identity_email" => cfg.identity_email = value.as_str(),
                    "rate" => {
                        if let super::json::Value::Object(rates) = value {
                            cfg.rate = Some(
                                rates
                                    .into_iter()
                                    .filter_map(|(key, value)| match value {
                                        super::json::Value::Number(rate) => Some((key, rate)),
                                        _ => None,
                                    })
                                    .collect(),
                            );
                        }
                    }
                    "auto_update" => {
                        if let super::json::Value::Bool(v) = value {
                            cfg.auto_update = Some(v);
                        }
                    }
                    "read_only" => {
                        if let super::json::Value::Bool(v) = value {
                            cfg.read_only = Some(v);
                        }
                    }
                    "write_allowlist" => cfg.write_allowlist = Some(value),
                    _ => {}
                }
            }
        }
        Self {
            d,
            cfg,
            taken: BTreeSet::new(),
            command: None,
            creds: RefCell::default(),
        }
    }

    pub fn env(&self, option: Option<&str>) -> Result<String, crate::core::errors::CliError> {
        let value = option
            .or_else(|| self.d.env.get("PURCHASELY_ENV").map(String::as_str))
            .unwrap_or("prod");
        if value != "prod" && value != "staging" {
            return Err(usage(format!(
                "--env must be prod or staging, got \"{value}\"."
            )));
        }
        Ok(value.to_owned())
    }

    pub fn surface(&self, top_level: &str, extension_command: bool) -> String {
        if extension_command {
            self.d
                .ext
                .as_ref()
                .and_then(|extension| {
                    extension
                        .surfaces
                        .iter()
                        .find(|(name, _)| *name == top_level)
                        .map(|(_, surface)| *surface)
                })
                .unwrap_or_else(|| self.d.ext.as_ref().map_or("management", |ext| ext.surface))
                .to_owned()
        } else {
            "management".to_owned()
        }
    }

    pub fn credential(&self, env: &str, surface: &str) -> Result<Option<Credential>, String> {
        // Demo mode uses no key, even when one is set: nothing reads the keychain.
        if self.d.env.get("PURCHASELY_DEMO").is_some_and(|v| v == "1") {
            return Ok(None);
        }
        self.creds
            .borrow_mut()
            .entry(format!("{env}:{surface}"))
            .or_insert_with(|| credentials::resolve_credential(&self.d, env, surface))
            .clone()
    }

    pub fn read_only(&self) -> bool {
        self.d
            .env
            .get("PURCHASELY_READ_ONLY")
            .is_some_and(|value| value == "1")
            || self.cfg.read_only == Some(true)
    }

    pub fn rate(
        &self,
        surface: &str,
        requested: Option<&str>,
        allow_above_cap: bool,
    ) -> Result<f64, errors::CliError> {
        let definition = credentials::surface_def(surface).map_err(usage)?;
        let configured = self
            .cfg
            .rate
            .as_ref()
            .and_then(|rates| {
                rates
                    .iter()
                    .find(|(name, _)| name == surface)
                    .map(|(_, rate)| *rate)
            })
            .unwrap_or(definition.rate.default);
        let rate = match requested {
            Some(value) => value
                .parse::<f64>()
                .map_err(|_| usage("--rate must be a number above 0."))?,
            None => configured,
        };
        if rate.is_nan() || rate <= 0.0 {
            return Err(usage("--rate must be a number above 0."));
        }
        if rate > definition.rate.cap && !allow_above_cap {
            return Err(usage(format!(
                "The rate {rate} req/s is above the cap of {} req/s for {surface}. Lower it, or ask the backend team and pass --i-asked-backend.",
                definition.rate.cap
            )));
        }
        Ok(rate)
    }
}

pub fn group<'a>(program: &'a mut Command, name: &str) -> &'a mut Command {
    if let Some(index) = program.commands.iter().position(|c| c.name == name) {
        return &mut program.commands[index];
    }
    let mut command = Command::new(name).description(format!("Management API: {name}"));
    if name == "applications" {
        command = command.alias("apps");
    }
    program.commands.push(command);
    program.commands.last_mut().expect("command was inserted")
}

pub fn add_globals(command: &mut Command) {
    command.options.extend([
        OptionDef::value("--output <format>", "json (default) or table"),
        OptionDef::value("--env <env>", "prod or staging. Mandatory for writes"),
        OptionDef::boolean("--dry-run", "print the request without sending it"),
        OptionDef::boolean("--yes", "accept the confirmation prompt of a risky change"),
        OptionDef::value(
            "--reason <text>",
            "why you run this command (sent in the User-Agent)",
        ),
        OptionDef::value("--rate <n>", "requests per second, lower than the default"),
        OptionDef::boolean("--i-asked-backend", "allow a rate above the hard cap"),
        OptionDef::value(
            "--out <file>",
            "write the JSON output to a file, or - for stdout",
        ),
        OptionDef::value("--app <id>", "application id"),
    ]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::deps::*;
    use std::{cell::Cell, collections::BTreeMap, path::PathBuf};

    struct CountingKeychain(Cell<u32>);
    impl Keychain for CountingKeychain {
        fn get(&self, _: &str, _: &str) -> Result<Option<String>, KeychainError> {
            self.0.set(self.0.get() + 1);
            Ok(Some(format!("key{}", self.0.get())))
        }
        fn set(&self, _: &str, _: &str, _: &str) -> Result<(), KeychainError> {
            Ok(())
        }
        fn delete(&self, _: &str, _: &str) -> Result<(), KeychainError> {
            Ok(())
        }
    }
    struct H(CountingKeychain);
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
            Some(&self.0)
        }
        fn io(&self) -> Option<&dyn Io> {
            None
        }
    }

    fn runtime(env: &[(&str, &str)]) -> Runtime {
        Runtime::new(Deps {
            host: Box::new(H(CountingKeychain(Cell::new(0)))),
            env: env
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect::<BTreeMap<_, _>>(),
            is_tty: false,
            config_dir: PathBuf::from("/nonexistent"),
            state_dir: PathBuf::from("/nonexistent"),
            cache_dir: PathBuf::from("/nonexistent"),
            version: "1.0.0".into(),
            pid: 1,
            name: "purchasely".into(),
            host_url: String::new(),
            ext: None,
            self_exe: PathBuf::new(),
            platform: "linux",
            stdout_tty: false,
            columns: None,
        })
    }

    #[test]
    fn credential_is_resolved_once_per_env_and_surface() {
        let rt = runtime(&[]);
        let key = |env| rt.credential(env, "management").unwrap().unwrap().key;
        assert_eq!(key("prod"), "key1");
        assert_eq!(key("prod"), "key1");
        assert_eq!(key("staging"), "key2");
    }

    #[test]
    fn demo_mode_reads_no_credential() {
        let rt = runtime(&[("PURCHASELY_DEMO", "1"), ("PURCHASELY_API_KEY", "pk_x")]);
        assert_eq!(rt.credential("prod", "management"), Ok(None));
    }
}
