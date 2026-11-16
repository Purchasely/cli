//! JSON state shared across CLI processes, protected by an exclusive PID lock.
use super::{
    deps::Deps,
    json::{self, Value},
};
use std::{
    fs,
    io::{self, Read, Write},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
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
    /// Removes the lock when it is stale: the pid is dead or, without a pid, the file is older
    /// than 10 s. An OS lock on the file lets one process at a time take it over, and the inode
    /// check keeps the unlink away from a fresh lock that another process put at the same path.
    fn remove_stale(&self, lock: &Path) -> bool {
        let Ok(mut fd) = fs::File::open(lock) else {
            return false;
        };
        // shortcut: a filesystem without flock keeps the unlocked check, as in the TS CLI.
        if matches!(fd.try_lock(), Err(fs::TryLockError::WouldBlock)) {
            return false;
        }
        let mut content = String::new();
        let _ = fd.read_to_string(&mut content);
        let Ok(meta) = fd.metadata() else {
            return false;
        };
        let pid = content.trim().parse::<u32>().unwrap_or(0);
        let stale = if pid == 0 {
            meta.modified()
                .map(|m| m.elapsed().unwrap_or_default() > Duration::from_secs(10))
                .unwrap_or(false)
        } else {
            !self.d.host.pid_alive(pid)
        };
        stale
            && fs::metadata(lock)
                .is_ok_and(|now| (now.dev(), now.ino()) == (meta.dev(), meta.ino()))
            && fs::remove_file(lock).is_ok()
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
                    write!(fd, "{}", self.d.pid).map_err(|e| e.to_string())?;
                    drop(fd);
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(&mut f));
                    let _ = fs::remove_file(&lock);
                    return match result {
                        Ok(x) => Ok(x),
                        Err(p) => std::panic::resume_unwind(p),
                    };
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    if self.remove_stale(&lock) {
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
    const DEAD: u32 = i32::MAX as u32;
    const RIVAL: u32 = 1;
    /// `sleep` advances the clock. `rival` is a process that takes the stale lock over while
    /// this one checks the dead pid.
    #[derive(Default)]
    struct Fake {
        clock: std::sync::atomic::AtomicI64,
        rival: Option<PathBuf>,
        real_sleep: bool,
    }
    impl Host for Fake {
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
            self.wall_now()
        }
        fn wall_now(&self) -> i64 {
            self.clock.load(std::sync::atomic::Ordering::SeqCst)
        }
        fn pid_alive(&self, p: u32) -> bool {
            if p == DEAD
                && let Some(lock) = &self.rival
            {
                fs::remove_file(lock).unwrap();
                fs::write(lock, RIVAL.to_string()).unwrap();
            }
            p != DEAD
        }
        fn sleep(&self, n: i64) {
            self.clock.fetch_add(n, std::sync::atomic::Ordering::SeqCst);
            if self.real_sleep {
                std::thread::sleep(Duration::from_millis(n as u64))
            }
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
        deps_with(dir, Box::new(H))
    }
    fn deps_with(dir: PathBuf, host: Box<dyn Host>) -> Deps {
        Deps {
            host,
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
    fn stale_takeover_keeps_a_lock_created_meanwhile() {
        let d = tempfile_dir();
        let lock = d.join("t.lock");
        fs::write(&lock, DEAD.to_string()).unwrap();
        let host = Fake {
            rival: Some(lock.clone()),
            ..Fake::default()
        };
        let dep = deps_with(d.clone(), Box::new(host));
        let mut ran = false;
        let result = StateStore::new(&dep, "t").with_lock(|| ran = true);
        assert_eq!(result, Err("state lock timeout".into()));
        assert!(
            !ran,
            "the critical section ran while the rival held the lock"
        );
        assert_eq!(fs::read_to_string(&lock).unwrap(), RIVAL.to_string());
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn stale_takeover_waits_for_a_takeover_in_progress() {
        let d = tempfile_dir();
        let lock = d.join("t.lock");
        fs::write(&lock, DEAD.to_string()).unwrap();
        // Another process is in the middle of the takeover: it holds the OS lock.
        let other = fs::File::open(&lock).unwrap();
        other.try_lock().unwrap();
        let dep = deps_with(d.clone(), Box::new(Fake::default()));
        let mut ran = false;
        let result = StateStore::new(&dep, "t").with_lock(|| ran = true);
        assert_eq!(result, Err("state lock timeout".into()));
        assert!(!ran);
        assert_eq!(fs::read_to_string(&lock).unwrap(), DEAD.to_string());
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn stale_lock_that_cannot_be_removed_times_out() {
        let d = tempfile_dir();
        // A directory stands for a stale lock that `unlink` cannot remove.
        let lock = d.join("t.lock");
        fs::create_dir(&lock).unwrap();
        let old = std::time::SystemTime::now() - Duration::from_secs(11);
        fs::File::open(&lock).unwrap().set_modified(old).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let dir = d.clone();
        std::thread::spawn(move || {
            let dep = deps_with(dir, Box::new(Fake::default()));
            let _ = tx.send(StateStore::new(&dep, "t").with_lock(|| ()));
        });
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(5)),
            Ok(Err("state lock timeout".into()))
        );
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn threads_and_crashed_holders_lose_no_update() {
        let d = tempfile_dir();
        let threads = (0..4)
            .map(|t| {
                let dir = d.clone();
                std::thread::spawn(move || {
                    let host = Fake {
                        real_sleep: true,
                        ..Fake::default()
                    };
                    let dep = deps_with(dir.clone(), Box::new(host));
                    let s = StateStore::new(&dep, "t");
                    for i in 0..25 {
                        // A holder that crashed: it took the lock and left it with a dead pid.
                        let mut o = fs::OpenOptions::new();
                        if (i + t) % 5 == 0
                            && let Ok(mut f) =
                                o.write(true).create_new(true).open(dir.join("t.lock"))
                        {
                            write!(f, "{DEAD}").unwrap();
                        }
                        s.with_lock(|| {
                            let Value::Number(n) = s.read("counter", Value::Number(0.0)) else {
                                panic!("counter")
                            };
                            s.write("counter", &Value::Number(n + 1.0)).unwrap();
                        })
                        .unwrap();
                    }
                })
            })
            .collect::<Vec<_>>();
        for t in threads {
            t.join().unwrap();
        }
        let dep = deps(d.clone());
        let total = StateStore::new(&dep, "t").read("counter", Value::Number(0.0));
        assert_eq!(total, Value::Number(100.0));
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
