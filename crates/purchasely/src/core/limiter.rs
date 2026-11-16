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
    pub fn release(self) -> Result<(), String> {
        self.store.with_lock(|| {
            let v = self.store.read("limiter", fresh());
            let mut s = slots(&v);
            s.retain(|x| x.id != self.id);
            let _ = self.store.write(
                "limiter",
                &value(number(&v, "next_at"), number(&v, "next_write_at"), &s),
            );
        })
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
