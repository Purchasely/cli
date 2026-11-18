use super::json::{self, Value};
use std::{
    fs, io,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    pub identity_email: Option<String>,
    pub rate: Option<Vec<(String, f64)>>,
    pub auto_update: Option<bool>,
    pub read_only: Option<bool>,
    pub write_allowlist: Option<Value>,
}
pub fn read_json(file: impl AsRef<Path>, fallback: Value) -> Value {
    fs::read_to_string(file)
        .ok()
        .and_then(|s| json::parse(&s).ok())
        .unwrap_or(fallback)
}
pub fn write_secret_json(file: impl AsRef<Path>, value: &Value) -> io::Result<()> {
    let file = file.as_ref();
    let dir = file.parent().unwrap_or(Path::new("."));
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    let data = format!(
        "{}\n",
        json::stringify_pretty(value)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.message))?
    );
    let mut opts = fs::OpenOptions::new();
    opts.create(true).truncate(true).write(true).mode(0o600);
    use std::io::Write;
    let mut f = opts.open(file)?;
    f.write_all(data.as_bytes())?;
    f.set_permissions(fs::Permissions::from_mode(0o600))?;
    Ok(())
}
pub fn config_path(dir: impl AsRef<Path>) -> PathBuf {
    dir.as_ref().join("config.json")
}
pub fn credentials_path(dir: impl AsRef<Path>) -> PathBuf {
    dir.as_ref().join("credentials.json")
}
pub fn read_config(dir: impl AsRef<Path>) -> Value {
    read_json(config_path(dir), Value::Object(Vec::new()))
}
pub fn write_config(dir: impl AsRef<Path>, value: &Value) -> io::Result<()> {
    write_secret_json(config_path(dir), value)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn config_file_names() {
        assert_eq!(
            config_path("/tmp/x").to_string_lossy(),
            "/tmp/x/config.json"
        );
        assert_eq!(
            credentials_path("/tmp/x").to_string_lossy(),
            "/tmp/x/credentials.json"
        );
    }
}
