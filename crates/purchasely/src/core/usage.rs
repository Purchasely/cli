//! Timestamped JSONL request usage log with daily retention and bounded compaction.
use super::{
    deps::Deps,
    json::{self, Value},
    time,
};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
};
pub const DAY_MS: i64 = 86_400_000;
pub const MAX_BYTES: u64 = 262_144;
#[derive(Clone, Debug, PartialEq)]
pub struct UsageLine {
    pub ts: String,
    pub env: String,
    pub surface: String,
    pub key: String,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub rate_limited: bool,
    pub request_id: String,
}
fn strfield(v: &Value, k: &str) -> String {
    if let Value::Object(o) = v
        && let Some((_, x)) = o.iter().find(|(n, _)| n == k)
    {
        return x.as_str().unwrap_or_default();
    }
    String::new()
}
fn num(v: &Value, k: &str) -> u16 {
    if let Value::Object(o) = v
        && let Some((_, Value::Number(x))) = o.iter().find(|(n, _)| n == k)
    {
        return *x as u16;
    }
    0
}
fn boolean(v: &Value, k: &str) -> bool {
    if let Value::Object(o) = v
        && let Some((_, Value::Bool(x))) = o.iter().find(|(n, _)| n == k)
    {
        return *x;
    }
    false
}
impl UsageLine {
    fn value(&self) -> Value {
        Value::Object(vec![
            ("ts".into(), Value::from(self.ts.clone())),
            ("env".into(), Value::from(self.env.clone())),
            ("surface".into(), Value::from(self.surface.clone())),
            ("key".into(), Value::from(self.key.clone())),
            ("method".into(), Value::from(self.method.clone())),
            ("path".into(), Value::from(self.path.clone())),
            ("status".into(), Value::Number(self.status as f64)),
            ("rate_limited".into(), Value::Bool(self.rate_limited)),
            ("request_id".into(), Value::from(self.request_id.clone())),
        ])
    }
    fn parse(s: &str) -> Option<Self> {
        let v = json::parse(s).ok()?;
        Some(Self {
            ts: strfield(&v, "ts"),
            env: strfield(&v, "env"),
            surface: strfield(&v, "surface"),
            key: strfield(&v, "key"),
            method: strfield(&v, "method"),
            path: strfield(&v, "path"),
            status: num(&v, "status"),
            rate_limited: boolean(&v, "rate_limited"),
            request_id: strfield(&v, "request_id"),
        })
    }
}
pub fn read_usage(d: &Deps) -> Vec<UsageLine> {
    let p = d.state_dir.join("usage.jsonl");
    let Ok(s) = fs::read_to_string(p) else {
        return vec![];
    };
    s.lines()
        .filter_map(UsageLine::parse)
        .filter(|u| {
            let t = parse_iso_ms(&u.ts);
            t.is_some_and(|t| d.host.now() - t < DAY_MS)
        })
        .collect()
}
fn parse_iso_ms(s: &str) -> Option<i64> {
    let (date, time) = s.split_once('T')?;
    let mut d = date.split('-');
    let (y, m, day) = (
        d.next()?.parse::<i64>().ok()?,
        d.next()?.parse::<u32>().ok()?,
        d.next()?.parse::<u32>().ok()?,
    );
    let mut t = time.trim_end_matches('Z').split(':');
    let (h, mi) = (
        t.next()?.parse::<i64>().ok()?,
        t.next()?.parse::<i64>().ok()?,
    );
    let sec = t.next()?;
    let (ss, frac) = sec.split_once('.').unwrap_or((sec, "0"));
    let ss = ss.parse::<i64>().ok()?;
    let ms = frac
        .chars()
        .take(3)
        .collect::<String>()
        .parse::<i64>()
        .ok()?;
    let days = days_from_civil(y, m, day);
    Some(days * DAY_MS + h * 3_600_000 + mi * 60_000 + ss * 1000 + ms)
}
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = y - if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = m as i64 + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
pub fn log_usage(d: &Deps, mut line: UsageLine) -> io::Result<()> {
    fs::create_dir_all(&d.state_dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&d.state_dir, fs::Permissions::from_mode(0o700))?;
    }
    line.ts = time::iso_datetime(d.host.now());
    let p = d.state_dir.join("usage.jsonl");
    let mut o = OpenOptions::new();
    o.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    let mut f = o.open(&p)?;
    writeln!(
        f,
        "{}",
        json::stringify(&line.value(), None).map_err(|e| io::Error::other(e.message))?
    )?;
    if f.metadata()?.len() > MAX_BYTES {
        drop(f);
        let kept = read_usage(d);
        let mut o = OpenOptions::new();
        o.create(true).truncate(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            o.mode(0o600);
        }
        let mut f = o.open(&p)?;
        for u in kept {
            writeln!(
                f,
                "{}",
                json::stringify(&u.value(), None).unwrap_or_default()
            )?
        }
    }
    Ok(())
}
pub fn append(path: &Path, line: &str) -> io::Result<()> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?
    }
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(f, "{line}")?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn usage_line_roundtrip_keeps_request_id() {
        let x = UsageLine {
            ts: "2024-01-01T00:00:00.000Z".into(),
            env: "prod".into(),
            surface: "management".into(),
            key: "abc".into(),
            method: "GET".into(),
            path: "/v1".into(),
            status: 200,
            rate_limited: false,
            request_id: "uuid".into(),
        };
        let s = json::stringify(&x.value(), None).unwrap();
        assert_eq!(UsageLine::parse(&s).unwrap(), x)
    }
    #[test]
    fn day_window_timestamp_parse() {
        assert_eq!(parse_iso_ms("1970-01-01T00:00:00.000Z"), Some(0));
        assert_eq!(DAY_MS, 86400000)
    }
}
