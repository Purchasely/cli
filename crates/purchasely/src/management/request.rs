//! Owner: P6b
//! Build generated Management requests and read their JSON body sources.

use super::spec::{Command, Op, Param};
use crate::core::{
    errors::usage,
    json::{self, Value},
};
use std::{collections::BTreeMap, fs};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Built {
    pub op: Op,
    pub url: String,
    pub body: Option<String>,
}

pub fn attr(flag: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for c in flag.chars() {
        if c == '-' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// One occurrence of an array option: a JSON array of scalars when it starts with `[`, else `a,b`.
fn split_items(
    flag: &str,
    example: &str,
    s: &str,
) -> Result<Vec<String>, crate::core::errors::CliError> {
    if !s.trim_start().starts_with('[') {
        return Ok(s.split(',').map(str::to_owned).collect());
    }
    let Ok(Value::Array(items)) = json::parse(s) else {
        return Err(usage(format!(
            "{flag} is not a valid JSON array. Use {flag} {example}."
        )));
    };
    items
        .into_iter()
        .map(|v| match v {
            Value::String(s) => Ok(String::from_utf16_lossy(&s)),
            Value::Bool(b) => Ok(b.to_string()),
            Value::Number(_) => Ok(json::stringify(&v, None).unwrap_or_default()),
            _ => Err(usage(format!(
                "{flag} takes a JSON array of strings, numbers or booleans."
            ))),
        })
        .collect()
}

/// TS `DATE`: `YYYY-MM-DD`, optionally with `THH:MM[:SS[.fff]]` and a zone.
fn is_date(v: &str) -> bool {
    let b = v.as_bytes();
    let digits =
        |r: std::ops::Range<usize>| b.get(r).is_some_and(|s| s.iter().all(u8::is_ascii_digit));
    if !(b.len() >= 10
        && digits(0..4)
        && b[4] == b'-'
        && digits(5..7)
        && b[7] == b'-'
        && digits(8..10))
    {
        return false;
    }
    let mut rest = &v[10..];
    if rest.is_empty() {
        return true;
    }
    let r = rest.as_bytes();
    if !(r.len() >= 6
        && r[0] == b'T'
        && r[1..3].iter().all(u8::is_ascii_digit)
        && r[3] == b':'
        && r[4..6].iter().all(u8::is_ascii_digit))
    {
        return false;
    }
    rest = &rest[6..];
    if let Some(sec) = rest.strip_prefix(':') {
        let s = sec.as_bytes();
        if !(s.len() >= 2 && s[..2].iter().all(u8::is_ascii_digit)) {
            return false;
        }
        rest = &sec[2..];
        if let Some(frac) = rest.strip_prefix('.') {
            let n = frac.bytes().take_while(u8::is_ascii_digit).count();
            if n == 0 {
                return false;
            }
            rest = &frac[n..];
        }
    }
    let z = rest.as_bytes();
    rest.is_empty()
        || rest == "Z"
        || (z.len() == 6
            && (z[0] == b'+' || z[0] == b'-')
            && z[1..3].iter().all(u8::is_ascii_digit)
            && z[3] == b':'
            && z[4..6].iter().all(u8::is_ascii_digit))
}

/// TS `parseValue`: `raw` holds every occurrence of the option (an array option is repeatable).
pub fn parse_value(param: &Param, raw: &[String]) -> Result<Value, crate::core::errors::CliError> {
    let flag = format!("--{}", super::naming::kebab(&param.name));
    let check = |s: &str, choices: &[String]| -> Result<Value, crate::core::errors::CliError> {
        if !choices.is_empty() && !choices.iter().any(|v| v == s) {
            return Err(usage(format!(
                "{flag} must be one of: {}.",
                choices.join(", ")
            )));
        }
        Ok(Value::from(s.to_owned()))
    };
    let last = raw.last().map(String::as_str).unwrap_or_default();
    match param.schema.kind.as_deref() {
        Some("array") => {
            let example = if param.name == "date_range" {
                "2026-01-01,2026-10-01"
            } else {
                "a,b"
            };
            let mut items = Vec::new();
            for one in raw {
                items.extend(split_items(&flag, example, one)?);
            }
            items.retain(|v| !v.is_empty());
            if param.name == "date_range" && (items.len() != 2 || !items.iter().all(|v| is_date(v)))
            {
                return Err(usage(format!(
                    "{flag} takes two dates (YYYY-MM-DD or ISO date-times): --date-range 2026-01-01,2026-10-01."
                )));
            }
            let choices = param
                .schema
                .items
                .as_ref()
                .map(|i| i.choices.clone())
                .unwrap_or_default();
            Ok(Value::Array(
                items
                    .iter()
                    .map(|v| check(v, &choices))
                    .collect::<Result<_, _>>()?,
            ))
        }
        Some("integer") => {
            let digits = last.strip_prefix('-').unwrap_or(last);
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return Err(usage(format!("{flag} must be an integer.")));
            }
            Ok(Value::Number(last.parse::<f64>().unwrap_or(0.0)))
        }
        Some("boolean") => Ok(Value::Bool(last == "true")),
        _ => check(last, &param.schema.choices),
    }
}

pub fn build_request(
    cmd: &Command,
    values: &BTreeMap<String, Value>,
    env: &str,
    data: Option<&str>,
    token: Option<&str>,
) -> Result<Built, crate::core::errors::CliError> {
    let app = values.get("application_id").and_then(Value::as_str);
    let op = if app.is_some() {
        cmd.app_op.as_ref()
    } else {
        cmd.global_op.as_ref()
    }
    .ok_or_else(|| {
        usage(if app.is_some() {
            format!(
                "{} {} has no app-scoped form. Do not pass --app.",
                cmd.resource, cmd.action
            )
        } else {
            format!("--app is required for {} {}.", cmd.resource, cmd.action)
        })
    })?;
    for opt in &cmd.options {
        if values.contains_key(&opt.name) && !op.params.iter().any(|p| p.name == opt.name) {
            return Err(usage(format!(
                "--{} is not accepted {} ({} {}).",
                opt.flag,
                if app.is_some() {
                    "with --app"
                } else {
                    "without --app"
                },
                op.method,
                op.path
            )));
        }
    }
    let mut path = op.path.clone();
    let mut query = Vec::new();
    for p in &op.params {
        if p.name == "confirm_token" {
            continue;
        }
        let value = if p.name == "application_id" {
            app.as_ref().map(|s| Value::from(s.to_owned()))
        } else {
            values.get(&p.name).cloned()
        };
        let Some(value) = value else {
            if p.required {
                return Err(usage(format!(
                    "--{} is required.",
                    if p.name == "application_id" {
                        "app".into()
                    } else {
                        p.name.replace('_', "-")
                    }
                )));
            }
            continue;
        };
        if p.location == "path" {
            path = path.replace(
                &format!("{{{}}}", p.name),
                &encode_component(&value_text(&value)),
            );
        } else if let Value::Array(items) = value {
            for item in items {
                query.push(format!(
                    "{}[]={}",
                    encode_component(&p.name),
                    encode_component(&value_text(&item))
                ));
            }
        } else {
            query.push(format!(
                "{}={}",
                encode_component(&p.name),
                encode_component(&value_text(&value))
            ));
        }
    }
    let mut body = match (&op.body, data) {
        (Some(b), None) if b.required => {
            return Err(usage("--data is required: pass '<json>' or @file.json."));
        }
        (Some(_), Some(s)) => Some(json_body(s)?),
        (None, Some(_)) => {
            return Err(usage(format!(
                "{} {} takes no body. Do not pass --data.",
                op.method, op.path
            )));
        }
        _ => None,
    };
    let mut url = format!(
        "{}{}{}",
        base_url(env),
        path,
        if query.is_empty() {
            String::new()
        } else {
            format!("?{}", query.join("&"))
        }
    );
    if let Some(t) = token.filter(|_| op.confirm.as_deref() == Some("query")) {
        url.push_str(if url.contains('?') { "&" } else { "?" });
        url.push_str(&format!("confirm_token={}", encode_component(t)));
    }
    if let Some(t) = token.filter(|_| op.confirm.as_deref() == Some("body")) {
        let parsed = json::parse(body.as_deref().unwrap_or("{}"))
            .map_err(|_| usage("--data must be a JSON object."))?;
        let Value::Object(mut obj) = parsed else {
            return Err(usage("--data must be a JSON object."));
        };
        obj.retain(|(k, _)| k != "confirm_token");
        obj.push(("confirm_token".into(), Value::from(t.to_owned())));
        body = Some(json::stringify(&Value::Object(obj), None).unwrap_or_default());
    }
    Ok(Built {
        op: op.clone(),
        url,
        body,
    })
}

pub fn json_body(data: &str) -> Result<String, crate::core::errors::CliError> {
    let parsed = json::parse(data).map_err(|_| usage("--data is not valid JSON."))?;
    json::stringify(&parsed, None).map_err(|_| usage("--data is not valid JSON."))
}

pub fn read_data(raw: &str, stdin: &str) -> Result<String, crate::core::errors::CliError> {
    if raw == "-" {
        return Ok(stdin.to_owned());
    }
    if let Some(file) = raw.strip_prefix('@') {
        return fs::read_to_string(file)
            .map_err(|_| usage(format!("Cannot read the file {file}.")));
    }
    Ok(raw.to_owned())
}

pub fn with_token(
    confirm: Option<&str>,
    url: &str,
    body: Option<&str>,
    token: Option<&str>,
) -> Result<(String, Option<String>), crate::core::errors::CliError> {
    let Some(token) = token else {
        return Ok((url.to_owned(), body.map(str::to_owned)));
    };
    match confirm {
        Some("query") => Ok((
            format!(
                "{}{}confirm_token={}",
                url,
                if url.contains('?') { "&" } else { "?" },
                encode_component(token)
            ),
            body.map(str::to_owned),
        )),
        Some("body") => {
            let parsed = json::parse(body.unwrap_or("{}"))
                .map_err(|_| usage("--data must be a JSON object."))?;
            let Value::Object(mut obj) = parsed else {
                return Err(usage("--data must be a JSON object."));
            };
            obj.retain(|(k, _)| k != "confirm_token");
            obj.push(("confirm_token".into(), Value::from(token.to_owned())));
            Ok((
                url.to_owned(),
                Some(json::stringify(&Value::Object(obj), None).unwrap_or_default()),
            ))
        }
        _ => Ok((url.to_owned(), body.map(str::to_owned))),
    }
}

/// TS `BASE_URLS`: the host only; the spec paths start with `/management/v1`.
pub fn base_url(env: &str) -> &'static str {
    if env == "staging" {
        "https://api-staging.purchasely.io"
    } else {
        "https://api.purchasely.io"
    }
}
fn value_text(v: &Value) -> String {
    match v {
        Value::String(s) => String::from_utf16_lossy(s),
        _ => json::stringify(v, None).unwrap_or_default(),
    }
}
fn encode_component(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            o.push(b as char);
        } else {
            o.push_str(&format!("%{b:02X}"));
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn request_encodes_path_and_array_query() {
        let (_, ops) = super::super::spec::embedded_spec();
        let cmd = super::super::spec::derive_commands(&ops)
            .into_iter()
            .find(|c| c.resource == "placements" && c.action == "list")
            .unwrap();
        let mut values = BTreeMap::new();
        values.insert("application_id".into(), Value::from("app 1".to_owned()));
        values.insert(
            "fields".into(),
            Value::Array(vec![
                Value::from("id".to_owned()),
                Value::from("name".to_owned()),
            ]),
        );
        let built = build_request(&cmd, &values, "prod", None, None).unwrap();
        assert!(built.url.contains("app%201"));
        assert!(built.url.ends_with("fields[]=id&fields[]=name"));
    }
    #[test]
    fn parse_value_follows_ts() {
        use super::super::spec::{Param, Schema};
        let param = |name: &str, kind: &str| Param {
            name: name.into(),
            location: "query".into(),
            required: false,
            description: None,
            schema: Schema {
                kind: Some(kind.into()),
                ..Schema::default()
            },
        };
        let raw = |v: &[&str]| v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        let fields = parse_value(
            &param("fields", "array"),
            &raw(&["id,name", "[\"x\",2]", ""]),
        )
        .unwrap();
        assert_eq!(
            json::stringify(&fields, None).unwrap(),
            r#"["id","name","x","2"]"#
        );
        let dates = param("date_range", "array");
        assert!(parse_value(&dates, &raw(&["2026-01-01,2026-10-01T10:00:00Z"])).is_ok());
        assert!(parse_value(&dates, &raw(&["2026-01-01"])).is_err());
        assert!(parse_value(&dates, &raw(&["2026-01-01,2026-10-01T10"])).is_err());
        assert!(parse_value(&param("page", "integer"), &raw(&["-2"])).is_ok());
        assert_eq!(
            parse_value(&param("page", "integer"), &raw(&["2.5"]))
                .unwrap_err()
                .messages,
            ["--page must be an integer."]
        );
    }
    #[test]
    fn malformed_body_is_usage_error() {
        assert_eq!(json_body("{").unwrap_err().code, "USAGE");
    }
}
