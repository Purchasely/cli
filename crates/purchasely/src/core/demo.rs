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
        if let Some((key, value)) = lists.iter().find(|(key, _)| {
            endpoint == format!("/{key}") || endpoint.ends_with(&format!("/{key}"))
        }) {
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

/// The fixed wall instant used by replay is the source for demo time series and month labels.
pub fn wall_date(d: &Deps) -> String {
    time::iso_date(d.host.wall_now())
}

fn last_months(wall_now: i64, count: usize) -> Vec<String> {
    let date = time::iso_date(wall_now);
    let mut parts = date.split('-').filter_map(|s| s.parse::<i32>().ok());
    let (Some(year), Some(month), _) = (parts.next(), parts.next(), parts.next()) else {
        return vec![];
    };
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
