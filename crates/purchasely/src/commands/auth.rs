//! Owner: P5b
use crate::{
    cli::{Command, OptionDef, Parsed},
    core::{
        config,
        credentials::{credential_display, hint, remove_credential, store_credential, what},
        errors::{CliError, EXIT, usage},
        json::Value,
        runtime::{Runtime, add_globals},
    },
};

fn json_output(rt: &Runtime, value: Value) {
    rt.d.host.stdout(&format!(
        "{}\n",
        crate::core::json::stringify_pretty(&value).unwrap_or_else(|_| "{}".into())
    ));
}

fn login(p: &Parsed, rt: &mut Runtime) -> Result<(), CliError> {
    let env = rt.env(p.opt("env"))?;
    let identity =
        rt.d.ext
            .as_ref()
            .is_some_and(|extension| extension.identity);
    let key = p
        .opt("api-key")
        .map(str::to_owned)
        .unwrap_or_else(|| {
            if rt.d.is_tty {
                rt.d.host.prompt("Management API key: ", true)
            } else {
                rt.d.host.read_stdin()
            }
        })
        .trim()
        .to_owned();
    if key.is_empty() {
        return Err(usage(format!(
            "No {}. Pass --api-key, type it at the prompt, or pipe it on stdin.",
            what("management").unwrap_or_else(|_| "Management API key".into())
        )));
    }
    let email = if identity {
        let supplied = p.opt("email").map(str::to_owned).unwrap_or_else(|| {
            if rt.d.is_tty {
                rt.d.host.prompt(
                    &format!(
                        "Your email [{}]: ",
                        rt.cfg.identity_email.as_deref().unwrap_or("")
                    ),
                    false,
                )
            } else {
                String::new()
            }
        });
        let email = if supplied.trim().is_empty() {
            rt.cfg.identity_email.clone().unwrap_or_default()
        } else {
            supplied.trim().to_owned()
        };
        if !email.is_empty() && !email.contains('@') {
            return Err(usage("The email is not valid."));
        }
        email
    } else {
        String::new()
    };
    let source = store_credential(&rt.d, &env, "management", &key).map_err(|message| {
        CliError::new(EXIT::GENERIC, "CREDENTIAL_STORE_FAILED", vec![message])
    })?;
    if identity {
        rt.cfg.identity_email = (!email.is_empty()).then_some(email.clone());
        let mut value = match config::read_config(&rt.d.config_dir) {
            Value::Object(entries) => entries,
            _ => vec![],
        };
        value.retain(|(name, _)| name != "identity_email");
        if let Some(address) = &rt.cfg.identity_email {
            value.push(("identity_email".into(), Value::from(address.clone())));
        }
        config::write_config(&rt.d.config_dir, &Value::Object(value)).map_err(|error| {
            CliError::new(
                EXIT::GENERIC,
                "CONFIG_WRITE_FAILED",
                vec![error.to_string()],
            )
        })?;
    }
    json_output(
        rt,
        Value::Object(vec![
            ("stored".into(), Value::Bool(true)),
            ("env".into(), Value::from(env)),
            ("surface".into(), Value::from("management".to_owned())),
            ("source".into(), Value::from(source.to_owned())),
            (
                "key".into(),
                Value::from(credential_display("management", Some(&key))),
            ),
            (
                "identity".into(),
                if email.is_empty() {
                    Value::Null
                } else {
                    Value::from(email)
                },
            ),
        ]),
    );
    Ok(())
}

fn status(p: &Parsed, rt: &mut Runtime) -> Result<(), CliError> {
    let env = rt.env(p.opt("env"))?;
    let credential = rt
        .credential(&env, "management")
        .map_err(|message| CliError::new(EXIT::GENERIC, "CREDENTIAL_READ_FAILED", vec![message]))?;
    let Some(credential) = credential else {
        return Err(CliError::new(
            EXIT::AUTH,
            "UNAUTHORIZED",
            vec![format!(
                "No {}. {}",
                what("management").unwrap_or_default(),
                hint("management", &rt.d.name).unwrap_or_default()
            )],
        ));
    };
    let identity = if rt
        .d
        .ext
        .as_ref()
        .is_some_and(|extension| extension.identity)
    {
        rt.cfg
            .identity_email
            .clone()
            .map(Value::from)
            .unwrap_or(Value::Null)
    } else {
        Value::Null
    };
    json_output(
        rt,
        Value::Object(vec![
            ("env".into(), Value::from(env)),
            ("surface".into(), Value::from("management".to_owned())),
            ("source".into(), Value::from(credential.source.to_owned())),
            (
                "key".into(),
                Value::from(credential_display("management", Some(&credential.key))),
            ),
            ("identity".into(), identity),
        ]),
    );
    Ok(())
}

fn logout(p: &Parsed, rt: &mut Runtime) -> Result<(), CliError> {
    let env = rt.env(p.opt("env"))?;
    remove_credential(&rt.d, &env, "management").map_err(|message| {
        CliError::new(EXIT::GENERIC, "CREDENTIAL_REMOVE_FAILED", vec![message])
    })?;
    json_output(
        rt,
        Value::Object(vec![
            ("removed".into(), Value::Bool(true)),
            ("env".into(), Value::from(env)),
            ("surface".into(), Value::from("management".to_owned())),
        ]),
    );
    Ok(())
}

pub fn register(program: &mut Command, rt: &mut Runtime) {
    let mut auth = Command::new("auth").description("Manage the Management API key");
    for name in ["login", "status", "logout"] {
        rt.taken.insert(format!("auth {name}"));
    }
    let mut login_command = Command::new("login").description("Store the Management API key");
    add_globals(&mut login_command);
    login_command.options.push(OptionDef::value(
        "--api-key <key>",
        "the key (visible in the shell history: prefer the prompt or stdin)",
    ));
    if rt
        .d
        .ext
        .as_ref()
        .is_some_and(|extension| extension.identity)
    {
        login_command.options.push(OptionDef::value(
            "--email <email>",
            "your email, sent in the User-Agent",
        ));
    }
    login_command = login_command.after_help("\nThe key comes from the hidden prompt, from --api-key or from stdin. It is not checked against the API: no read endpoint works for every Management scope.\n");
    login_command = login_command.action(login);
    let mut status_command =
        Command::new("status").description("Show where the credential comes from");
    add_globals(&mut status_command);
    status_command = status_command.after_help("\nPrints the source of the credential (env, keychain or file), its last 4 characters and the identity. Exit 3 when there is no key.\n");
    status_command = status_command.action(status);
    let mut logout_command = Command::new("logout").description("Delete the stored credential");
    add_globals(&mut logout_command);
    logout_command = logout_command.after_help("\nDeletes the key stored in the keychain or in the file. A key in the environment is not touched.\n");
    logout_command = logout_command.action(logout);
    auth.commands
        .extend([login_command, status_command, logout_command]);
    program.commands.push(auth);
}
