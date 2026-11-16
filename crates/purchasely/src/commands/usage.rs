//! Owner: P5b
use crate::{
    cli::{Command, Parsed},
    core::{
        credentials::{credential_display, key_hash},
        errors::CliError,
        json::Value,
        output::{EmitOptions, emit},
        runtime::{Runtime, add_globals},
        state::StateStore,
        usage::read_usage,
    },
};

fn field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    if let Value::Object(entries) = value {
        entries
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    } else {
        None
    }
}
fn number(value: &Value, key: &str) -> f64 {
    field(value, key)
        .and_then(|v| {
            if let Value::Number(n) = v {
                Some(*n)
            } else {
                None
            }
        })
        .unwrap_or(0.0)
}
fn usage(p: &Parsed, rt: &mut Runtime) -> Result<(), CliError> {
    let env = rt.env(p.opt("env"))?;
    let credential = rt
        .credential(&env, "management")
        .map_err(|message| CliError::new(1, "CREDENTIAL_READ_FAILED", vec![message]))?;
    let hash = credential
        .as_ref()
        .map(|credential| key_hash(&credential.key));
    let label = credential
        .as_ref()
        .map(|credential| credential_display("management", Some(&credential.key)));
    let mut rows: Vec<Value> = Vec::new();
    for line in read_usage(&rt.d) {
        if line.env != env
            || line.surface != "management"
            || rt.d.host.now() - parse_ts(&line.ts) >= 3_600_000
        {
            continue;
        }
        let key = format!("{}/{}/{}", line.env, line.surface, line.key);
        let existing = rows
            .iter_mut()
            .find(|row| field(row, "_id").and_then(Value::as_str).as_deref() == Some(key.as_str()));
        let row = if let Some(row) = existing {
            row
        } else {
            rows.push(Value::Object(vec![
                ("_id".into(), Value::from(key)),
                ("env".into(), Value::from(line.env.clone())),
                ("surface".into(), Value::from(line.surface.clone())),
                ("key".into(), Value::from(line.key.clone())),
                (
                    "label".into(),
                    Value::from(if hash.as_deref() == Some(&line.key) {
                        label.clone().unwrap_or_default()
                    } else {
                        format!("hash:{}", line.key)
                    }),
                ),
                ("requests".into(), Value::Number(0.0)),
                ("rejections".into(), Value::Number(0.0)),
            ]));
            rows.last_mut().unwrap()
        };
        if let Value::Object(entries) = row {
            if let Some((_, Value::Number(count))) =
                entries.iter_mut().find(|(name, _)| name == "requests")
            {
                *count += 1.0;
            }
            if line.rate_limited
                && let Some((_, Value::Number(count))) =
                    entries.iter_mut().find(|(name, _)| name == "rejections")
            {
                *count += 1.0;
            }
        }
    }
    for row in &mut rows {
        if let Value::Object(entries) = row {
            entries.retain(|(name, _)| name != "_id");
        }
    }
    let current = credential.as_ref().map(|credential| {
        let key = key_hash(&credential.key);
        let state = StateStore::new(&rt.d, format!("{env}-management-{key}"));
        let limiter = state.read("limiter", Value::Object(vec![]));
        let breaker = crate::core::breaker::Breaker::new(&rt.d, &state).state();
        let inflight = field(&limiter, "inflight")
            .and_then(|v| {
                if let Value::Array(a) = v {
                    Some(a.len())
                } else {
                    None
                }
            })
            .unwrap_or(0);
        let paused_until = number(&breaker, "paused_until");
        let rejections = field(&breaker, "rejections")
            .and_then(|v| {
                if let Value::Array(a) = v {
                    Some(a.len())
                } else {
                    None
                }
            })
            .unwrap_or(0);
        Value::Object(vec![
            ("env".into(), Value::from(env.clone())),
            ("surface".into(), Value::from("management".to_owned())),
            ("key".into(), Value::from(key)),
            (
                "label".into(),
                Value::from(credential_display("management", Some(&credential.key))),
            ),
            (
                "rate".into(),
                Value::Number(rt.rate("management", None, false).unwrap_or(0.8)),
            ),
            ("in_flight".into(), Value::Number(inflight as f64)),
            (
                "breaker".into(),
                Value::Object(vec![
                    (
                        "paused_until".into(),
                        if paused_until > rt.d.host.now() as f64 {
                            Value::from(crate::core::time::iso_datetime(paused_until as i64))
                        } else {
                            Value::Null
                        },
                    ),
                    (
                        "rate_factor".into(),
                        Value::Number(number(&breaker, "rate_factor")),
                    ),
                    (
                        "rejections_last_5_min".into(),
                        Value::Number(rejections as f64),
                    ),
                ]),
            ),
        ])
    });
    let surfaces = current.clone().into_iter().collect::<Vec<_>>();
    let result = Value::Object(vec![
        ("last_hour".into(), Value::Array(rows)),
        ("current".into(), current.unwrap_or(Value::Null)),
        ("surfaces".into(), Value::Array(surfaces)),
    ]);
    emit(
        &rt.d,
        &result,
        &EmitOptions {
            table: p.opt("output") == Some("table"),
            out: p.opt("out").map(str::to_owned),
            env,
            key: credential.map(|credential| credential.key),
            name: "usage".into(),
            ..EmitOptions::default()
        },
    );
    Ok(())
}

fn parse_ts(value: &str) -> i64 {
    let date = value.split('T').next().unwrap_or("");
    let time = value.split('T').nth(1).unwrap_or("");
    let mut d = date.split('-').filter_map(|v| v.parse::<i64>().ok());
    let (Some(year), Some(month), Some(day)) = (d.next(), d.next(), d.next()) else {
        return 0;
    };
    let mut t = time.trim_end_matches('Z').split(':');
    let (Some(hour), Some(minute), Some(second)) = (t.next(), t.next(), t.next()) else {
        return 0;
    };
    let (sec, ms) = second.split_once('.').unwrap_or((second, "0"));
    let y = year - if month <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let days =
        era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + (153 * mp + 2) / 5 + day - 1 - 719468;
    days * 86_400_000
        + hour.parse::<i64>().unwrap_or(0) * 3_600_000
        + minute.parse::<i64>().unwrap_or(0) * 60_000
        + sec.parse::<i64>().unwrap_or(0) * 1000
        + ms.chars()
            .take(3)
            .collect::<String>()
            .parse::<i64>()
            .unwrap_or(0)
}

pub fn register(program: &mut Command, rt: &mut Runtime) {
    rt.taken.insert("usage".into());
    let mut command = Command::new("usage")
        .description("Requests of the last hour, rejections, limiter and breaker state");
    add_globals(&mut command);
    command = command.after_help("\nManagement API only.\n");
    program.commands.push(command.action(usage));
    rt.taken.insert("purge".into());
    let mut purge = Command::new("purge").description("Delete the cached client data");
    add_globals(&mut purge);
    purge = purge.after_help("\nDeletes the cached outputs over 200 KB. Without --env it deletes the cache of every environment.\n");
    program.commands.push(purge.action(|p, rt| {
        let target = p
            .opt("env")
            .map(|env| {
                rt.d.cache_dir
                    .join(rt.env(Some(env)).unwrap_or_else(|_| "prod".into()))
            })
            .unwrap_or_else(|| rt.d.cache_dir.clone());
        let _ = std::fs::remove_dir_all(&target);
        rt.d.host.stdout(&format!(
            "{}\n",
            crate::core::json::stringify(
                &Value::Object(vec![(
                    "purged".into(),
                    Value::from(target.to_string_lossy().into_owned())
                )]),
                None
            )
            .unwrap_or_default()
        ));
        Ok(())
    }));
}
