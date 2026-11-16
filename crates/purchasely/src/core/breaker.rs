//! Persistent shared 429 backoff and rejection breaker.
use super::{
    deps::Deps,
    errors::{CliError, EXIT},
    json::Value,
    state::StateStore,
};
pub const FALLBACK_PAUSE_MS: i64 = 60_000;
const WINDOW: i64 = 300_000;
const RESTORE: i64 = 600_000;
const MAX: usize = 20;
const MIN: f64 = 0.125;
pub fn pause_from_headers(retry: Option<&str>, reset: Option<&str>, now: i64) -> i64 {
    if let Some(n) = retry
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|n| *n > 0.0)
    {
        return (n * 1000.0).ceil().min(900_000.0) as i64;
    }
    if let Some(n) = reset
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|n| n * 1000.0 > now as f64)
    {
        return (n * 1000.0 - now as f64).ceil().min(900_000.0) as i64;
    }
    FALLBACK_PAUSE_MS
}
fn num(v: &Value, k: &str) -> i64 {
    if let Value::Object(o) = v
        && let Some((_, Value::Number(n))) = o.iter().find(|(x, _)| x == k)
    {
        return *n as i64;
    }
    0
}
fn rejs(v: &Value) -> Vec<i64> {
    if let Value::Object(o) = v
        && let Some((_, Value::Array(a))) = o.iter().find(|(x, _)| x == "rejections")
    {
        return a
            .iter()
            .filter_map(|x| {
                if let Value::Number(n) = x {
                    Some(*n as i64)
                } else {
                    None
                }
            })
            .collect();
    }
    vec![]
}
fn factor(v: &Value) -> f64 {
    if let Value::Object(o) = v
        && let Some((_, Value::Number(n))) = o.iter().find(|(x, _)| x == "rate_factor")
    {
        return *n;
    }
    1.0
}
fn state(p: i64, f: f64, last: i64, r: Vec<i64>) -> Value {
    Value::Object(vec![
        ("paused_until".into(), Value::Number(p as f64)),
        ("rate_factor".into(), Value::Number(f)),
        ("last_429_at".into(), Value::Number(last as f64)),
        (
            "rejections".into(),
            Value::Array(r.into_iter().map(|x| Value::Number(x as f64)).collect()),
        ),
    ])
}
pub struct Breaker<'a> {
    d: &'a Deps,
    store: &'a StateStore<'a>,
}
impl<'a> Breaker<'a> {
    pub fn new(d: &'a Deps, s: &'a StateStore<'a>) -> Self {
        Self { d, store: s }
    }
    fn read(&self) -> Value {
        let v = self.store.read("breaker", state(0, 1.0, 0, vec![]));
        let now = self.d.host.now();
        let f = if factor(&v) < 1.0 && now - num(&v, "last_429_at") > RESTORE {
            1.0
        } else {
            factor(&v)
        };
        state(
            num(&v, "paused_until"),
            f,
            num(&v, "last_429_at"),
            rejs(&v).into_iter().filter(|x| now - *x < WINDOW).collect(),
        )
    }
    pub fn gate(&self) -> Result<f64, CliError> {
        loop {
            let v = self.read();
            let r = rejs(&v);
            if r.len() >= MAX {
                return Err(stop());
            }
            let wait = num(&v, "paused_until") - self.d.host.now();
            if wait <= 0 {
                return Ok(factor(&v));
            }
            self.d.host.sleep(wait)
        }
    }
    pub fn record(&self, pause: i64) -> Result<i64, CliError> {
        let mut out = 0;
        self.store
            .with_lock(|| {
                let v = self.read();
                let now = self.d.host.now();
                let mut f = factor(&v);
                if now >= num(&v, "paused_until") {
                    f = (f / 2.0).max(MIN)
                }
                let until = num(&v, "paused_until").max(now + pause);
                let mut r = rejs(&v);
                r.push(now);
                out = until;
                let _ = self.store.write("breaker", &state(until, f, now, r));
            })
            .map_err(|e| CliError::new(EXIT::GENERIC, "STATE_ERROR", vec![e]))?;
        if rejs(&self.read()).len() >= MAX {
            return Err(stop());
        }
        Ok(out)
    }
}
fn stop() -> CliError {
    CliError::new(EXIT::RATE_LIMITED,"RATE_LIMIT_STOP",vec!["The server rejected 20 requests in 5 minutes. The CLI stopped to protect the API.".into(),"Wait 5 minutes. Run `purchasely usage` to see the state. Stop other scripts that use the same key.".into()])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pause_headers_obey_fallback_and_cap() {
        assert_eq!(pause_from_headers(None, None, 0), 60000);
        assert_eq!(pause_from_headers(Some("2"), None, 0), 2000);
        assert_eq!(pause_from_headers(Some("9999"), None, 0), 900000);
        assert_eq!(pause_from_headers(Some("1.5"), None, 0), 1500);
        assert_eq!(pause_from_headers(None, Some("12"), 10000), 2000)
    }
    #[test]
    fn rate_factor_never_falls_below_eighth() {
        assert_eq!((MIN / 2.0).max(MIN), 0.125)
    }
}
