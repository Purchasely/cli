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

pub fn parse_value(param: &Param, raw: &str) -> Result<Value, crate::core::errors::CliError> {
    let flag = format!("--{}", param.name.replace('_', "-"));
    let invalid = |why: &str| usage(format!("{flag} {why}"));
    let scalar = |s: &str| -> Result<Value, crate::core::errors::CliError> {
        if !param.schema.choices.is_empty() && !param.schema.choices.iter().any(|v| v == s) {
            return Err(invalid(&format!(
                "must be one of: {}.",
                param.schema.choices.join(", ")
            )));
        }
        Ok(Value::from(s.to_owned()))
    };
    match param.schema.kind.as_deref() {
        Some("integer") => raw
            .parse::<i64>()
            .map(|v| Value::Number(v as f64))
            .map_err(|_| invalid("must be an integer.")),
        Some("boolean") => Ok(Value::Bool(raw == "true")),
        Some("array") => {
            let items: Vec<String> = if raw.trim_start().starts_with('[') {
                let parsed = json::parse(raw).map_err(|_| invalid("is not a valid JSON array."))?;
                let Value::Array(values) = parsed else {
                    return Err(invalid("is not a valid JSON array."));
                };
                values
                    .into_iter()
                    .map(|v| match v {
                        Value::String(s) => String::from_utf16_lossy(&s),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => String::new(),
                    })
                    .collect()
            } else {
                raw.split(',').map(str::to_owned).collect()
            };
            if param.name == "date_range"
                && (items.len() != 2
                    || items
                        .iter()
                        .any(|v| v.len() < 10 || !v.as_bytes().get(4).is_some_and(|c| *c == b'-')))
            {
                return Err(invalid(
                    "takes two dates (YYYY-MM-DD or ISO date-times): --date-range 2026-01-01,2026-10-01.",
                ));
            }
            let values = items
                .iter()
                .map(|s| scalar(s))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Value::Array(values))
        }
        _ => scalar(raw),
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

pub fn base_url(env: &str) -> &'static str {
    if env == "staging" {
        "https://api-staging.purchasely.io/management/v1"
    } else {
        "https://api.purchasely.io/management/v1"
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
    fn malformed_body_is_usage_error() {
        assert_eq!(json_body("{").unwrap_err().code, "USAGE");
    }
}
