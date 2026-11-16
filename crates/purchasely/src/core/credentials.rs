//! Credential precedence, surface definitions, hashing, and macOS Security CLI adapter.
use super::{
    config::{credentials_path, read_json, write_secret_json},
    deps::{Deps, Keychain, KeychainError},
    json::Value,
};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    io::Write,
    process::{Command, Stdio},
    sync::{OnceLock, RwLock},
};
pub type Surface = String;
pub type Source = &'static str;
#[derive(Clone, Copy, Debug)]
pub struct Rate {
    pub default: f64,
    pub cap: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct SurfaceDef {
    pub env_var: &'static str,
    pub what: &'static str,
    pub hint: fn(&str) -> String,
    pub rate: Rate,
    pub display: Option<fn(&str) -> String>,
}
fn management_hint(name: &str) -> String {
    format!("Run `{name} auth login` or set PURCHASELY_API_KEY.")
}
fn registry() -> &'static RwLock<HashMap<String, SurfaceDef>> {
    static R: OnceLock<RwLock<HashMap<String, SurfaceDef>>> = OnceLock::new();
    R.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert(
            "management".into(),
            SurfaceDef {
                env_var: "PURCHASELY_API_KEY",
                what: "Management API key",
                hint: management_hint,
                rate: Rate {
                    default: 0.8,
                    cap: 1.0,
                },
                display: None,
            },
        );
        RwLock::new(m)
    })
}
pub fn define_surface(id: impl Into<String>, def: SurfaceDef) {
    if let Ok(mut m) = registry().write() {
        m.insert(id.into(), def);
    }
}
pub fn surface_def(id: &str) -> Result<SurfaceDef, String> {
    registry()
        .read()
        .map_err(|_| "surface registry unavailable".to_string())?
        .get(id)
        .cloned()
        .ok_or_else(|| format!("unknown surface {id}"))
}
pub fn what(id: &str) -> Result<String, String> {
    Ok(surface_def(id)?.what.into())
}
pub fn hint(id: &str, name: &str) -> Result<String, String> {
    Ok((surface_def(id)?.hint)(name))
}
pub fn key_hash(key: &str) -> String {
    let h = Sha256::digest(key.as_bytes());
    h[..6].iter().map(|x| format!("{x:02x}")).collect()
}
pub fn redact(key: Option<&str>) -> String {
    key.map(|s| {
        format!(
            "…{}",
            s.chars()
                .rev()
                .take(4)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>()
        )
    })
    .unwrap_or_else(|| "-".into())
}
pub fn credential_display(surface: &str, key: Option<&str>) -> String {
    if let (Some(k), Ok(def)) = (key, surface_def(surface))
        && let Some(display) = def.display
    {
        return display(k);
    }
    redact(key)
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Credential {
    pub source: Source,
    pub key: String,
}
type Creds = HashMap<String, HashMap<String, String>>;
fn read_creds(path: std::path::PathBuf) -> Creds {
    let v = read_json(path, Value::Object(vec![]));
    let mut out = Creds::new();
    if let Value::Object(envs) = v {
        for (e, v) in envs {
            if let Value::Object(surfs) = v {
                for (s, v) in surfs {
                    if let Some(k) = v.as_str() {
                        out.entry(e.clone()).or_default().insert(s, k);
                    }
                }
            }
        }
    }
    out
}
fn creds_value(c: &Creds) -> Value {
    Value::Object(
        c.iter()
            .map(|(e, ss)| {
                (
                    e.clone(),
                    Value::Object(
                        ss.iter()
                            .map(|(s, k)| (s.clone(), Value::from(k.clone())))
                            .collect(),
                    ),
                )
            })
            .collect(),
    )
}
pub fn resolve_credential(
    d: &Deps,
    env: &str,
    surface: &str,
) -> Result<Option<Credential>, String> {
    let def = surface_def(surface)?;
    if let Some(k) = d.env.get(def.env_var).filter(|k| !k.is_empty()) {
        return Ok(Some(Credential {
            source: "env",
            key: k.clone(),
        }));
    }
    let account = format!("{env}:{surface}");
    let from_keychain = if let Some(kc) = d.host.keychain() {
        kc.get("purchasely-cli", &account)
    } else if let Some(kc) = default_keychain(d.platform) {
        kc.get("purchasely-cli", &account)
    } else {
        Ok(None)
    };
    if let Ok(Some(k)) = from_keychain
        && !k.is_empty()
    {
        return Ok(Some(Credential {
            source: "keychain",
            key: k,
        }));
    }
    let file = read_creds(credentials_path(&d.config_dir));
    Ok(file
        .get(env)
        .and_then(|x| x.get(surface))
        .map(|k| Credential {
            source: "file",
            key: k.clone(),
        }))
}
pub fn store_credential(d: &Deps, env: &str, surface: &str, key: &str) -> Result<Source, String> {
    let account = format!("{env}:{surface}");
    let stored = if let Some(kc) = d.host.keychain() {
        kc.set("purchasely-cli", &account, key)
    } else if let Some(kc) = default_keychain(d.platform) {
        kc.set("purchasely-cli", &account, key)
    } else {
        Err(KeychainError("keychain unavailable".into()))
    };
    if stored.is_ok() {
        return Ok("keychain");
    }
    let path = credentials_path(&d.config_dir);
    let mut c = read_creds(path.clone());
    c.entry(env.into())
        .or_default()
        .insert(surface.into(), key.into());
    write_secret_json(path, &creds_value(&c)).map_err(|e| e.to_string())?;
    Ok("file")
}
pub fn remove_credential(d: &Deps, env: &str, surface: &str) -> Result<(), String> {
    let account = format!("{env}:{surface}");
    if let Some(kc) = d.host.keychain() {
        let _ = kc.delete("purchasely-cli", &account);
    } else if let Some(kc) = default_keychain(d.platform) {
        let _ = kc.delete("purchasely-cli", &account);
    }
    let path = credentials_path(&d.config_dir);
    let mut c = read_creds(path.clone());
    if let Some(x) = c.get_mut(env) {
        x.remove(surface);
        if x.is_empty() {
            c.remove(env);
        }
    }
    if c.is_empty() {
        let _ = std::fs::remove_file(path);
    } else {
        write_secret_json(path, &creds_value(&c)).map_err(|e| e.to_string())?
    }
    Ok(())
}
/// The macOS keychain through `/usr/bin/security` (D3). `program` is `security`; tests give a fake.
pub struct SecurityKeychain {
    pub program: std::path::PathBuf,
}
impl SecurityKeychain {
    fn call(&self, args: &[&str], input: Option<&[u8]>) -> Result<String, KeychainError> {
        let mut c = Command::new(&self.program);
        c.args(args).stdout(Stdio::piped()).stderr(Stdio::null());
        if input.is_some() {
            c.stdin(Stdio::piped());
        }
        let mut p = super::exec::spawn(&mut c).map_err(|e| KeychainError(e.to_string()))?;
        if let Some(bytes) = input
            && let Some(mut s) = p.stdin.take()
        {
            s.write_all(bytes)
                .map_err(|e| KeychainError(e.to_string()))?
        }
        let o = p
            .wait_with_output()
            .map_err(|e| KeychainError(e.to_string()))?;
        if !o.status.success() {
            return Err(KeychainError("security command failed".into()));
        }
        Ok(String::from_utf8_lossy(&o.stdout).trim().into())
    }
}
impl Keychain for SecurityKeychain {
    fn get(&self, s: &str, a: &str) -> Result<Option<String>, KeychainError> {
        let o = self.call(&["find-generic-password", "-s", s, "-a", a, "-w"], None)?;
        Ok((!o.is_empty()).then_some(o))
    }
    fn set(&self, s: &str, a: &str, k: &str) -> Result<(), KeychainError> {
        let hex = k
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        self.call(
            &["-i"],
            Some(format!("add-generic-password -U -s {s} -a {a} -X {hex}\n").as_bytes()),
        )
        .map(|_| ())
    }
    fn delete(&self, s: &str, a: &str) -> Result<(), KeychainError> {
        self.call(&["delete-generic-password", "-s", s, "-a", a], None)
            .map(|_| ())
    }
}
pub fn default_keychain(platform: &str) -> Option<SecurityKeychain> {
    (platform == "darwin").then(|| SecurityKeychain {
        program: "security".into(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::deps::*;
    use std::{collections::BTreeMap, path::PathBuf};
    struct H {
        kc: Option<Box<dyn Keychain>>,
    }
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
            self.kc.as_deref()
        }
        fn io(&self) -> Option<&dyn Io> {
            None
        }
    }
    fn d(dir: PathBuf, kc: Option<Box<dyn Keychain>>) -> Deps {
        Deps {
            host: Box::new(H { kc }),
            env: BTreeMap::new(),
            is_tty: false,
            config_dir: dir.clone(),
            state_dir: dir.clone(),
            cache_dir: dir,
            version: "x".into(),
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
    #[test]
    fn keychain_via_fake_security() {
        let root = std::env::temp_dir().join(format!("fake-security-{}", std::process::id()));
        let bin = root.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let script = bin.join("security");
        std::fs::write(&script,"#!/bin/sh\nif [ \"$1\" = find-generic-password ]; then echo keychain-secret; exit 0; fi\nexit 0\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let kc = SecurityKeychain { program: script };
        assert_eq!(
            kc.get("purchasely-cli", "prod:management")
                .unwrap()
                .as_deref(),
            Some("keychain-secret")
        );
        let _ = std::fs::remove_dir_all(root);
    }
    struct BrokenKeychain;
    impl Keychain for BrokenKeychain {
        fn get(&self, _: &str, _: &str) -> Result<Option<String>, KeychainError> {
            Err(KeychainError("unavailable".into()))
        }
        fn set(&self, _: &str, _: &str, _: &str) -> Result<(), KeychainError> {
            Err(KeychainError("unavailable".into()))
        }
        fn delete(&self, _: &str, _: &str) -> Result<(), KeychainError> {
            Err(KeychainError("unavailable".into()))
        }
    }
    #[test]
    fn fallback_on_failure() {
        let dir = std::env::temp_dir().join(format!("credential-fallback-{}", std::process::id()));
        let d = d(dir.clone(), Some(Box::new(BrokenKeychain)));
        let creds = Value::Object(vec![(
            "prod".into(),
            Value::Object(vec![(
                "management".into(),
                Value::from("file-secret".into()),
            )]),
        )]);
        write_secret_json(credentials_path(&dir), &creds).unwrap();
        let got = resolve_credential(&d, "prod", "management")
            .unwrap()
            .unwrap();
        assert_eq!(got.source, "file");
        assert_eq!(got.key, "file-secret");
        let _ = std::fs::remove_dir_all(dir);
    }
}
