use super::{
    deps::Deps,
    json::{self, Value},
};
pub const MAX_STDOUT_BYTES: usize = 200 * 1024;
const CACHE_TTL_MS: i64 = 24 * 3_600_000;
fn cell(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => String::from_utf16_lossy(s),
        Value::Object(_) | Value::Array(_) => json::stringify(v, None).unwrap_or_default(),
        _ => json::stringify(v, None).unwrap_or_default(),
    }
}
fn grid(rows: &[Vec<String>], width: usize) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let n = rows[0].len();
    let mut widths: Vec<usize> = (0..n)
        .map(|i| {
            rows.iter()
                .map(|r| r.get(i).map_or(0, String::len))
                .max()
                .unwrap_or(0)
        })
        .collect();
    while widths.iter().sum::<usize>() + 2 * n.saturating_sub(1) > width {
        let Some((i, _)) = widths.iter().enumerate().max_by_key(|(_, w)| *w) else {
            break;
        };
        if widths[i] <= 3 {
            break;
        }
        widths[i] -= 1;
    }
    rows.iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(i, c)| {
                    let w = widths[i];
                    let cut = if c.len() > w {
                        format!("{}...", &c[..w.saturating_sub(3)])
                    } else {
                        c.clone()
                    };
                    format!("{cut:<w$}")
                })
                .collect::<Vec<_>>()
                .join("  ")
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
pub fn to_table(rows: &[(&str, &str)], width: usize) -> String {
    let mut data = vec![vec!["key".into(), "value".into()]];
    data.extend(rows.iter().map(|(k, v)| vec![(*k).into(), (*v).into()]));
    grid(&data, width)
}
pub fn to_table_value(value: &Value, fields: Option<&[String]>, width: usize) -> Option<String> {
    let Value::Object(root) = value else {
        return None;
    };
    let roots: Vec<_> = root.iter().filter(|(k, _)| k != "meta").collect();
    if roots.len() != 1 {
        return None;
    }
    let body = &roots[0].1;
    if let Value::Array(items) = body {
        if items.is_empty() || !items.iter().all(|v| matches!(v, Value::Object(_))) {
            return None;
        }
        let mut keys = Vec::<String>::new();
        for item in items {
            if let Value::Object(o) = item {
                for (k, _) in o {
                    if !keys.contains(k) {
                        keys.push(k.clone())
                    }
                }
            }
        }
        let scalars: Vec<_> = keys
            .into_iter()
            .filter(|k| {
                items.iter().all(|v| {
                    if let Value::Object(o) = v {
                        o.iter().find(|(x, _)| x == k).is_none_or(|(_, v)| {
                            matches!(
                                v,
                                Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
                            )
                        })
                    } else {
                        false
                    }
                })
            })
            .collect();
        let selected: Vec<String> =
            if let Some(f) = fields.filter(|f| !f.is_empty() && !f.iter().any(|x| x == "*")) {
                f.to_vec()
            } else {
                let mut v: Vec<_> = ["id", "vendor_id", "name", "status", "type"]
                    .iter()
                    .filter(|k| scalars.iter().any(|x| x == **k))
                    .map(|x| x.to_string())
                    .collect();
                for x in scalars {
                    if v.len() >= 6 {
                        break;
                    }
                    if !v.contains(&x) {
                        v.push(x)
                    }
                }
                v
            };
        let mut rows = vec![selected.clone()];
        for item in items {
            if let Value::Object(o) = item {
                rows.push(
                    selected
                        .iter()
                        .map(|k| {
                            o.iter()
                                .find(|(x, _)| x == k)
                                .map(|(_, v)| cell(v))
                                .unwrap_or_default()
                        })
                        .collect(),
                )
            }
        }
        return Some(grid(&rows, width));
    }
    let o = if let Value::Object(inner) = body {
        inner
    } else {
        return None;
    };
    Some(grid(
        &std::iter::once(vec!["key".into(), "value".into()])
            .chain(o.iter().map(|(k, v)| vec![k.clone(), cell(v)]))
            .collect::<Vec<_>>(),
        width,
    ))
}
pub fn cache_dir_for(d: &Deps, env: &str, key: &str) -> std::path::PathBuf {
    use std::{fs, path::Path};
    let hash = format!("{:x}", sha2::Sha256::digest(key.as_bytes()));
    let dir = d.cache_dir.join(env).join(&hash[..12]);
    let _ = fs::create_dir_all(&dir);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o700));
    }
    if let Ok(entries) = fs::read_dir(&dir) {
        for e in entries.flatten() {
            if e.metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|age| age.as_millis() as i64 > CACHE_TTL_MS)
            {
                let _ = fs::remove_file(e.path());
            }
        }
    }
    let _ = Path::new("");
    dir
}
use sha2::Digest;
#[derive(Clone, Debug, Default)]
pub struct EmitOptions {
    pub table: bool,
    pub out: Option<String>,
    pub env: String,
    pub key: Option<String>,
    pub name: String,
    pub columns: Option<Vec<String>>,
    pub width: usize,
}
pub fn emit(d: &Deps, value: &Value, opts: &EmitOptions) {
    if opts.table
        && let Some(t) = to_table_value(
            value,
            opts.columns.as_deref(),
            if opts.width == 0 { 120 } else { opts.width },
        )
    {
        d.host.stdout(&format!("{t}\n"));
        return;
    }
    let text = format!(
        "{}\n",
        json::stringify_pretty(value).unwrap_or_else(|_| "null".into())
    );
    let bytes = text.len();
    if opts.out.as_deref() == Some("-") || (opts.out.is_none() && bytes <= MAX_STDOUT_BYTES) {
        d.host.stdout(&text);
        return;
    }
    let file = if let Some(file) = &opts.out {
        std::path::PathBuf::from(file)
    } else {
        let dir = cache_dir_for(d, &opts.env, opts.key.as_deref().unwrap_or(""));
        dir.join(format!(
            "{}-{}.json",
            d.host.now(),
            opts.name
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect::<String>()
        ))
    };
    if std::fs::write(&file, &text).is_ok() {
        d.host.stdout(&format!(
            "{}\n",
            json::stringify(
                &Value::Object(vec![
                    (
                        "file".into(),
                        Value::from(file.to_string_lossy().into_owned())
                    ),
                    ("bytes".into(), Value::Number(bytes as f64))
                ]),
                None
            )
            .unwrap_or_default()
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stdout_limit_is_200_kib() {
        assert_eq!(MAX_STDOUT_BYTES, 200 * 1024)
    }
    #[test]
    fn table_formats_rows() {
        assert_eq!(
            to_table(&[("id", "1"), ("name", "Ada")], 80),
            "key   value\nid    1\nname  Ada"
        )
    }
}
