//! Owner: P8
//! Sample Management API data for `PURCHASELY_DEMO=1`.
use super::{
    deps::Deps,
    errors::{CliError, EXIT},
    http::HttpRequest,
    json::{self, Value},
    time,
};

pub const DEMO_APP: &str = "app_demo";

pub fn enabled(d: &Deps) -> bool {
    d.env.get("PURCHASELY_DEMO").is_some_and(|v| v == "1")
}

pub fn write_error() -> CliError {
    CliError::new(EXIT::REFUSED, "REFUSED", vec!["Demo mode (PURCHASELY_DEMO=1) never sends a write, so the CLI sent nothing. --dry-run and --plan still show the request.".into()])
}

fn parsed(s: &str) -> Value {
    json::parse(s).unwrap_or(Value::Null)
}

fn with_stamps(mut v: Value) -> Value {
    if let Value::Object(ref mut fields) = v {
        fields.push((
            "created_at".into(),
            Value::from("2026-01-12T09:30:00Z".into()),
        ));
        fields.push((
            "updated_at".into(),
            Value::from("2026-09-28T14:05:00Z".into()),
        ));
    }
    v
}

fn stamped_list(mut root: Value) -> Value {
    if let Value::Object(ref mut fields) = root
        && let Some((_, Value::Array(items))) = fields.first_mut()
    {
        for item in items.iter_mut() {
            *item = with_stamps(item.clone());
        }
    }
    root
}

fn application() -> Value {
    with_stamps(parsed(
        r#"{"id":"app_demo","name":"Sample Meditation App (demo)","client_id":"cli_demo","default_language":"en","default_dashboard_currency":"USD","default_integration_currency":"USD","default_screen_id":"pres_demo_onboarding","redeem_presentation_id":null,"icon_url":null,"show_purchasely":true,"has_mocked_data":true,"client_webhook_url":null,"s2s_notifications_endpoints":[],"ios_platform":{"bundle_id":"com.example.meditation.demo"},"android_platform":{"package_name":"com.example.meditation.demo"},"stripe_platform":null,"platforms_configured":{"ios":true,"android":true,"stripe":false}}"#,
    ))
}

fn sample_lists() -> Vec<(&'static str, Value)> {
    let screens = parsed(
        r#"{"screens":[{"id":"pres_demo_onboarding","name":"Onboarding paywall","vendor_id":"onboarding_paywall","status":"published","is_default":true,"template_identifier":"full_screen","category_tag_ids":[],"web_enabled":false,"web_slug":null,"web_slug_sandbox":null},{"id":"pres_demo_upsell","name":"Home upsell","vendor_id":"home_upsell","status":"published","is_default":false,"template_identifier":"bottom_sheet","category_tag_ids":[],"web_enabled":false,"web_slug":null,"web_slug_sandbox":null},{"id":"pres_demo_winback","name":"Winback offer","vendor_id":"winback_offer","status":"draft","is_default":false,"template_identifier":"full_screen","category_tag_ids":[],"web_enabled":false,"web_slug":null,"web_slug_sandbox":null}]}"#,
    );
    let products = parsed(
        r#"{"products":[{"id":"prd_demo_premium","name":"Premium","vendor_id":"premium","description":"All the guided sessions","icon_url":null,"default_screen_id":"pres_demo_onboarding","application_id":"app_demo"}]}"#,
    );
    let campaigns = parsed(
        r#"{"campaigns":[{"id":"cmp_demo_winback","name":"Winback after a cancellation","vendor_id":"winback","status":"running","position":1,"application_id":"app_demo","audience_id":"aud_demo_churned","ab_test_id":null,"presentable_id":"pres_demo_winback","placement_ids":["plc_demo_home"],"category_tag_ids":[],"config":{},"starts_at":"2026-09-01T00:00:00Z","ends_at":null}]}"#,
    );
    let placements = parsed(
        r#"{"placements":[{"id":"plc_demo_onboarding","vendor_id":"onboarding","name":"Onboarding","description":"Shown at the end of the first launch","is_default":true,"has_running_ab_test":false,"default_presentable_id":"pres_demo_onboarding","category_tag_ids":[],"audiences":[],"active_campaigns":[]},{"id":"plc_demo_home","vendor_id":"home","name":"Home banner","description":"The upgrade button of the home screen","is_default":false,"has_running_ab_test":false,"default_presentable_id":"pres_demo_upsell","category_tag_ids":[],"audiences":[{"id":"aud_demo_churned","is_in_ab_test":false,"priority":1,"presentable_id":"pres_demo_winback"}],"active_campaigns":[{"id":"cmp_demo_winback","name":"Winback after a cancellation","audience_id":"aud_demo_churned","presentable_id":"pres_demo_winback","starts_at":"2026-09-01T00:00:00Z","ends_at":null,"position":1,"is_in_ab_test":false,"ab_test_id":null,"presentation_views":null,"unique_views":null}]},{"id":"plc_demo_settings","vendor_id":"settings","name":"Settings","description":"The subscription row of the settings","is_default":false,"has_running_ab_test":false,"default_presentable_id":"pres_demo_upsell","category_tag_ids":[],"audiences":[],"active_campaigns":[] }]}"#,
    );
    vec![
        ("screens", stamped_list(screens)),
        ("products", stamped_list(products)),
        ("campaigns", stamped_list(campaigns)),
        ("placements", stamped_list(placements)),
    ]
}

fn query(url: &str, name: &str) -> Vec<String> {
    let Some((_, search)) = url.split_once('?') else {
        return vec![];
    };
    search
        .split('&')
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (decode(key.replace('+', " ").as_bytes()) == name)
                .then(|| decode(value.replace('+', " ").as_bytes()))
        })
        .collect()
}

fn decode(bytes: &[u8]) -> String {
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(n) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
        {
            out.push(n);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn path(url: &str) -> &str {
    let path = url.split_once("://").map_or(url, |(_, rest)| rest);
    let path = path.find('/').map_or("/", |i| &path[i..]);
    path.split('?').next().unwrap_or(path)
}

/// Answer a generated Management read from the fictional app. Dynamic dashboard dates use the host wall clock.
pub fn response(req: &HttpRequest, name: &str, wall_now: i64) -> (u16, Value) {
    let endpoint = path(&req.url)
        .strip_prefix("/management/v1")
        .unwrap_or(path(&req.url));
    let lists = sample_lists();
    let mut body = None;
    if req.method == "GET" && endpoint == "/applications" {
        body = Some(Value::Object(vec![(
            "mobile_applications".into(),
            Value::Array(vec![application()]),
        )]));
    } else if req.method == "GET" {
        if endpoint == "/dashboard_v2/mau_reports" {
            let rows = last_months(wall_now, 10);
            let users: Vec<_> = rows
                .iter()
                .enumerate()
                .map(|(i, period)| {
                    let value = (15_000.0 + 400.0 * i as f64 + 800.0 * (i as f64).sin()).round();
                    format!("{{\"period\":\"{period}\",\"value\":{value}}}")
                })
                .collect();
            let starts: Vec<_> = rows
                .iter()
                .enumerate()
                .map(|(i, period)| {
                    let value = ((15_000.0 + 400.0 * i as f64 + 800.0 * (i as f64).sin()).round()
                        as i64)
                        * 8;
                    format!("{{\"period\":\"{period}\",\"value\":{value}}}")
                })
                .collect();
            body = Some(parsed(&format!(
                "{{\"mau_reports\":{{\"data\":{{\"app_starts\":[{}],\"unique_users\":[{}]}}}}}}",
                starts.join(","),
                users.join(",")
            )));
        } else if endpoint == "/dashboard_v2/paywall_latency" {
            let n = query(&req.url, "period")
                .first()
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(30)
                .clamp(7, 90) as usize;
            let dates: Vec<_> = (0..n)
                .map(|i| {
                    format!(
                        "\"{}\"",
                        time::iso_date(wall_now - (n - 1 - i) as i64 * 86_400_000)
                    )
                })
                .collect();
            let series = |base: f64| {
                (0..n)
                    .map(|i| {
                        let day = (wall_now.div_euclid(86_400_000) - (n - 1 - i) as i64) as f64;
                        (base + base / 10.0 * (day / 1.1).sin()).round().max(0.0) as i64
                    })
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            };
            body = Some(parsed(&format!(
                "{{\"paywall_latency\":{{\"data\":{{\"dates\":[{}],\"p50\":[{}],\"p95\":[{}],\"p99\":[{}],\"request_count\":[{}],\"countries\":[\"US\",\"FR\",\"GB\"]}}}}}}",
                dates.join(","),
                series(120.0),
                series(340.0),
                series(780.0),
                series(2400.0)
            )));
        }
        if let Some((key, value)) = lists.iter().find(|(key, _)| endpoint == format!("/{key}")) {
            let _ = key;
            body = Some(value.clone());
        }
        let prefix = format!("/applications/{DEMO_APP}");
        if endpoint == prefix {
            body = Some(Value::Object(vec![(
                "mobile_application".into(),
                application(),
            )]));
        }
        if let Some(rest) = endpoint.strip_prefix(&format!("{prefix}/")) {
            if let Some((_, value)) = lists.iter().find(|(key, _)| rest == *key) {
                body = Some(value.clone());
            }
            if let Some(value) = dashboard(rest, &req.url, wall_now) {
                body = Some(value);
            }
            if rest == "plans"
                && query(&req.url, "product_id")
                    .first()
                    .is_some_and(|v| v == "prd_demo_premium")
            {
                body = Some(parsed(
                    r#"{"plans":[{"id":"pln_demo_monthly","name":"Premium monthly","vendor_id":"premium_monthly","product_type":"RENEWING_SUBSCRIPTION","level":1,"period_type":"normal","commitment_billing_type":null,"promo_offers":[],"internal_notes":null,"default_screen_id":null,"product_id":"prd_demo_premium","application_id":"app_demo","stores":{"app_store":{"platform_name":"ios","store_product_id":"com.example.meditation.demo.monthly"},"play_store":{"platform_name":"android","store_product_id":"com.example.meditation.demo.monthly"},"stripe":[]},"created_at":"2026-01-12T09:30:00Z","updated_at":"2026-09-28T14:05:00Z"},{"id":"pln_demo_yearly","name":"Premium yearly","vendor_id":"premium_yearly","product_type":"RENEWING_SUBSCRIPTION","level":2,"period_type":"normal","commitment_billing_type":null,"promo_offers":[{"id":"off_demo_trial","name":"7-day free trial","vendor_id":"yearly_trial","appstore_offer_id":null,"playstore_offer_id":"trial-7d","stripe_trial_days":null,"stripe_coupon_id":null,"commitment_billing_type":null}],"internal_notes":null,"default_screen_id":null,"product_id":"prd_demo_premium","application_id":"app_demo","stores":{"app_store":{"platform_name":"ios","store_product_id":"com.example.meditation.demo.yearly"},"play_store":{"platform_name":"android","store_product_id":"com.example.meditation.demo.yearly"},"stripe":[]},"created_at":"2026-01-12T09:30:00Z","updated_at":"2026-09-28T14:05:00Z"}]}"#,
                ));
            }
            if let Some((plural, value)) = lists
                .iter()
                .find(|(key, _)| rest.starts_with(&format!("{key}/")))
            {
                let target = decode(rest.split_once('/').unwrap().1.as_bytes());
                if let Value::Object(roots) = value
                    && let Some((_, Value::Array(items))) = roots.first()
                    && let Some(item) = items.iter().find(|item| {
                        field(item, "id") == target || field(item, "vendor_id") == target
                    })
                {
                    let singular = plural.trim_end_matches('s');
                    body = Some(Value::Object(vec![(singular.into(), item.clone())]));
                }
            }
        }
    }
    if let Some(value) = body {
        return (200, value);
    }
    let msg = format!(
        "Demo mode (PURCHASELY_DEMO=1) has no sample data for {} {}. The sample app is {DEMO_APP}. Run `{name} demo` for the commands to try.",
        req.method, req.path_template
    );
    (
        404,
        Value::Object(vec![(
            "error".into(),
            Value::Object(vec![
                ("code".into(), Value::from("NOT_FOUND".into())),
                ("messages".into(), Value::Array(vec![Value::from(msg)])),
            ]),
        )]),
    )
}

fn field(v: &Value, key: &str) -> String {
    if let Value::Object(items) = v {
        items
            .iter()
            .find(|(k, _)| k == key)
            .and_then(|(_, v)| v.as_str())
            .unwrap_or_default()
    } else {
        String::new()
    }
}

#[derive(Clone)]
struct Bucket {
    day: i64,
    days: i64,
    period: String,
}

fn obj(fields: Vec<(&str, Value)>) -> Value {
    Value::Object(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}

fn text(v: impl Into<String>) -> Value {
    Value::from(v.into())
}

fn number(v: f64) -> Value {
    Value::Number(v)
}

fn number_field(v: &Value, key: &str) -> f64 {
    if let Value::Object(fields) = v {
        fields
            .iter()
            .find_map(|(k, value)| match value {
                Value::Number(n) if k == key => Some(*n),
                _ => None,
            })
            .unwrap_or(0.0)
    } else {
        0.0
    }
}

fn array(v: Vec<Value>) -> Value {
    Value::Array(v)
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - if month <= 2 { 1 } else { 0 };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

fn day_index(date: &str) -> Option<i64> {
    let year = date.get(..4)?.parse::<i64>().ok()?;
    let month = date.get(5..7)?.parse::<i64>().ok()?;
    let day = date.get(8..10)?.parse::<i64>().ok()?;
    Some(days_from_civil(year, month, day))
}

fn buckets(url: &str, default: &str) -> Vec<Bucket> {
    let dates = query(url, "date_range[]");
    if dates.len() != 2 {
        return vec![];
    }
    let (Some(from), Some(to)) = (day_index(&dates[0]), day_index(&dates[1])) else {
        return vec![];
    };
    let period = query(url, "periodicity")
        .first()
        .cloned()
        .unwrap_or_else(|| default.to_owned());
    let step = match period.as_str() {
        "WEEK" => 7,
        "MONTH" => 30,
        _ => 1,
    };
    if from > to {
        return vec![];
    }
    let mut out = Vec::new();
    let mut day = from;
    while day <= to {
        out.push(Bucket {
            day,
            days: step.min(to - day + 1),
            period: time::iso_date(day * 86_400_000),
        });
        day += step;
    }
    out
}

fn wave(day: i64, base: f64, growth: f64, swing: f64) -> f64 {
    (base + growth * (day as f64 - 20_000.0) + swing * (day as f64 / 1.1).sin())
        .round()
        .max(0.0)
}

fn total(b: &Bucket, base: f64, swing: f64) -> f64 {
    (0..b.days).map(|i| wave(b.day + i, base, 0.0, swing)).sum()
}

fn row(b: &Bucket, rest: Vec<(&str, Value)>) -> Value {
    let mut fields = vec![("period".into(), text(b.period.clone()))];
    fields.extend(rest.into_iter().map(|(k, v)| (k.into(), v)));
    Value::Object(fields)
}

fn values(url: &str, default: &str, mut f: impl FnMut(&Bucket) -> Value) -> Value {
    array(buckets(url, default).iter().map(&mut f).collect())
}

fn active(day: i64) -> f64 {
    wave(day, 2_900.0, 2.0, 20.0)
}

fn split(n: f64, shares: &[(&str, f64)]) -> Value {
    obj(shares
        .iter()
        .map(|(k, share)| (*k, number((n * share).round())))
        .collect())
}

fn revenue(url: &str, base: f64, plans: &[(&str, f64)]) -> Value {
    let stores = [("google", 0.4), ("apple", 0.5), ("stripe", 0.1)];
    let countries = [
        ("United States", 0.45),
        ("France", 0.25),
        ("United Kingdom", 0.2),
        ("Germany", 0.1),
    ];
    let series = |shares: &[(&str, f64)]| {
        values(url, "MONTH", |b| {
            let mut fields = vec![("period".into(), text(b.period.clone()))];
            if let Value::Object(xs) = split(total(b, base, 150.0), shares) {
                fields.extend(xs);
            }
            Value::Object(fields)
        })
    };
    obj(vec![(
        "data",
        obj(vec![
            ("revenue", obj(vec![("values", series(&stores))])),
            (
                "revenue_by_country",
                obj(vec![("values", series(&countries))]),
            ),
            ("revenue_by_plan", obj(vec![("values", series(plans))])),
        ]),
    )])
}

fn flow(url: &str, outer: &str, count: &str, base: f64) -> Value {
    obj(vec![(
        outer,
        values(url, "MONTH", |b| {
            row(
                b,
                vec![
                    ("group_by", Value::Null),
                    ("group_by_public_id", Value::Null),
                    ("group_by_name", Value::Null),
                    (count, number(total(b, base, 5.0))),
                ],
            )
        }),
    )])
}

fn card(category: &str, fields: &[(&str, f64)]) -> Value {
    let sum: f64 = fields.iter().map(|(_, v)| *v).sum();
    let title = format!(
        "{}{}",
        category[..1].to_ascii_uppercase(),
        category[1..].replace('_', " ")
    );
    let data = fields
        .iter()
        .map(|(key, value)| {
            let color =
                if key.contains("not_renewing") || key.contains("grace") || key.contains("retry") {
                    "orange"
                } else if key.contains("renewing")
                    || key.contains("paused")
                    || key.contains("subscribed")
                    || key.contains("renewed")
                {
                    "green"
                } else {
                    "red"
                };
            obj(vec![
                ("title_key", text(*key)),
                ("subcategory", text(*key)),
                ("val", number(*value)),
                (
                    "percentage",
                    number(if sum == 0.0 {
                        0.0
                    } else {
                        ((*value / sum) * 1000.0).round() / 10.0
                    }),
                ),
                ("color", text(color)),
                ("with_link", Value::Bool(false)),
                ("downloadable", Value::Bool(false)),
            ])
        })
        .collect();
    obj(vec![
        ("title", text(title)),
        ("category", text(category)),
        ("total", text(format!("{sum} - 100.0%"))),
        ("downloadable", Value::Bool(false)),
        ("with_link", Value::Bool(false)),
        ("data", array(data)),
    ])
}

fn dashboard(path: &str, url: &str, wall_now: i64) -> Option<Value> {
    Some(match path {
        "dashboard/mrr_evolution_live" => obj(vec![(
            "mrr_evolution_live",
            obj(vec![(
                "data",
                obj(vec![
                    (
                        "mrr",
                        obj(vec![(
                            "values",
                            obj(vec![
                                (
                                    "by_day",
                                    values(url, "DAY", |b| {
                                        row(
                                            b,
                                            vec![
                                                ("date_kinds", array(vec![text("date_period")])),
                                                (
                                                    "value",
                                                    number(wave(b.day, 24_000.0, 18.0, 150.0)),
                                                ),
                                            ],
                                        )
                                    }),
                                ),
                                ("by_month", array(vec![])),
                            ]),
                        )]),
                    ),
                    ("variations", array(vec![])),
                ]),
            )]),
        )]),
        "dashboard_v2/subscription_evolution" => obj(vec![(
            "subscription_evolution",
            values(url, "DAY", |b| {
                row(
                    b,
                    vec![("group_by", Value::Null), ("count", number(active(b.day)))],
                )
            }),
        )]),
        "dashboard/composition_live" => parsed(
            r#"{"composition_live":{"data":{"composition_plan":{"values":[{"name":"Premium monthly","value":2600},{"name":"Premium yearly","value":1800}]},"composition_platform":{"values":[{"name":"ios","value":2200},{"name":"android","value":1900},{"name":"stripe","value":300}]},"composition_product":{"values":[{"name":"Premium","value":4400}]},"subscription_state":{"active":3700,"active_intro":60,"active_trial":240,"canceled_intro":10,"canceled_trial":90,"churners":300}}}}"#,
        ),
        "dashboard/events" => {
            let events = values(url, "MONTH", |b| {
                row(
                    b,
                    vec![
                        ("PRESENTATION_VIEWED", number(total(b, 850.0, 120.0))),
                        ("SUBSCRIPTION_STARTED", number(total(b, 30.0, 6.0))),
                    ],
                )
            });
            let mut map = Vec::new();
            if let Value::Array(rows) = events {
                for r in rows {
                    let p = field(&r, "period");
                    map.push((p, r));
                }
            }
            obj(vec![("events", obj(vec![("data", Value::Object(map))]))])
        }
        "dashboard/flow_paths" => parsed(
            r#"{"flow_paths":{"data":{"steps":[{"screen_id":"pres_demo_onboarding","views":700,"next":[{"screen_id":"pres_demo_upsell","count":260}]},{"screen_id":"pres_demo_upsell","views":260,"next":[]}]}}}"#,
        ),
        "dashboard/funnel" => {
            let n = buckets(url, "DAY").len() as f64;
            let dimension = query(url, "dimension")
                .first()
                .cloned()
                .unwrap_or_else(|| "platform".into());
            let groups: &[&str] = match dimension.as_str() {
                "country" => &["US", "FR", "GB"],
                "placement" => &["plc_demo_onboarding", "plc_demo_home", "plc_demo_settings"],
                "plan" => &["pln_demo_monthly", "pln_demo_yearly", "pln_demo_monthly"],
                "screen" => &[
                    "pres_demo_onboarding",
                    "pres_demo_upsell",
                    "pres_demo_winback",
                ],
                _ => &["IOS", "ANDROID", "STRIPE"],
            };
            let shares = [0.5, 0.4, 0.1];
            let rows = groups
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    let c = |v: f64| number((v * n * shares[i]).round());
                    obj(vec![
                        ("name", text(*name)),
                        (
                            "results",
                            obj(vec![
                                ("subscription_started_count", c(25.0)),
                                ("subscription_terminated_count", c(3.0)),
                                ("trial_started_count", c(12.0)),
                                ("trial_converted_count", c(9.0)),
                                ("trial_not_converted_count", c(3.0)),
                                ("first_renewals_count", c(20.0)),
                            ]),
                        ),
                    ])
                })
                .collect();
            obj(vec![("funnel", obj(vec![("data", array(rows))]))])
        }
        "dashboard/header_live" => {
            let active = card(
                "active_subscriptions_statuses",
                &[
                    ("active_renewing", 3500.0),
                    ("active_not_renewing", 420.0),
                    ("active_grace_period", 60.0),
                    ("trial_renewing", 180.0),
                    ("trial_not_renewing", 70.0),
                    ("trial_grace_period", 5.0),
                    ("intro_period_renewing", 50.0),
                    ("intro_period_not_renewing", 10.0),
                    ("intro_period_grace_period", 0.0),
                    ("promo_code_renewing", 80.0),
                    ("promo_code_not_renewing", 15.0),
                    ("promo_code_grace_period", 0.0),
                    ("promotional_offer_renewing", 10.0),
                    ("promotional_offer_not_renewing", 0.0),
                    ("promotional_offer_grace_period", 0.0),
                ],
            );
            let inactive = card(
                "inactive_subscriptions_causes",
                &[
                    ("terminated", 1900.0),
                    ("refunded", 40.0),
                    ("unpaid", 25.0),
                    ("paused", 12.0),
                    ("billing_retry", 90.0),
                    ("fraud_attemps_blocked", 0.0),
                ],
            );
            let selected = query(url, "category").first().cloned();
            let data = match selected.as_deref() {
                Some("active_subscriptions_statuses") => vec![active],
                Some("inactive_subscriptions_causes") => vec![inactive],
                Some(_) => vec![],
                None => vec![active, inactive],
            };
            obj(vec![("header_live", obj(vec![("data", array(data))]))])
        }
        "dashboard/header_today" => {
            let mut today = card(
                "today",
                &[
                    ("subscribed_with_free_trial", 9.0),
                    ("subscribed_with_intro", 2.0),
                    ("subscribed_with_promo_code", 1.0),
                    ("subscribed_with_promotional_offer", 0.0),
                    ("subscribed_paying", 18.0),
                    ("renewed", 95.0),
                    ("entered_grace_period", 3.0),
                    ("churned", 14.0),
                ],
            );
            if let Value::Object(ref mut f) = today
                && let Some((_, value)) = f.iter_mut().find(|(k, _)| k == "title")
            {
                *value = text("Today");
            }
            obj(vec![(
                "header_today",
                obj(vec![("data", array(vec![today]))]),
            )])
        }
        "dashboard/one_time_purchases_revenues" => obj(vec![(
            "one_time_purchases_revenues",
            revenue(url, 40.0, &[("200 coins", 0.7), ("1000 coins", 0.3)]),
        )]),
        "dashboard/subscriptions_revenues" => obj(vec![(
            "subscriptions_revenues",
            revenue(
                url,
                1200.0,
                &[("Premium monthly", 0.6), ("Premium yearly", 0.4)],
            ),
        )]),
        "dashboard/one_time_purchases_stats" => {
            let stores = values(url, "MONTH", |b| {
                let mut fields = vec![("period".into(), text(b.period.clone()))];
                if let Value::Object(shares) = split(
                    total(b, 3.0, 1.0),
                    &[("google", 0.4), ("apple", 0.5), ("stripe", 0.1)],
                ) {
                    fields.extend(shares);
                }
                Value::Object(fields)
            });
            let plans = values(url, "MONTH", |b| {
                let mut fields = vec![("period".into(), text(b.period.clone()))];
                if let Value::Object(shares) = split(
                    total(b, 3.0, 1.0),
                    &[("200 coins", 0.7), ("1000 coins", 0.3)],
                ) {
                    fields.extend(shares);
                }
                Value::Object(fields)
            });
            let refunds = values(url, "MONTH", |_| obj(vec![("percent", number(1.5))]));
            obj(vec![(
                "one_time_purchases_stats",
                obj(vec![(
                    "data",
                    obj(vec![
                        ("otp_by_store", obj(vec![("values", stores)])),
                        ("otp_by_plan", obj(vec![("values", plans)])),
                        ("otp_refunds", obj(vec![("values", refunds)])),
                    ]),
                )]),
            )])
        }
        "dashboard/subscriptions_stats" => {
            let active = values(url, "MONTH", |b| {
                row(b, vec![("value", number(active(b.day)))])
            });
            let churn = values(url, "MONTH", |b| {
                let n = total(b, 30.0, 6.0);
                let c = total(b, 24.0, 5.0);
                row(
                    b,
                    vec![
                        ("new", number(n)),
                        ("churn", number(c)),
                        ("sum", number(n - c)),
                    ],
                )
            });
            let refund = values(url, "MONTH", |b| {
                let count = total(b, 30.0, 6.0).round();
                let r = (count * 0.015).round();
                row(
                    b,
                    vec![
                        ("refund_count", number(r)),
                        ("subscription_count", number(count)),
                        ("percent", number((r / count * 1000.0).round() / 10.0)),
                    ],
                )
            });
            obj(vec![(
                "subscriptions_stats",
                obj(vec![(
                    "data",
                    obj(vec![
                        ("active", obj(vec![("values", active)])),
                        (
                            "new_churn",
                            obj(vec![("values", churn), ("variations", array(vec![]))]),
                        ),
                        ("refund", obj(vec![("values", refund)])),
                    ]),
                )]),
            )])
        }
        "dashboard/trials_live" => {
            let trial = |b: &Bucket, p: &str| {
                vec![
                    (format!("{p}_converted"), number(total(b, 9.0, 3.0))),
                    (format!("{p}_not_converted"), number(total(b, 3.0, 2.0))),
                    (
                        format!("{p}_ongoing_auto_renewing"),
                        number(total(b, 6.0, 2.0)),
                    ),
                    (format!("{p}_ongoing_billing_retry"), number(0.0)),
                    (format!("{p}_ongoing_grace_period"), number(0.0)),
                    (
                        format!("{p}_ongoing_non_auto_renewing"),
                        number(total(b, 1.0, 1.0)),
                    ),
                ]
            };
            let balance = |b: &Bucket, p: &str| {
                vec![
                    (format!("{p}_started"), number(total(b, 12.0, 3.0))),
                    (
                        format!("{p}_started_auto_renewing"),
                        number(total(b, 11.0, 3.0)),
                    ),
                    (
                        format!("{p}_started_non_auto_renewing"),
                        number(total(b, 1.0, 1.0)),
                    ),
                    (format!("{p}_converted"), number(total(b, 9.0, 3.0))),
                    (format!("{p}_not_converted"), number(total(b, 3.0, 2.0))),
                    ("balance".into(), number(total(b, 0.0, 2.0))),
                ]
            };
            let series = |f: &dyn Fn(&Bucket) -> Vec<(String, Value)>| {
                array(
                    buckets(url, "MONTH")
                        .iter()
                        .map(|b| {
                            let mut x = vec![("period".into(), text(b.period.clone()))];
                            x.extend(f(b));
                            Value::Object(x)
                        })
                        .collect(),
                )
            };
            let val = obj(vec![(
                "data",
                obj(vec![
                    (
                        "trials",
                        obj(vec![("values", series(&|b| trial(b, "trials")))]),
                    ),
                    (
                        "intro_offers",
                        obj(vec![("values", series(&|b| trial(b, "intro_offers")))]),
                    ),
                    (
                        "trials_balance",
                        obj(vec![("values", series(&|b| balance(b, "trials")))]),
                    ),
                    (
                        "intro_offers_balance",
                        obj(vec![("values", series(&|b| balance(b, "intro_offers")))]),
                    ),
                ]),
            )]);
            obj(vec![("trials_live", val)])
        }
        "dashboard_v2/mau_reports" => {
            let months = last_months(wall_now, 10);
            let users: Vec<_> = months
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let v = (15_000.0 + 400.0 * i as f64 + 800.0 * (i as f64).sin()).round();
                    obj(vec![("period", text(p.clone())), ("value", number(v))])
                })
                .collect();
            let starts: Vec<_> = users
                .iter()
                .map(|u| {
                    obj(vec![
                        ("period", text(field(u, "period"))),
                        ("value", number(number_field(u, "value") * 8.0)),
                    ])
                })
                .collect();
            obj(vec![(
                "mau_reports",
                obj(vec![(
                    "data",
                    obj(vec![
                        ("app_starts", array(starts)),
                        ("unique_users", array(users)),
                    ]),
                )]),
            )])
        }
        "dashboard_v2/paid_subscription_movements" => obj(vec![(
            "paid_subscription_movements",
            values(url, "MONTH", |b| {
                let a = total(b, 28.0, 5.0);
                let c = total(b, 22.0, 5.0);
                row(
                    b,
                    vec![
                        ("group_by", Value::Null),
                        ("group_by_public_id", Value::Null),
                        ("group_by_name", Value::Null),
                        ("total_activations_count", number(a)),
                        ("total_churns_count", number(c)),
                        ("net_movement_count", number(a - c)),
                    ],
                )
            }),
        )]),
        "dashboard_v2/paid_subscription_retention" => {
            obj(vec![("paid_subscription_retention", retention_value(url))])
        }
        "dashboard_v2/paywall_latency" => paywall_value(url, wall_now),
        "dashboard_v2/sdk_evolution" => flow(url, "sdk_evolution", "app_starts_count", 2500.0),
        "dashboard_v2/sdk_versions" => parsed(
            r#"{"sdk_versions":[{"sdk_version":"6.2.0","events_count":84000},{"sdk_version":"6.1.0","events_count":3500},{"sdk_version":"6.3.0","events_count":900},{"sdk_version":"6.0.0","events_count":20}]}"#,
        ),
        "dashboard_v2/subscription_activations" => flow(
            url,
            "subscription_activations",
            "total_activations_count",
            30.0,
        ),
        "dashboard_v2/subscription_reactivations" => flow(
            url,
            "subscription_reactivations",
            "total_reactivations_count",
            4.0,
        ),
        "dashboard_v2/conversion" => {
            let bs = buckets(url, "DAY");
            let viewers: Vec<_> = bs
                .iter()
                .map(|b| {
                    row(
                        b,
                        vec![("unique_viewers_count", number(total(b, 850.0, 120.0)))],
                    )
                })
                .collect();
            let sum: f64 = viewers
                .iter()
                .map(|v| number_field(v, "unique_viewers_count"))
                .sum();
            let secondary = bs
                .iter()
                .map(|b| {
                    row(
                        b,
                        vec![("total_conversions_count", number(total(b, 30.0, 6.0)))],
                    )
                })
                .collect();
            obj(vec![(
                "conversion",
                obj(vec![
                    (
                        "kpi",
                        array(vec![obj(vec![(
                            "unique_viewers_count",
                            number((sum * 0.8).round()),
                        )])]),
                    ),
                    ("primary_metric_by_period", array(viewers)),
                    ("secondary_metric_by_period", array(secondary)),
                ]),
            )])
        }
        _ => return None,
    })
}

fn last_months(wall_now: i64, count: usize) -> Vec<String> {
    let date = time::iso_date(wall_now);
    let year = date
        .get(..4)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1970);
    let month = date
        .get(5..7)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1);
    (0..count)
        .map(|i| {
            let absolute = year * 12 + month - 1 - (count - 1 - i) as i32;
            format!(
                "{:04}-{:02}",
                absolute.div_euclid(12),
                absolute.rem_euclid(12) + 1
            )
        })
        .collect()
}

fn retention_value(url: &str) -> Value {
    let dates = query(url, "date_range[]");
    let mut months = Vec::new();
    if dates.len() == 2 && day_index(&dates[0]).is_some() && day_index(&dates[1]).is_some() {
        let year = |d: &str| {
            d.get(..4)
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(1970)
        };
        let month = |d: &str| d.get(5..7).and_then(|s| s.parse::<i64>().ok()).unwrap_or(1);
        let start = year(&dates[0]) * 12 + month(&dates[0]) - 1;
        let end = year(&dates[1]) * 12 + month(&dates[1]) - 1;
        for index in start..=end {
            let y = index.div_euclid(12);
            let m = index.rem_euclid(12) + 1;
            months.push(format!("{y:04}-{m:02}-01"));
        }
    }
    let size = 140.0;
    let mut rows = Vec::new();
    for (c, cohort) in months.iter().enumerate() {
        for (k, period) in months[c..].iter().enumerate() {
            rows.push(obj(vec![
                ("cohort_date", text(cohort.clone())),
                ("period_date", text(period.clone())),
                ("period_name", text(format!("MONTH {k}"))),
                ("is_partial", Value::Bool(c + k == months.len() - 1)),
                (
                    "retained_count",
                    number((size * 0.82_f64.powi(k as i32)).round()),
                ),
                (
                    "retained_ratio",
                    number((0.82_f64.powi(k as i32) * 1000.0).round() / 1000.0),
                ),
            ]));
        }
    }
    let totals = months
        .iter()
        .map(|cohort| {
            obj(vec![
                ("cohort_date", text(cohort.clone())),
                ("total_count", number(size)),
                (
                    "total_revenue_in_dashboard_currency",
                    number((size * 2.4 * 9.5).round()),
                ),
                (
                    "revenue_per_subscription_in_dashboard_currency",
                    number(22.8),
                ),
            ])
        })
        .collect();
    let periods: Vec<_> = months
        .iter()
        .enumerate()
        .map(|(k, m)| {
            let cohorts = months.len() - k;
            obj(vec![
                ("period_name", text(format!("MONTH {k}"))),
                ("period_date", text(m.clone())),
                (
                    "retained_count",
                    number((size * 0.82_f64.powi(k as i32)).round() * cohorts as f64),
                ),
                ("total_count", number(size * cohorts as f64)),
                (
                    "retained_ratio",
                    number((0.82_f64.powi(k as i32) * 1000.0).round() / 1000.0),
                ),
            ])
        })
        .collect();
    obj(vec![
        ("rows", array(rows)),
        ("totals", array(totals)),
        (
            "grand_total",
            obj(vec![
                ("total_count", number(size * months.len() as f64)),
                (
                    "total_revenue_in_dashboard_currency",
                    number((size * 2.4 * 9.5).round() * months.len() as f64),
                ),
                (
                    "revenue_per_subscription_in_dashboard_currency",
                    number(22.8),
                ),
                ("periods", array(periods)),
            ]),
        ),
    ])
}

fn paywall_value(url: &str, wall_now: i64) -> Value {
    let n = query(url, "period")
        .first()
        .and_then(|x| x.parse::<i64>().ok())
        .unwrap_or(30)
        .clamp(7, 90) as usize;
    let dates: Vec<_> = (0..n)
        .map(|i| text(time::iso_date(wall_now - (n - 1 - i) as i64 * 86_400_000)))
        .collect();
    let at = |base: f64| {
        array(
            (0..n)
                .map(|i| {
                    number(wave(
                        wall_now.div_euclid(86_400_000) - (n - 1 - i) as i64,
                        base,
                        0.0,
                        base / 10.0,
                    ))
                })
                .collect(),
        )
    };
    obj(vec![(
        "paywall_latency",
        obj(vec![(
            "data",
            obj(vec![
                ("dates", array(dates)),
                ("p50", at(120.0)),
                ("p95", at(340.0)),
                ("p99", at(780.0)),
                ("request_count", at(2400.0)),
                ("countries", array(vec![text("US"), text("FR"), text("GB")])),
            ]),
        )]),
    )])
}
