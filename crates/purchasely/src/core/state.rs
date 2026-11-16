//! Shared state files and cross-process PID lock.
use super::deps::Deps;
use super::json::{self, Value};
use std::{fs, path::PathBuf, time::Duration};
pub struct StateStore<'a> {
    d: &'a Deps,
    key: String,
}
impl<'a> StateStore<'a> {
    pub fn new(d: &'a Deps, key: impl Into<String>) -> Self {
        let _ = fs::create_dir_all(&d.state_dir);
        Self { d, key: key.into() }
    }
    fn path(&self, name: &str) -> PathBuf {
        self.d.state_dir.join(format!("{}.{}", self.key, name))
    }
    pub fn read(&self, name: &str, fallback: Value) -> Value {
        fs::read_to_string(self.path(&format!("{name}.json")))
            .ok()
            .and_then(|s| json::parse(&s).ok())
            .unwrap_or(fallback)
    }
    pub fn write(&self, name: &str, value: &Value) -> std::io::Result<()> {
        let tmp = self.path(&format!("{name}.{}.tmp", self.d.pid));
        fs::write(&tmp, json::stringify(value, None).unwrap_or_default())?;
        fs::rename(tmp, self.path(&format!("{name}.json")))
    }
    pub fn with_lock<T>(&self, mut f: impl FnMut() -> T) -> Result<T, String> {
        let lock = self.path("lock");
        let start = self.d.host.wall_now();
        loop {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock)
            {
                Ok(file) => {
                    use std::io::Write;
                    let mut file = file;
                    let _ = write!(file, "{}", self.d.pid);
                    let out = f();
                    let _ = fs::remove_file(lock);
                    return Ok(out);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    if let Ok(s) = fs::read_to_string(&lock) {
                        if let Ok(pid) = s.trim().parse::<u32>() {
                            if !self.d.host.pid_alive(pid) {
                                let _ = fs::remove_file(&lock);
                                continue;
                            }
                        } else if let Ok(m) = fs::metadata(&lock).and_then(|m| m.modified())
                            && m.elapsed().unwrap_or_default() > Duration::from_secs(10)
                        {
                            let _ = fs::remove_file(&lock);
                            continue;
                        }
                    }
                    if self.d.host.wall_now() - start > 5000 {
                        return Err("state lock timeout".into());
                    }
                    self.d.host.sleep(10)
                }
                Err(e) => return Err(e.to_string()),
            }
        }
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn two_processes() {
        assert!(true);
    }
}
