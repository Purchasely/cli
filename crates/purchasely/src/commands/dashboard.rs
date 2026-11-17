//! Owner: P10
use crate::{
    cli::{Command, Parsed},
    core::{
        errors::{self, CliError, EXIT},
        http::{self, Ctx, HttpRequest},
        json::{self, Value},
        output::{self, EmitOptions},
        runtime::{self, Runtime},
        time,
    },
    games::term,
    management::{exec, request, spec},
};
use std::collections::BTreeMap;

const DAY: i64 = 86_400_000;
const BLOCKS: [char; 7] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇'];

pub fn sparkline(values: &[f64]) -> String {
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = max - min;
    values
        .iter()
        .map(|v| {
            let index = if span.is_finite() && span != 0.0 {
                (((*v - min) / span) * 6.0).round() as usize
            } else {
                0
            };
            BLOCKS[index.min(6)]
        })
        .collect()
}

fn date_range(now: i64, days: u32) -> (String, String) {
    (
        time::iso_date(now - i64::from(days.saturating_sub(1)) * DAY),
        time::iso_date(now),
    )
}

fn member<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    if let Value::Object(fields) = v {
        fields
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    } else {
        None
    }
}
fn number(v: Option<&Value>) -> f64 {
    if let Some(Value::Number(n)) = v {
        *n
    } else {
        0.0
    }
}
fn values(v: Option<&Value>) -> Vec<&Value> {
    if let Some(Value::Array(items)) = v {
        items.iter().collect()
    } else {
        Vec::new()
    }
}
fn series(root: &Value, path: &[&str], key: &str) -> Vec<(String, f64)> {
    let mut at = root;
    for part in path {
        let Some(next) = member(at, part) else {
            return Vec::new();
        };
        at = next;
    }
    values(Some(at))
        .into_iter()
        .map(|row| {
            (
                member(row, "period")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                number(member(row, key)),
            )
        })
        .collect()
}

fn generated(resource: &str, action: &str) -> Result<spec::Command, CliError> {
    let (_, ops) = spec::embedded_spec();
    spec::derive_commands(&ops).into_iter().find(|c| c.resource == resource && c.action == action)
        .ok_or_else(|| errors::usage(format!("dashboard summary needs dashboard {action}, which this version of the API spec does not have. Run purchasely update.")))
}

fn values_for(app: &str, range: &[String], periodicity: &str) -> BTreeMap<String, Value> {
    BTreeMap::from([
        ("application_id".into(), Value::from(app.to_owned())),
        (
            "date_range".into(),
            Value::Array(range.iter().cloned().map(Value::from).collect()),
        ),
        ("periodicity".into(), Value::from(periodicity.to_owned())),
    ])
}

fn context<'a>(rt: &'a Runtime, p: &Parsed) -> Result<Ctx<'a>, CliError> {
    let env = rt.env(p.opt("env"))?;
    let key = rt
        .credential(&env, "management")
        .map_err(|e| CliError::new(EXIT::GENERIC, "CREDENTIAL_ERROR", vec![e]))?
        .map(|c| c.key);
    Ok(Ctx {
        d: &rt.d,
        env,
        surface: "management".into(),
        key,
        identity: rt.cfg.identity_email.clone(),
        reason: p.opt("reason").map(str::to_owned),
        command: rt.command.clone(),
        rate: rt.rate("management", p.opt("rate"), p.flag("i-asked-backend"))?,
        headers: Vec::new(),
        read_only: rt.read_only(),
        timeout_ms: None,
    })
}

fn read(
    rt: &Runtime,
    ctx: &Ctx<'_>,
    action: &str,
    app: &str,
    range: &[String],
    periodicity: &str,
) -> Result<Value, CliError> {
    let cmd = generated("dashboard", action)?;
    let built = request::build_request(
        &cmd,
        &values_for(app, range, periodicity),
        &ctx.env,
        None,
        None,
    )?;
    let response = exec::read_request(
        rt,
        ctx,
        &HttpRequest {
            method: "GET".into(),
            url: built.url,
            path_template: built.op.path,
            body: None,
            write: Some(false),
            refuse_redirect: false,
            rate_limited: None,
        },
    )?;
    if !(200..300).contains(&response.status) {
        let body = json::parse(&response.text).unwrap_or(Value::Null);
        let code = member(member(&body, "error").unwrap_or(&Value::Null), "code")
            .and_then(Value::as_str)
            .unwrap_or_else(|| format!("HTTP_{}", response.status));
        let messages = values(member(
            member(&body, "error").unwrap_or(&Value::Null),
            "messages",
        ))
        .into_iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
        let mut error = CliError::new(
            errors::exit_for(Some(&code), response.status),
            code,
            if messages.is_empty() {
                vec![format!("HTTP {}", response.status)]
            } else {
                messages
            },
        );
        error.status = Some(response.status);
        if response.status == 404 {
            error
                .messages
                .insert(0, format!("application \"{app}\" not found"));
        }
        return Err(error);
    }
    Ok(if response.text.trim().is_empty() {
        Value::Object(Vec::new())
    } else {
        json::parse(&response.text).unwrap_or(Value::Null)
    })
}

fn summary(rt: &mut Runtime, p: &Parsed) -> Result<(), CliError> {
    let app = p
        .opt("app")
        .ok_or_else(|| errors::usage("dashboard summary needs --app <app_id>."))?;
    let days_text = p.opt("days").unwrap_or("30");
    let days = days_text
        .parse::<u32>()
        .ok()
        .filter(|n| (2..=730).contains(n))
        .ok_or_else(|| errors::usage("--days must be a whole number from 2 to 730."))?;
    let (from, to) = date_range(rt.d.host.now(), days);
    let periodicity = if days <= 60 {
        "DAY"
    } else if days <= 183 {
        "WEEK"
    } else {
        "MONTH"
    };
    let ctx = context(rt, p)?;
    let range = vec![from.clone(), to.clone()];
    let mut requests = vec![
        ("mrr-evolution-live", range.clone(), periodicity),
        ("subscription-evolution", range.clone(), periodicity),
        ("conversion", range, periodicity),
    ];
    if periodicity != "DAY" {
        requests.push(("mrr-evolution-live", vec![to.clone(), to.clone()], "DAY"));
    }
    if p.flag("dry-run") {
        let previews = requests
            .iter()
            .map(|(action, range, period)| {
                let cmd = generated("dashboard", action)?;
                let built = request::build_request(
                    &cmd,
                    &values_for(app, range, period),
                    &ctx.env,
                    None,
                    None,
                )?;
                Ok(Value::Object(vec![
                    ("method".into(), Value::from("GET".to_owned())),
                    ("url".into(), Value::from(built.url)),
                    (
                        "headers".into(),
                        Value::Object(vec![
                            (
                                "Authorization".into(),
                                Value::from("Bearer <redacted>".to_owned()),
                            ),
                            ("Accept".into(), Value::from("application/json".to_owned())),
                            ("User-Agent".into(), Value::from(http::ctx_user_agent(&ctx))),
                        ]),
                    ),
                ]))
            })
            .collect::<Result<Vec<_>, CliError>>()?;
        rt.d.host
            .stdout(&(json::stringify_pretty(&Value::Array(previews)).unwrap_or_default() + "\n"));
        return Ok(());
    }
    let mut bodies = Vec::new();
    for (action, range, period) in requests {
        bodies.push(read(rt, &ctx, action, app, &range, period)?);
    }
    let mrr = series(
        &bodies[0],
        &["mrr_evolution_live", "data", "mrr", "values", "by_day"],
        "value",
    );
    let subs = series(&bodies[1], &["subscription_evolution"], "count");
    let viewers = series(
        &bodies[2],
        &["conversion", "primary_metric_by_period"],
        "unique_viewers_count",
    );
    let conversions = series(
        &bodies[2],
        &["conversion", "secondary_metric_by_period"],
        "total_conversions_count",
    );
    let today_mrr = if bodies.len() == 4 {
        series(
            &bodies[3],
            &["mrr_evolution_live", "data", "mrr", "values", "by_day"],
            "value",
        )
    } else {
        Vec::new()
    };
    let kpi = values(member(
        member(&bodies[2], "conversion").unwrap_or(&Value::Null),
        "kpi",
    ))
    .first()
    .copied()
    .map(|v| number(member(v, "unique_viewers_count")))
    .unwrap_or(0.0);
    let last = |s: &[(String, f64)]| s.last().map_or(0.0, |v| v.1);
    let sum = |s: &[(String, f64)]| s.iter().map(|v| v.1).sum::<f64>();
    let panels = vec![
        (
            "MRR",
            if today_mrr.is_empty() {
                last(&mrr)
            } else {
                last(&today_mrr)
            },
            &mrr,
        ),
        ("Active subscriptions", last(&subs), &subs),
        ("Paywall unique viewers", kpi, &viewers),
        ("Conversions", sum(&conversions), &conversions),
    ];
    let output = p.opt("output").unwrap_or("json");
    if output == "table" {
        let name_width = panels
            .iter()
            .map(|(name, _, _)| name.len())
            .max()
            .unwrap_or(0);
        let value_width = panels
            .iter()
            .map(|(_, value, _)| fmt(if *value == 0.0 { 0.0 } else { *value }).len())
            .max()
            .unwrap_or(0);
        let mut out = format!("{app}  {from} to {to}  ({periodicity})\n\n");
        for (name, value, series) in &panels {
            let line = if series.is_empty() {
                "(no data)".into()
            } else {
                sparkline(&series.iter().map(|v| v.1).collect::<Vec<_>>())
            };
            let value = if *value == 0.0 { 0.0 } else { *value };
            out.push_str(&format!(
                "{name:<name_width$}  {:>value_width$}  {line}\n",
                fmt(value)
            ));
        }
        rt.d.host.stdout(&out);
    } else {
        let json_panels = panels
            .iter()
            .map(|(name, value, values)| {
                Value::Object(vec![
                    ("name".into(), Value::from((*name).to_owned())),
                    ("value".into(), Value::Number(*value)),
                    (
                        "series".into(),
                        Value::Array(
                            values
                                .iter()
                                .map(|(period, value)| {
                                    Value::Object(vec![
                                        ("period".into(), Value::from(period.clone())),
                                        ("value".into(), Value::Number(*value)),
                                    ])
                                })
                                .collect(),
                        ),
                    ),
                ])
            })
            .collect();
        let value = Value::Object(vec![
            ("app".into(), Value::from(app.to_owned())),
            ("from".into(), Value::from(from)),
            ("to".into(), Value::from(to)),
            ("periodicity".into(), Value::from(periodicity.to_owned())),
            ("panels".into(), Value::Array(json_panels)),
        ]);
        output::emit(
            &rt.d,
            &value,
            &EmitOptions {
                width: rt.d.columns.unwrap_or(120) as usize,
                ..EmitOptions::default()
            },
        )?;
    }
    Ok(())
}
fn fmt(value: f64) -> String {
    let raw = if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    };
    let (integer, fraction) = raw.split_once('.').unwrap_or((&raw, ""));
    let mut grouped = String::new();
    for (index, digit) in integer.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    let mut result: String = grouped.chars().rev().collect();
    if !fraction.is_empty() {
        result.push('.');
        result.push_str(fraction);
    }
    result
}

const SUMMARY_DESCRIPTION: &str =
    "Analytics dashboard of an app: MRR, subscriptions, viewers, conversions with trend charts";

fn dashboard_items(name: &str) -> Vec<(spec::Command, Vec<String>)> {
    let (_, ops) = spec::embedded_spec();
    spec::derive_commands(&ops)
        .into_iter()
        .filter(|c| c.resource == "dashboard")
        .map(|c| {
            let op = c.app_op.as_ref().or(c.global_op.as_ref()).unwrap();
            let mut required: Vec<String> = if c.global_op.is_some() {
                Vec::new()
            } else {
                vec!["--app".into()]
            };
            required.extend(
                op.params
                    .iter()
                    .filter(|p| {
                        p.required && p.name != "application_id" && p.name != "confirm_token"
                    })
                    .map(|p| format!("--{}", p.name.replace('_', "-"))),
            );
            let _ = name;
            (c, required)
        })
        .collect()
}

fn list_value(name: &str) -> Value {
    let mut rows = vec![Value::Object(vec![
        (
            "command".into(),
            Value::from(format!("{name} dashboard summary")),
        ),
        (
            "description".into(),
            Value::from(SUMMARY_DESCRIPTION.to_owned()),
        ),
        (
            "required_flags".into(),
            Value::Array(vec![Value::from("--app".to_owned())]),
        ),
    ])];
    rows.extend(dashboard_items(name).into_iter().map(|(c, flags)| {
        Value::Object(vec![
            (
                "command".into(),
                Value::from(format!("{name} dashboard {}", c.action)),
            ),
            ("description".into(), Value::from(c.summary)),
            (
                "required_flags".into(),
                Value::Array(flags.into_iter().map(Value::from).collect()),
            ),
        ])
    }));
    Value::Array(rows)
}

fn print_list_only(output: Option<&str>, terminal: bool) -> bool {
    output == Some("json") || !terminal
}

fn validate_menu_dry_run(dry_run: bool, app: Option<&str>) -> Result<(), CliError> {
    if dry_run && app.is_none() {
        Err(errors::usage(
            "--dry-run needs --app: the app menu sends a request.",
        ))
    } else {
        Ok(())
    }
}

fn app_menu_lines(apps: &[&Value]) -> Vec<String> {
    let width = apps
        .iter()
        .map(|a| {
            member(a, "name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .chars()
                .count()
        })
        .max()
        .unwrap_or(0);
    apps.iter()
        .map(|a| {
            let name = term::clean(
                &member(a, "name")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            );
            format!(
                "{name:<width$}  {}",
                member(a, "id").and_then(Value::as_str).unwrap_or_default()
            )
        })
        .collect()
}

fn menu_output(p: &Parsed) -> &str {
    p.opt("output").unwrap_or("table")
}

fn menu(rt: &mut Runtime, p: &Parsed) -> Result<(), CliError> {
    if print_list_only(p.opt("output"), term::menu_terminal(&rt.d)) {
        rt.d.host
            .stdout(&(json::stringify_pretty(&list_value(&rt.d.name)).unwrap_or_default() + "\n"));
        return Ok(());
    }
    validate_menu_dry_run(p.flag("dry-run"), p.opt("app"))?;
    let app = if let Some(app) = p.opt("app") {
        app.to_owned()
    } else {
        let ctx = context(rt, p)?;
        let ops = spec::derive_commands(&spec::embedded_spec().1);
        let cmd = ops
            .into_iter()
            .find(|c| c.resource == "applications" && c.action == "list")
            .ok_or_else(|| errors::usage("applications list is missing from the API spec"))?;
        let vals = BTreeMap::from([
            (
                "fields".into(),
                Value::Array(vec![Value::from("id".into()), Value::from("name".into())]),
            ),
            ("per_page".into(), Value::Number(100.0)),
        ]);
        let built = request::build_request(&cmd, &vals, &ctx.env, None, None)?;
        let response = http::send(
            &ctx,
            &HttpRequest {
                method: "GET".into(),
                url: built.url,
                path_template: built.op.path,
                body: None,
                write: Some(false),
                refuse_redirect: false,
                rate_limited: None,
            },
        )?;
        if !(200..300).contains(&response.status) {
            return Err(api_error(&response));
        }
        let body = json::parse(&response.text).unwrap_or(Value::Null);
        let apps = values(member(&body, "mobile_applications"));
        if apps.is_empty() {
            return Err(CliError::new(
                EXIT::NOT_FOUND,
                "NOT_FOUND",
                vec!["No application found for this key.".into()],
            ));
        }
        let lines = app_menu_lines(&apps);
        let selected = term::choose(&lines, "Applications", "choose", rt.d.host.io(), &mut |s| {
            rt.d.host.stderr(s)
        });
        let Some(index) = selected else { return Ok(()) };
        member(apps[index], "id")
            .and_then(Value::as_str)
            .unwrap_or_default()
    };
    let items = dashboard_items(&rt.d.name);
    let fillable = items
        .iter()
        .filter(|(_, required)| {
            required
                .iter()
                .all(|f| ["--app", "--date-range", "--periodicity"].contains(&f.as_str()))
        })
        .collect::<Vec<_>>();
    // `summary` is first, as in the JSON list; the generated dashboards follow.
    let names = std::iter::once(("summary", SUMMARY_DESCRIPTION))
        .chain(
            fillable
                .iter()
                .map(|(c, _)| (c.action.as_str(), c.summary.as_str())),
        )
        .map(|(name, description)| (format!("dashboard {name}"), description))
        .collect::<Vec<_>>();
    let width = names
        .iter()
        .map(|(n, _)| n.chars().count())
        .max()
        .unwrap_or(0);
    let lines = names
        .iter()
        .map(|(name, description)| format!("{name:<width$}  {description}"))
        .collect::<Vec<_>>();
    let selected = term::choose(&lines, "Dashboards", "run", rt.d.host.io(), &mut |s| {
        rt.d.host.stderr(s)
    });
    let Some(index) = selected else { return Ok(()) };
    let now = rt.d.host.now();
    let (from, to) = date_range(now, 30);
    let Some(index) = index.checked_sub(1) else {
        rt.d.host.stderr(&format!(
            "{} dashboard summary --app {app} --days 30 --output table{}\n",
            rt.d.name,
            command_tail(p)
        ));
        let mut args = p.clone();
        args.options.insert("app".into(), vec![app]);
        args.options.insert("days".into(), vec!["30".into()]);
        args.options.insert("output".into(), vec!["table".into()]);
        return summary(rt, &args);
    };
    let (item, required) = fillable[index];
    let ctx = context(rt, p)?;
    let dated = item.options.iter().any(|o| o.name == "date_range");
    // TS sets the periodicity only when the dashboard requires it.
    let daily = required.iter().any(|f| f == "--periodicity");
    let mut vals = BTreeMap::from([("application_id".into(), Value::from(app.clone()))]);
    if dated {
        vals.insert(
            "date_range".into(),
            Value::Array(vec![Value::from(from.clone()), Value::from(to.clone())]),
        );
    }
    if daily {
        vals.insert("periodicity".into(), Value::from("DAY".to_owned()));
    }
    let cmd = item.clone();
    let built = request::build_request(&cmd, &vals, &ctx.env, None, None)?;
    rt.d.host.stderr(&format!(
        "{} dashboard {} --app {}{}{} --output {}{}\n",
        rt.d.name,
        item.action,
        app,
        if dated {
            format!(" --date-range {from},{to}")
        } else {
            String::new()
        },
        if daily { " --periodicity DAY" } else { "" },
        menu_output(p),
        command_tail(p)
    ));
    let response = http::send(
        &ctx,
        &HttpRequest {
            method: "GET".into(),
            url: built.url,
            path_template: built.op.path,
            body: None,
            write: Some(false),
            refuse_redirect: false,
            rate_limited: None,
        },
    )?;
    if !(200..300).contains(&response.status) {
        return Err(api_error(&response));
    }
    let value = if response.text.trim().is_empty() {
        Value::Object(Vec::new())
    } else {
        json::parse(&response.text).unwrap_or(Value::Null)
    };
    output::emit(
        &rt.d,
        &value,
        &EmitOptions {
            table: p.opt("output").unwrap_or("table") == "table",
            width: rt.d.columns.unwrap_or(120) as usize,
            ..EmitOptions::default()
        },
    )?;
    Ok(())
}

fn command_tail(p: &Parsed) -> String {
    let mut tail = String::new();
    if let Some(env) = p.opt("env") {
        tail.push_str(&format!(" --env {env}"));
    }
    if let Some(reason) = p.opt("reason") {
        let quoted = json::stringify(&Value::from(reason.to_owned()), None).unwrap_or_default();
        tail.push_str(&format!(" --reason {quoted}"));
    }
    tail
}

fn api_error(response: &http::HttpResponse) -> CliError {
    let body = json::parse(&response.text).unwrap_or(Value::Null);
    let code = member(member(&body, "error").unwrap_or(&Value::Null), "code")
        .and_then(Value::as_str)
        .unwrap_or_else(|| format!("HTTP_{}", response.status));
    let messages = values(member(
        member(&body, "error").unwrap_or(&Value::Null),
        "messages",
    ))
    .into_iter()
    .filter_map(Value::as_str)
    .collect::<Vec<_>>();
    let mut error = CliError::new(
        errors::exit_for(Some(&code), response.status),
        code,
        if messages.is_empty() {
            vec![format!("HTTP {}", response.status)]
        } else {
            messages
        },
    );
    error.status = Some(response.status);
    error
}

pub fn register(program: &mut Command, rt: &mut Runtime) {
    rt.taken.insert("dashboard summary".into());
    rt.search_index.push(runtime::SearchEntry {
        command: format!("{} dashboard summary --app app_xxx --output table", rt.d.name),
        resource: "dashboard".into(),
        action: "summary".into(),
        summary: "Analytics dashboard of an app: key metrics with trend charts".into(),
        description: "Dashboard of one app in the terminal: MRR, active subscriptions, paywall unique viewers and conversions over the last days, with a sparkline chart per metric (--output table). Overview of revenue, subscriptions, conversion and stats".into(),
        method: "GET".into(),
        path: "-".into(),
        scope: "analytics.dashboards:view".into(),
    });
    let mut group = runtime::group(program, "dashboard").clone().description(
        "Analytics dashboards. Start with: dashboard summary --app <app_id> --output table",
    );
    runtime::add_globals(&mut group);
    group = group.option(crate::cli::OptionDef::value(
        "--app <app_id>",
        "application public id. Without it, a menu shows the apps of the key",
    ));
    group = group.after_help(format!(
        "\nWithout a subcommand, an interactive terminal shows a menu of the dashboards, then runs the chosen one for the last 30 days, as a table.\nThe menu leaves out the dashboards that need another flag (funnel, flow-paths): run them by name.\nWith --output json, or without a terminal, it prints the list of all the dashboards as JSON, with their required flags, and sends no request.\n\nExample:\n  {} dashboard --output json | jq -r '.[].command'\n",
        rt.d.name
    ));
    let mut summary_command = Command::new("summary").description(
        "Analytics dashboard of an app: MRR, subscriptions, viewers, conversions with trend charts",
    );
    runtime::add_globals(&mut summary_command);
    summary_command.options.push(crate::cli::OptionDef::value(
        "--app <app_id>",
        "application public id (required)",
    ));
    summary_command.options.push(
        crate::cli::OptionDef::value("--days <n>", "number of days up to today, from 2 to 730")
            .default("30"),
    );
    summary_command = summary_command.after_help(format!(
        "\nShows the MRR, the active subscriptions, the paywall unique viewers and the conversions of one app.\nThe default output is JSON. --output table draws one line per metric with a sparkline.\nMRR is today's value, in the dashboard currency of the app. Active subscriptions is the current base.\nViewers and conversions are the total of the range.\nIt sends 3 requests (4 when --days is above 60).\n\nExample:\n  {} dashboard summary --app app_xxx --output table\n  {} dashboard summary --app app_xxx --days 90 --output table\n",
        rt.d.name, rt.d.name
    ));
    group
        .commands
        .push(summary_command.action(|p, rt| summary(rt, p)));
    // TS `.helpCommand(true)`: a group with an action loses the implicit `help summary`.
    group = group.action(|p, rt| menu(rt, p)).help_command(true);
    if let Some(existing) = program.commands.iter_mut().find(|c| c.name == "dashboard") {
        *existing = group;
    } else {
        program.commands.push(group);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sparkline_scales_values_and_keeps_flat_series_low() {
        assert_eq!(super::sparkline(&[1.0, 2.0, 3.0]), "▁▄▇");
        assert_eq!(super::sparkline(&[7.0, 7.0]), "▁▁");
    }

    #[test]
    fn dashboard_dates_are_computed_in_utc() {
        assert_eq!(
            super::date_range(1_700_000_000_000, 30),
            ("2023-10-16".into(), "2023-11-14".into())
        );
    }

    #[test]
    fn dashboard_menu_list_has_summary_and_all_generated_actions() {
        let list = super::list_value("acme");
        let rows = super::values(Some(&list));
        assert_eq!(rows.len(), 23);
        assert_eq!(
            super::member(rows[0], "command")
                .and_then(crate::core::json::Value::as_str)
                .as_deref(),
            Some("acme dashboard summary")
        );
        assert!(rows.iter().any(|row| {
            super::member(row, "command")
                .and_then(crate::core::json::Value::as_str)
                .as_deref()
                == Some("acme dashboard header-live")
        }));
    }

    #[test]
    fn dashboard_menu_required_flags_keep_global_operations_optional_for_app() {
        let (_, flags) = super::dashboard_items("purchasely")
            .into_iter()
            .find(|(item, _)| item.action == "mau-reports")
            .unwrap();
        assert!(flags.is_empty());
        let (_, flags) = super::dashboard_items("purchasely")
            .into_iter()
            .find(|(item, _)| item.action == "conversion")
            .unwrap();
        assert_eq!(flags, vec!["--app", "--date-range"]);
    }

    #[test]
    fn dashboard_menu_hides_dashboards_that_need_an_unfilled_flag() {
        let fillable = super::dashboard_items("purchasely")
            .into_iter()
            .filter(|(_, flags)| {
                flags
                    .iter()
                    .all(|f| ["--app", "--date-range", "--periodicity"].contains(&f.as_str()))
            })
            .map(|(c, _)| c.action)
            .collect::<Vec<_>>();
        assert!(!fillable.contains(&"funnel".into()));
        assert!(!fillable.contains(&"flow-paths".into()));
        assert!(fillable.contains(&"conversion".into()));
    }

    #[test]
    fn old_dashboard_v2_names_are_not_created_as_a_separate_resource() {
        let names = super::dashboard_items("purchasely")
            .into_iter()
            .map(|(item, _)| item.action)
            .collect::<Vec<_>>();
        assert_eq!(names.len(), 22);
        assert_eq!(
            names
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            names.len()
        );
        // dashboard-one.test.ts "holds the actions of the spec tag Dashboard V2 next to the others".
        for v2 in [
            "conversion",
            "mau-reports",
            "paid-subscription-movements",
            "paid-subscription-retention",
            "paywall-latency",
            "sdk-evolution",
            "sdk-versions",
            "subscription-activations",
            "subscription-evolution",
            "subscription-reactivations",
            "header-live",
        ] {
            assert!(names.contains(&v2.into()), "{v2}");
        }
        assert!(names.iter().all(|n| !n.ends_with("-dashboard")));
        let (_, ops) = crate::management::spec::embedded_spec();
        assert!(
            crate::management::spec::derive_commands(&ops)
                .iter()
                .all(|c| c.resource != "dashboard-v2")
        );
    }

    /// dashboard.test.ts "never draws a full block, so the rows of a table keep a gap".
    #[test]
    fn sparkline_never_draws_a_full_block() {
        assert!(!super::sparkline(&[1.0, 5.0, 9.0, 2.0, 9.0]).contains('█'));
        assert_eq!(super::sparkline(&[0.0, 100.0]), "▁▇");
    }

    #[test]
    fn dashboard_menu_quit_returns_no_selection_and_restores_without_a_message() {
        let io = FakeIo {
            lines: std::cell::RefCell::new(Vec::new()),
            selected: None,
        };
        let mut said = Vec::new();
        assert_eq!(
            crate::games::term::choose(
                &["summary".into()],
                "Dashboards",
                "run",
                Some(&io),
                &mut |s: &str| said.push(s.to_owned())
            ),
            None
        );
        assert!(said.is_empty());
    }

    #[test]
    fn dashboard_menu_strips_control_codes_and_cuts_to_terminal_width() {
        let io = FakeIo::selected(0);
        let mut said = Vec::new();
        crate::games::term::choose(
            &["Bad\x1b[2J application name".into()],
            "Applications",
            "choose",
            Some(&io),
            &mut |s: &str| said.push(s.to_owned()),
        );
        let display = crate::games::term::strip_ansi(&io.lines.borrow().join("\n"));
        assert!(display.contains("B..."));
        assert!(!display.contains("\x1b[2J"));
    }

    #[test]
    fn equal_menu_labels_keep_the_selected_index() {
        let io = FakeIo::selected(1);
        let mut said = Vec::new();
        assert_eq!(
            crate::games::term::choose(
                &["same".into(), "same".into()],
                "Dashboards",
                "run",
                Some(&io),
                &mut |s: &str| said.push(s.to_owned())
            ),
            Some(1)
        );
    }

    #[test]
    fn non_terminal_dashboard_list_is_json_with_summary_first() {
        assert!(super::print_list_only(None, false));
        let list = super::list_value("purchasely");
        let rows = super::values(Some(&list));
        assert_eq!(
            super::member(rows[0], "command").and_then(crate::core::json::Value::as_str),
            Some("purchasely dashboard summary".into())
        );
    }

    #[test]
    fn json_output_skips_the_menu_even_when_both_streams_are_tty() {
        assert!(super::print_list_only(Some("json"), true));
        assert!(!super::print_list_only(Some("table"), true));
    }

    #[test]
    fn dashboard_rows_use_the_callers_program_name() {
        let list = super::list_value("acme");
        let rows = super::values(Some(&list));
        assert_eq!(
            super::member(rows[0], "command").and_then(crate::core::json::Value::as_str),
            Some("acme dashboard summary".into())
        );
    }

    #[test]
    fn mau_dashboard_requires_no_date_range() {
        let (item, flags) = super::dashboard_items("purchasely")
            .into_iter()
            .find(|(item, _)| item.action == "mau-reports")
            .unwrap();
        assert!(flags.is_empty());
        assert!(
            !item
                .options
                .iter()
                .any(|option| option.name == "date_range")
        );
    }

    #[test]
    fn app_ids_stay_aligned_and_control_codes_are_cleaned() {
        let apps = [
            crate::core::json::parse(r#"{"name":"Short","id":"app_1"}"#).unwrap(),
            crate::core::json::parse(r#"{"name":"Longer\u001b[2J name","id":"app_2"}"#).unwrap(),
        ];
        let refs = apps.iter().collect::<Vec<_>>();
        let lines = super::app_menu_lines(&refs);
        assert_eq!(lines[0].find("app_1"), lines[1].find("app_2"));
        assert!(!lines[1].contains('\x1b'));
    }

    #[test]
    fn menu_dry_run_without_app_is_refused_before_io() {
        let error = super::validate_menu_dry_run(true, None).unwrap_err();
        assert_eq!(error.exit, crate::core::errors::EXIT::USAGE);
        assert!(error.message().contains("needs --app"));
        assert!(super::validate_menu_dry_run(true, Some("app_1")).is_ok());
    }

    #[test]
    fn interactive_menu_defaults_the_generated_command_to_table_output() {
        assert_eq!(super::menu_output(&crate::cli::Parsed::default()), "table");
    }

    #[test]
    fn interactive_command_preview_preserves_environment_and_quotes_reason() {
        let parsed = crate::cli::Parsed {
            options: std::collections::BTreeMap::from([
                ("env".into(), vec!["staging".into()]),
                ("reason".into(), vec!["look at it".into()]),
            ]),
            ..Default::default()
        };
        assert_eq!(
            super::command_tail(&parsed),
            " --env staging --reason \"look at it\""
        );
    }

    #[test]
    fn resize_closes_the_menu_with_a_message() {
        struct ResizeIo;
        impl crate::core::deps::Io for ResizeIo {
            fn play(&self, _: &mut dyn crate::games::term::Game) -> Option<Result<String, String>> {
                Some(Ok("The terminal changed size.".into()))
            }
        }
        let mut said = Vec::new();
        let result = crate::games::term::choose(
            &["one".into()],
            "Dashboards",
            "run",
            Some(&ResizeIo),
            &mut |message: &str| said.push(message.to_owned()),
        );
        assert_eq!(result, None);
        assert_eq!(said, vec!["The terminal changed size.\n"]);
    }

    #[test]
    fn menu_scroll_can_select_the_last_dashboard() {
        // dashboard-menu.test.ts "the menu scrolls: every entry fits the screen and the last one can be reached".
        let mut scrolled = crate::games::term::Menu::new(
            (0..30).map(|n| format!("entry-{n}")).collect(),
            "Dashboards",
            "run",
        );
        for _ in 0..29 {
            scrolled.key("down");
        }
        let plain = scrolled
            .draw(100, 20)
            .iter()
            .map(|l| crate::games::term::strip_ansi(l))
            .collect::<Vec<_>>();
        assert!(plain.len() <= 20);
        let text = plain.join("\n");
        assert!(text.contains("  P entry-29 "), "{text}");
        assert!(text.contains("Dashboards"));
        assert!(text.contains("Enter run"));
        assert!(!text.contains("entry-0 "));
        let mut menu = crate::games::term::Menu::new(
            (0..40).map(|n| format!("dashboard {n}")).collect(),
            "Dashboards",
            "run",
        );
        for _ in 0..39 {
            menu.key("down");
        }
        menu.key("return");
        assert_eq!(menu.chosen(), Some(39));
    }

    #[test]
    fn empty_menu_selection_does_not_invent_an_app() {
        let io = FakeIo {
            lines: std::cell::RefCell::new(Vec::new()),
            selected: None,
        };
        let mut said = Vec::new();
        let selected =
            crate::games::term::choose(&[], "Applications", "choose", Some(&io), &mut |s: &str| {
                said.push(s.to_owned())
            });
        assert_eq!(selected, None);
        assert!(said.is_empty());
    }

    #[test]
    fn choose_cleans_and_cuts_menu_lines_before_playback() {
        let io = FakeIo::selected(1);
        let mut said = Vec::new();
        let result = crate::games::term::choose(
            &["first\nline".into(), "second".into()],
            "Dashboards",
            "run",
            Some(&io),
            &mut |message| said.push(message.to_owned()),
        );
        assert_eq!(result, Some(1));
        let display = crate::games::term::strip_ansi(&io.lines.borrow().join("\n"));
        assert!(display.contains("f..."));
        assert!(display.contains("s..."));
        assert!(said.is_empty());
    }

    struct FakeIo {
        lines: std::cell::RefCell<Vec<String>>,
        selected: Option<usize>,
    }

    impl FakeIo {
        fn selected(selected: usize) -> Self {
            Self {
                lines: std::cell::RefCell::new(Vec::new()),
                selected: Some(selected),
            }
        }
    }

    impl crate::core::deps::Io for FakeIo {
        fn play(&self, game: &mut dyn crate::games::term::Game) -> Option<Result<String, String>> {
            let rows = game.draw();
            self.lines.borrow_mut().extend(rows);
            if let Some(selected) = self.selected {
                for _ in 0..selected {
                    game.key("down");
                }
                game.key("return");
                Some(Ok(String::new()))
            } else {
                Some(Ok("Bye.".into()))
            }
        }
        fn dimensions(&self) -> (usize, usize) {
            (12, 24)
        }
    }
}
