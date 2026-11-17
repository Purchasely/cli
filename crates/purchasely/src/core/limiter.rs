//! File-backed request spacing, write serialization, and in-flight cap.
use super::{deps::Deps, json::Value, state::StateStore};
use std::sync::atomic::{AtomicU64, Ordering};
pub const MAX_IN_FLIGHT: usize = 4;
pub const WRITE_GAP_MS: i64 = 7000;
pub const STALE_SLOT_MS: i64 = 300_000;
static IDS: AtomicU64 = AtomicU64::new(0);
#[derive(Clone, Debug)]
struct Slot {
    id: String,
    pid: u32,
    at: i64,
}
fn number(v: &Value, k: &str) -> i64 {
    if let Value::Object(a) = v
        && let Some((_, Value::Number(n))) = a.iter().find(|(x, _)| x == k)
    {
        return *n as i64;
    }
    0
}
fn fresh() -> Value {
    Value::Object(vec![
        ("next_at".into(), Value::Number(0.)),
        ("next_write_at".into(), Value::Number(0.)),
        ("inflight".into(), Value::Array(vec![])),
    ])
}
fn slots(v: &Value) -> Vec<Slot> {
    if let Value::Object(o) = v
        && let Some((_, Value::Array(a))) = o.iter().find(|(k, _)| k == "inflight")
    {
        return a
            .iter()
            .filter_map(|x| {
                if let Value::Object(x) = x {
                    let get = |k: &str| x.iter().find(|(n, _)| n == k).map(|(_, v)| v);
                    Some(Slot {
                        id: get("id")?.as_str()?,
                        pid: match get("pid")? {
                            Value::Number(n) => *n as u32,
                            _ => return None,
                        },
                        at: match get("at")? {
                            Value::Number(n) => *n as i64,
                            _ => return None,
                        },
                    })
                } else {
                    None
                }
            })
            .collect();
    }
    vec![]
}
fn value(next: i64, write: i64, s: &[Slot]) -> Value {
    Value::Object(vec![
        ("next_at".into(), Value::Number(next as f64)),
        ("next_write_at".into(), Value::Number(write as f64)),
        (
            "inflight".into(),
            Value::Array(
                s.iter()
                    .map(|i| {
                        Value::Object(vec![
                            ("id".into(), Value::from(i.id.clone())),
                            ("pid".into(), Value::Number(i.pid as f64)),
                            ("at".into(), Value::Number(i.at as f64)),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}
pub struct Limiter<'a> {
    d: &'a Deps,
    store: &'a StateStore<'a>,
}
pub struct Permit<'a> {
    pub at: i64,
    store: &'a StateStore<'a>,
    id: String,
}
impl Permit<'_> {
    /// TS `release`: a failed state write fails the call, as a failed lock does.
    pub fn release(self) -> Result<(), String> {
        self.store.with_lock(|| {
            let v = self.store.read("limiter", fresh());
            let mut s = slots(&v);
            s.retain(|x| x.id != self.id);
            self.store
                .write(
                    "limiter",
                    &value(number(&v, "next_at"), number(&v, "next_write_at"), &s),
                )
                .map_err(|e| e.to_string())
        })?
    }
}
impl<'a> Limiter<'a> {
    pub fn new(d: &'a Deps, store: &'a StateStore<'a>) -> Self {
        Self { d, store }
    }
    fn wait(&self, until: i64) {
        while self.d.host.now() < until {
            self.d.host.sleep(until - self.d.host.now())
        }
    }
    pub fn acquire(&self, rate: f64, write: bool) -> Result<Permit<'a>, String> {
        if write {
            let at = self.store.with_lock(|| {
                let v = self.store.read("limiter", fresh());
                let n = self.d.host.now().max(number(&v, "next_write_at"));
                let s = slots(&v);
                self.store
                    .write(
                        "limiter",
                        &value(number(&v, "next_at"), n + WRITE_GAP_MS, &s),
                    )
                    .map_err(|e| e.to_string())?;
                Ok::<_, String>(n)
            })??;
            self.wait(at)
        }
        let id = format!("{}-{}", self.d.pid, IDS.fetch_add(1, Ordering::Relaxed));
        loop {
            let got = self.store.with_lock(|| {
                let v = self.store.read("limiter", fresh());
                let now = self.d.host.now();
                let mut s = slots(&v);
                s.retain(|x| {
                    now - x.at < STALE_SLOT_MS
                        && (x.pid == self.d.pid || self.d.host.pid_alive(x.pid))
                });
                if s.len() >= MAX_IN_FLIGHT {
                    self.store
                        .write(
                            "limiter",
                            &value(number(&v, "next_at"), number(&v, "next_write_at"), &s),
                        )
                        .map_err(|e| e.to_string())?;
                    return Ok::<bool, String>(false);
                }
                s.push(Slot {
                    id: id.clone(),
                    pid: self.d.pid,
                    at: now,
                });
                self.store
                    .write(
                        "limiter",
                        &value(number(&v, "next_at"), number(&v, "next_write_at"), &s),
                    )
                    .map_err(|e| e.to_string())?;
                Ok(true)
            })??;
            if got {
                break;
            }
            self.d.host.sleep(25)
        }
        let at = self.store.with_lock(|| {
            let v = self.store.read("limiter", fresh());
            let n = self.d.host.now().max(number(&v, "next_at"));
            let s = slots(&v);
            self.store
                .write(
                    "limiter",
                    &value(
                        n + (1000.0 / rate).ceil() as i64,
                        number(&v, "next_write_at"),
                        &s,
                    ),
                )
                .map_err(|e| e.to_string())?;
            Ok::<_, String>(n)
        })??;
        self.wait(at);
        Ok(Permit {
            at,
            store: self.store,
            id,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::deps::*;
    use std::{
        collections::BTreeMap,
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicI64, Ordering},
        },
    };
    struct Clock(Arc<AtomicI64>);
    impl Host for Clock {
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
            self.0.load(Ordering::SeqCst)
        }
        fn wall_now(&self) -> i64 {
            self.now()
        }
        fn pid_alive(&self, _: u32) -> bool {
            false
        }
        fn sleep(&self, n: i64) {
            self.0.fetch_add(n, Ordering::SeqCst);
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
    fn fixture() -> (Deps, Arc<AtomicI64>, PathBuf) {
        let n = Arc::new(AtomicI64::new(0));
        let dir = std::env::temp_dir().join(format!("limiter-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let d = Deps {
            host: Box::new(Clock(n.clone())),
            env: BTreeMap::new(),
            is_tty: false,
            config_dir: dir.clone(),
            state_dir: dir.clone(),
            cache_dir: dir.clone(),
            version: "x".into(),
            pid: std::process::id(),
            name: "purchasely".into(),
            host_url: String::new(),
            ext: None,
            self_exe: PathBuf::new(),
            platform: "linux",
            stdout_tty: false,
            columns: None,
        };
        (d, n, dir)
    }
    #[test]
    fn fake_clock_reserves_rate_and_write_slots() {
        let (d, clock, dir) = fixture();
        let store = StateStore::new(&d, "limit");
        let l = Limiter::new(&d, &store);
        let a = l.acquire(0.8, false).unwrap();
        let b = l.acquire(0.8, false).unwrap();
        assert_eq!((a.at, b.at), (0, 1250));
        a.release().unwrap();
        b.release().unwrap();
        let c = l.acquire(0.8, true).unwrap();
        assert_eq!(c.at, 2500);
        let dpermit = l.acquire(0.8, true).unwrap();
        assert_eq!(dpermit.at, 8250);
        c.release().unwrap();
        dpermit.release().unwrap();
        assert!(clock.load(Ordering::SeqCst) >= 8250);
        let _ = std::fs::remove_dir_all(dir);
    }
    /// The real clock of the process, in ms, with real sleeps: several threads or processes share it.
    struct RealClock;
    impl Host for RealClock {
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
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64
        }
        fn wall_now(&self) -> i64 {
            self.now()
        }
        fn pid_alive(&self, p: u32) -> bool {
            unsafe { libc::kill(p as i32, 0) == 0 }
        }
        fn sleep(&self, n: i64) {
            std::thread::sleep(std::time::Duration::from_millis(n.max(0) as u64))
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
    /// `fixture()` with its own dir: `fixture()` clears a dir that is the same for each test.
    fn deps_in(dir: &std::path::Path, host: Box<dyn Host>) -> Deps {
        Deps {
            host,
            env: BTreeMap::new(),
            is_tty: false,
            config_dir: dir.into(),
            state_dir: dir.into(),
            cache_dir: dir.into(),
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
    fn temp(tag: &str) -> PathBuf {
        static N: AtomicI64 = AtomicI64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "limiter-{tag}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
    fn max_in_window(starts: &[i64], window: i64) -> usize {
        starts
            .iter()
            .map(|s| {
                starts
                    .iter()
                    .filter(|x| **x >= *s && **x < s + window)
                    .count()
            })
            .max()
            .unwrap_or(0)
    }
    fn max_concurrent(spans: &[(i64, i64)]) -> usize {
        let mut events = spans
            .iter()
            .flat_map(|(start, end)| [(*start, 1), (*end, -1)])
            .collect::<Vec<_>>();
        events.sort();
        let (mut current, mut max) = (0i64, 0i64);
        for (_, delta) in events {
            current += delta;
            max = max.max(current);
        }
        max as usize
    }

    /// limiter.test.ts "50 parallel callers never exceed the rate nor 4 in flight", on the real clock
    /// with the times divided by 10: rate 50 per s in 100 ms windows, each call held 100 ms.
    #[test]
    fn fifty_parallel_callers_never_exceed_the_rate_nor_4_in_flight() {
        let dir = temp("parallel");
        let rate = 50.0;
        let spans = std::sync::Mutex::new(Vec::new());
        std::thread::scope(|scope| {
            for _ in 0..50 {
                scope.spawn(|| {
                    let d = deps_in(&dir, Box::new(RealClock));
                    let store = StateStore::new(&d, "k");
                    let permit = Limiter::new(&d, &store).acquire(rate, false).unwrap();
                    let start = permit.at;
                    d.host.sleep(100);
                    spans.lock().unwrap().push((start, d.host.now()));
                    permit.release().unwrap();
                });
            }
        });
        let spans = spans.into_inner().unwrap();
        assert_eq!(spans.len(), 50);
        let starts = spans.iter().map(|s| s.0).collect::<Vec<_>>();
        assert!(max_in_window(&starts, 100) <= 5);
        // The cap was reached: the test bites.
        assert_eq!(max_concurrent(&spans), MAX_IN_FLIGHT);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// limiter.test.ts "spaces writes by 7 s".
    #[test]
    fn spaces_writes_by_7_s() {
        let dir = temp("writes");
        let clock = Arc::new(AtomicI64::new(1_000_000));
        let d = deps_in(&dir, Box::new(Clock(clock.clone())));
        let store = StateStore::new(&d, "k");
        let limiter = Limiter::new(&d, &store);
        let ats = (0..3)
            .map(|_| {
                let permit = limiter.acquire(1.0, true).unwrap();
                let at = clock.load(Ordering::SeqCst);
                permit.release().unwrap();
                at
            })
            .collect::<Vec<_>>();
        assert!(ats[1] - ats[0] >= WRITE_GAP_MS, "{ats:?}");
        assert!(ats[2] - ats[1] >= WRITE_GAP_MS, "{ats:?}");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// limiter.test.ts "frees the slots of a dead process": 4 slots of a dead pid do not block.
    #[test]
    fn frees_the_slots_of_a_dead_process() {
        let dir = temp("dead-slots");
        let clock = Arc::new(AtomicI64::new(1_000_000));
        let d = deps_in(&dir, Box::new(Clock(clock.clone())));
        let store = StateStore::new(&d, "k");
        let dead = (0..4)
            .map(|i| Slot {
                id: format!("x{i}"),
                pid: (1 << 22) + 7,
                at: 1_000_000,
            })
            .collect::<Vec<_>>();
        store.write("limiter", &value(0, 0, &dead)).unwrap();
        let permit = Limiter::new(&d, &store).acquire(5.0, false).unwrap();
        permit.release().unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }

    /// limiter.test.ts "breaks the lock of a dead process".
    #[test]
    fn breaks_the_lock_of_a_dead_process() {
        let dir = temp("dead-lock");
        let d = deps_in(&dir, Box::new(Clock(Arc::new(AtomicI64::new(1_000_000)))));
        std::fs::write(dir.join("k.lock"), ((1u32 << 22) + 7).to_string()).unwrap();
        assert_eq!(StateStore::new(&d, "k").with_lock(|| 42), Ok(42));
        let _ = std::fs::remove_dir_all(dir);
    }

    /// limiter.test.ts "limiter (8 real processes)": 8 processes of 2 calls each, rate 10, held 500 ms.
    #[test]
    fn keeps_the_same_guarantees_across_8_processes() {
        let exe = std::env::current_exe().unwrap();
        let dir = temp("processes");
        let log = dir.join("log.jsonl");
        std::fs::write(&log, "").unwrap();
        let children = (0..8)
            .map(|_| {
                std::process::Command::new(&exe)
                    .args([
                        "--exact",
                        "core::limiter::tests::limiter_child",
                        "--nocapture",
                    ])
                    .env("LIMITER_CHILD_DIR", &dir)
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
        let rows = std::fs::read_to_string(&log)
            .unwrap()
            .lines()
            .map(|line| {
                let n = line
                    .split(' ')
                    .map(|x| x.parse::<i64>().unwrap())
                    .collect::<Vec<_>>();
                (n[0], n[1], n[2])
            })
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 16);
        let ats = rows.iter().map(|r| r.0).collect::<Vec<_>>();
        assert!(max_in_window(&ats, 1000) <= 10);
        let spans = rows.iter().map(|r| (r.1, r.2)).collect::<Vec<_>>();
        assert!(max_concurrent(&spans) <= MAX_IN_FLIGHT);
        let _ = std::fs::remove_dir_all(dir);
    }
    /// The child of `keeps_the_same_guarantees_across_8_processes`: it logs `at start end` per call.
    #[test]
    fn limiter_child() {
        let Some(dir) = std::env::var_os("LIMITER_CHILD_DIR") else {
            return;
        };
        let dir = PathBuf::from(dir);
        let d = deps_in(&dir, Box::new(RealClock));
        let store = StateStore::new(&d, "shared");
        let limiter = Limiter::new(&d, &store);
        for _ in 0..2 {
            let permit = limiter.acquire(10.0, false).unwrap();
            let start = d.host.now();
            d.host.sleep(500);
            let line = format!("{} {start} {}\n", permit.at, d.host.now());
            use std::io::Write;
            std::fs::OpenOptions::new()
                .append(true)
                .open(dir.join("log.jsonl"))
                .unwrap()
                .write_all(line.as_bytes())
                .unwrap();
            permit.release().unwrap();
        }
    }
    /// TS `store.write` throws inside `release`: the error reaches the caller.
    #[test]
    fn release_returns_a_failed_state_write() {
        let dir = temp("release-write");
        let d = deps_in(&dir, Box::new(Clock(Arc::new(AtomicI64::new(1_000_000)))));
        let store = StateStore::new(&d, "k");
        let permit = Limiter::new(&d, &store).acquire(5.0, false).unwrap();
        // A folder at the state file path makes the rename of the write fail.
        std::fs::remove_file(dir.join("k.limiter.json")).unwrap();
        std::fs::create_dir(dir.join("k.limiter.json")).unwrap();
        assert!(permit.release().is_err());
        assert!(!dir.join("k.lock").exists());
        let _ = std::fs::remove_dir_all(dir);
    }
    #[test]
    fn cap_and_stale_slot_contract() {
        assert_eq!(MAX_IN_FLIGHT, 4);
        assert_eq!(WRITE_GAP_MS, 7000);
        assert_eq!(STALE_SLOT_MS, 300000)
    }
    #[test]
    fn slots_roundtrip() {
        let s = Slot {
            id: "a".into(),
            pid: 8,
            at: 3,
        };
        assert_eq!(slots(&value(4, 5, std::slice::from_ref(&s)))[0].id, s.id)
    }
}
