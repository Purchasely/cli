//! JSON state shared across CLI processes, protected by an exclusive PID lock.
use super::{
    deps::Deps,
    json::{self, Value},
};
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
    time::Duration,
};
pub struct StateStore<'a> {
    d: &'a Deps,
    key: String,
}
impl<'a> StateStore<'a> {
    pub fn new(d: &'a Deps, key: impl Into<String>) -> Self {
        let _ = fs::create_dir_all(&d.state_dir);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&d.state_dir, fs::Permissions::from_mode(0o700));
        }
        Self { d, key: key.into() }
    }
    fn path(&self, n: &str) -> PathBuf {
        self.d.state_dir.join(format!("{}.{}", self.key, n))
    }
    pub fn read(&self, n: &str, fallback: Value) -> Value {
        fs::read_to_string(self.path(&format!("{n}.json")))
            .ok()
            .and_then(|x| json::parse(&x).ok())
            .unwrap_or(fallback)
    }
    pub fn write(&self, n: &str, v: &Value) -> io::Result<()> {
        let p = self.path(&format!("{n}.json"));
        let tmp = self.path(&format!("{n}.{}.tmp", self.d.pid));
        let mut o = fs::OpenOptions::new();
        o.create(true).truncate(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            o.mode(0o600);
        }
        let mut f = o.open(&tmp)?;
        f.write_all(json::stringify(v, None).unwrap_or_default().as_bytes())?;
        f.sync_all()?;
        fs::rename(tmp, p)
    }
    pub fn with_lock<T>(&self, mut f: impl FnMut() -> T) -> Result<T, String> {
        let lock = self.path("lock");
        let start = self.d.host.wall_now();
        loop {
            let mut o = fs::OpenOptions::new();
            o.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                o.mode(0o600);
            }
            match o.open(&lock) {
                Ok(mut fd) => {
                    writeln!(fd, "{}", self.d.pid).map_err(|e| e.to_string())?;
                    drop(fd);
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(&mut f));
                    let _ = fs::remove_file(&lock);
                    return match result {
                        Ok(x) => Ok(x),
                        Err(p) => std::panic::resume_unwind(p),
                    };
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    let content = fs::read_to_string(&lock).unwrap_or_default();
                    let pid = content.trim().parse::<u32>().unwrap_or(0);
                    let stale = if pid == 0 {
                        fs::metadata(&lock)
                            .and_then(|m| m.modified())
                            .map(|m| m.elapsed().unwrap_or_default() > Duration::from_secs(10))
                            .unwrap_or(false)
                    } else {
                        !self.d.host.pid_alive(pid)
                    };
                    if stale {
                        let _ = fs::remove_file(&lock);
                        continue;
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
    use super::*;
    use crate::core::deps::*;
    use std::{collections::BTreeMap, path::PathBuf, process::Command};
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
        fn pid_alive(&self, p: u32) -> bool {
            unsafe { libc::kill(p as i32, 0) == 0 }
        }
        fn sleep(&self, n: i64) {
            std::thread::sleep(Duration::from_millis(n as u64))
        }
        fn stdout(&self, _: &str) {}
        fn stderr(&self, _: &str) {}
        fn keychain(&self) -> Option<&dyn Keychain> {
            None
        }
        fn io(&self) -> Option<&dyn Io> {
            None
        }
    }
    fn deps(dir: PathBuf) -> Deps {
        Deps {
            host: Box::new(H),
            env: BTreeMap::new(),
            is_tty: false,
            config_dir: dir.clone(),
            state_dir: dir.clone(),
            cache_dir: dir,
            version: "x".into(),
            pid: std::process::id(),
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
    fn lock_file_is_exclusive_and_released() {
        let d = tempfile_dir();
        let dep = deps(d.clone());
        let store = StateStore::new(&dep, "t");
        assert_eq!(store.with_lock(|| 7).unwrap(), 7);
        assert!(!d.join("t.lock").exists());
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn two_processes() {
        let exe = std::env::current_exe().unwrap();
        let dir = tempfile_dir();
        let children = (0..2)
            .map(|_| {
                Command::new(&exe)
                    .args(["--exact", "core::state::tests::lock_child", "--nocapture"])
                    .env("P4_STATE_DIR", &dir)
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .unwrap()
            })
            .collect::<Vec<_>>();
        for child in children {
            let out = child.wait_with_output().unwrap();
            assert!(
                out.status.success(),
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
        }
        let value = fs::read_to_string(dir.join("child.counter.json")).unwrap();
        assert_eq!(json::parse(&value).unwrap(), Value::Number(2.0));
        let _ = fs::remove_dir_all(dir);
    }
    #[test]
    fn lock_child() {
        if std::env::var_os("P4_STATE_DIR").is_none() {
            return;
        }
        let d = PathBuf::from(std::env::var_os("P4_STATE_DIR").unwrap());
        let deps = deps(d.clone());
        let s = StateStore::new(&deps, "child");
        s.with_lock(|| {
            let previous = s.read("counter", Value::Number(0.0));
            let next = if let Value::Number(n) = previous {
                Value::Number(n + 1.0)
            } else {
                Value::Number(1.0)
            };
            std::thread::sleep(Duration::from_millis(40));
            s.write("counter", &next).unwrap();
        })
        .unwrap();
    }
    fn tempfile_dir() -> PathBuf {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("ply-state-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }
}
