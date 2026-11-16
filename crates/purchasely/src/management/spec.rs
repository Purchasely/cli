//! Runtime parser and command derivation for the embedded API description.

use super::naming::{derive_name, kebab, tag_slug};
use crate::core::json::{self, Value};
use std::collections::HashMap;

type NamedOperation = (Op, String, String, Option<String>);

const SPEC: &str = include_str!("../../../../spec/management.json");
const METHODS: [&str; 5] = ["get", "post", "put", "patch", "delete"];
const NEEDS_DATE_RANGE: [&str; 8] = [
    "conversion",
    "paid_subscription_movements",
    "paid_subscription_retention",
    "sdk_evolution",
    "sdk_versions",
    "subscription_activations",
    "subscription_evolution",
    "subscription_reactivations",
];
const SUMMARY_OVERRIDE: &str = "Paid-subscription retention cohort matrix for a single application";
const RETENTION_PATH: &str =
    "/management/v1/applications/{application_id}/dashboard_v2/paid_subscription_retention";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Schema {
    pub kind: Option<String>,
    pub choices: Vec<String>,
    pub items: Option<Box<Schema>>,
    pub description: Option<String>,
    pub any_of: Vec<Schema>,
    pub properties: Vec<(String, Schema)>,
    pub required: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub location: String,
    pub required: bool,
    pub description: Option<String>,
    pub schema: Schema,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Body {
    pub required: bool,
    pub schema: Schema,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Op {
    pub method: String,
    pub path: String,
    pub tag: String,
    pub summary: String,
    pub description: String,
    pub scope: String,
    pub params: Vec<Param>,
    pub body: Option<Body>,
    pub confirm: Option<String>,
    pub app: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opt {
    pub name: String,
    pub flag: String,
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    pub resource: String,
    pub action: String,
    pub summary: String,
    pub app_op: Option<Op>,
    pub global_op: Option<Op>,
    pub options: Vec<Opt>,
    pub legacy: Option<String>,
}

pub fn parse_spec_doc(source: &str) -> Result<(String, Vec<Op>), String> {
    let root = json::parse(source)
        .map_err(|e| format!("invalid embedded spec at {}: {}", e.offset, e.message))?;
    let servers = array(field(&root, "servers").ok_or("spec has no servers")?);
    let base = servers
        .first()
        .and_then(|s| field(s, "url"))
        .and_then(text)
        .ok_or("spec has no server URL")?;
    let paths = field(&root, "paths")
        .and_then(as_object)
        .ok_or("spec has no paths")?;
    let mut ops = Vec::new();
    for (path, item) in paths {
        for method in METHODS {
            let Some(raw) = field(item, method) else {
                continue;
            };
            let upper = method.to_ascii_uppercase();
            let mut params = Vec::new();
            for p in field(raw, "parameters").map(array).unwrap_or_default() {
                let Some(name) = field(p, "name").and_then(text) else {
                    continue;
                };
                let location = field(p, "in").and_then(text).unwrap_or_default();
                let required_gap = name == "date_range"
                    && upper == "GET"
                    && NEEDS_DATE_RANGE
                        .iter()
                        .any(|v| path.ends_with(&format!("/dashboard_v2/{v}")));
                params.push(Param {
                    name: name.to_owned(),
                    location,
                    required: truthy(field(p, "required")) || required_gap,
                    description: field(p, "description").and_then(text),
                    schema: parse_schema(field(p, "schema").unwrap_or(&Value::Null)),
                });
            }
            let request_body = field(raw, "requestBody");
            let body_schema = request_body
                .and_then(|b| field(b, "content"))
                .and_then(|c| field(c, "application/json"))
                .and_then(|c| field(c, "schema"));
            let summary = field(raw, "summary").and_then(text).unwrap_or_default();
            let summary = if path == RETENTION_PATH {
                SUMMARY_OVERRIDE.to_owned()
            } else {
                summary
            };
            let old_summary = field(raw, "summary").and_then(text).unwrap_or_default();
            let mut description = field(raw, "description").and_then(text).unwrap_or_default();
            if path == RETENTION_PATH && !old_summary.is_empty() {
                description = description.replacen(&old_summary, SUMMARY_OVERRIDE, 1);
            }
            let description = plain(&description);
            let mut confirm = None;
            if params
                .iter()
                .any(|p| p.location == "query" && p.name == "confirm_token")
            {
                confirm = Some("query".to_owned());
            } else if body_schema
                .and_then(|s| field(s, "properties"))
                .and_then(|p| field(p, "confirm_token"))
                .is_some()
            {
                confirm = Some("body".to_owned());
            }
            let tag = field(raw, "tags")
                .map(array)
                .and_then(|v| v.first().copied())
                .and_then(text)
                .unwrap_or_default();
            ops.push(Op {
                method: upper,
                path: path.clone(),
                tag,
                summary: plain(&summary),
                description,
                scope: field(raw, "x-required-scope")
                    .and_then(text)
                    .unwrap_or_default(),
                params,
                body: request_body.map(|rb| Body {
                    required: truthy(field(rb, "required")),
                    schema: parse_schema(body_schema.unwrap_or(&Value::Null)),
                }),
                confirm,
                app: path.contains("{application_id}"),
            });
        }
    }
    ops.sort_by_key(|op| {
        source
            .find(&format!("\"{}\"", op.path))
            .unwrap_or(usize::MAX)
    });
    Ok((base, ops))
}

pub fn parse_spec(source: &str) -> (usize, Vec<Op>) {
    let root = json::parse(source).expect("valid embedded management spec");
    let count = field(&root, "paths")
        .and_then(as_object)
        .map_or(0, Vec::len);
    let (_, ops) = parse_spec_doc(source).expect("valid embedded management spec");
    (count, ops)
}

pub fn embedded_spec() -> (String, Vec<Op>) {
    // The spec is constant: parse it once per process (a replay runs thousands of commands).
    static PARSED: std::sync::OnceLock<(String, Vec<Op>)> = std::sync::OnceLock::new();
    PARSED
        .get_or_init(|| parse_spec_doc(SPEC).expect("valid embedded management spec"))
        .clone()
}

fn parse_schema(value: &Value) -> Schema {
    Schema {
        kind: field(value, "type").and_then(text),
        choices: field(value, "enum")
            .map(array)
            .unwrap_or_default()
            .into_iter()
            .filter_map(text)
            .collect(),
        items: field(value, "items").map(|v| Box::new(parse_schema(v))),
        description: field(value, "description").and_then(text),
        any_of: field(value, "anyOf")
            .map(array)
            .unwrap_or_default()
            .into_iter()
            .map(parse_schema)
            .collect(),
        properties: field(value, "properties")
            .and_then(as_object)
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|(k, v)| (k.clone(), parse_schema(v)))
            .collect(),
        required: field(value, "required")
            .map(array)
            .unwrap_or_default()
            .into_iter()
            .filter_map(text)
            .collect(),
    }
}

pub fn derive_commands(ops: &[Op]) -> Vec<Command> {
    let mut groups: HashMap<String, Vec<NamedOperation>> = HashMap::new();
    let mut keys = Vec::new();
    for op in ops {
        let mut n = derive_name(&op.method, &op.path, &op.tag);
        if op.tag == "Web Activations" && op.summary.ends_with("of a flow") {
            n.resource = "web-activations".to_owned();
            n.action = "flow".to_owned();
        } else if op.tag == "Web Activations" && op.summary.ends_with("of a screen") {
            n.resource = "web-activations".to_owned();
            n.action = "screen".to_owned();
        }
        let key = format!("{} {}", n.resource, n.action);
        if !groups.contains_key(&key) {
            keys.push(key.clone());
        }
        groups
            .entry(key)
            .or_default()
            .push((op.clone(), n.resource, n.action, n.parent));
    }
    let mut commands = Vec::new();
    for key in keys {
        let list = groups.remove(&key).unwrap_or_default();
        let apps = list.iter().filter(|x| x.0.app).count();
        let globals = list.len() - apps;
        if apps <= 1 && globals <= 1 {
            let mut parts = key.splitn(2, ' ');
            let resource = parts.next().unwrap_or("").to_owned();
            let action = parts.next().unwrap_or("").to_owned();
            let app = list.iter().find(|x| x.0.app).map(|x| x.0.clone());
            let global = list.iter().find(|x| !x.0.app).map(|x| x.0.clone());
            commands.push(make_command(resource, action, app, global));
        } else {
            for (op, resource, action, parent) in list {
                let split_action = if action == "get" {
                    parent.clone().unwrap_or(action)
                } else {
                    format!("{}-{}", action, parent.unwrap_or_default())
                };
                let app = if op.app { Some(op.clone()) } else { None };
                let global = if op.app { None } else { Some(op) };
                commands.push(make_command(resource, split_action, app, global));
            }
        }
    }
    commands
}

fn make_command(
    resource: String,
    action: String,
    app_op: Option<Op>,
    global_op: Option<Op>,
) -> Command {
    let mut options = Vec::new();
    for op in app_op.iter().chain(global_op.iter()) {
        for p in &op.params {
            if p.name == "application_id" || p.name == "confirm_token" {
                continue;
            }
            if !options.iter().any(|o: &Opt| o.name == p.name) {
                options.push(Opt {
                    name: p.name.clone(),
                    flag: kebab(&p.name),
                    kind: p.location.clone(),
                });
            }
        }
    }
    let chosen = app_op
        .as_ref()
        .or(global_op.as_ref())
        .expect("command has an operation");
    let slug = tag_slug(&chosen.tag);
    let legacy = (slug != resource).then_some(slug);
    Command {
        resource,
        action,
        summary: chosen.summary.clone(),
        app_op,
        global_op,
        options,
        legacy,
    }
}

fn field<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    as_object(value)?
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v)
}
fn as_object(value: &Value) -> Option<&Vec<(String, Value)>> {
    if let Value::Object(v) = value {
        Some(v)
    } else {
        None
    }
}
fn array(value: &Value) -> Vec<&Value> {
    if let Value::Array(v) = value {
        v.iter().collect()
    } else {
        Vec::new()
    }
}
fn text(value: &Value) -> Option<String> {
    if let Value::String(v) = value {
        String::from_utf16(v).ok()
    } else {
        None
    }
}
fn truthy(value: Option<&Value>) -> bool {
    matches!(value, Some(Value::Bool(true)))
}
fn plain(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{2013}' || ch == '\u{2014}' {
            while out.ends_with(char::is_whitespace) {
                out.pop();
            }
            while chars.peek().is_some_and(|next| next.is_whitespace()) {
                chars.next();
            }
            out.push_str(", ");
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn reads_spec_counts() {
        let (paths, ops) = parse_spec(SPEC);
        assert_eq!(paths, 128);
        assert_eq!(ops.len(), 172);
    }

    #[test]
    fn keeps_prod_base_url() {
        let (base, _) = parse_spec_doc(SPEC).unwrap();
        assert_eq!(base, "https://api.purchasely.io");
    }

    #[test]
    fn derives_unique_commands_and_reaches_every_operation() {
        let (_, ops) = embedded_spec();
        let commands = derive_commands(&ops);
        let names = commands
            .iter()
            .map(|c| format!("{} {}", c.resource, c.action))
            .collect::<BTreeSet<_>>();
        assert_eq!(names.len(), commands.len());
        assert_eq!(commands.len(), 140);
        assert_eq!(
            commands
                .iter()
                .map(|c| usize::from(c.app_op.is_some()) + usize::from(c.global_op.is_some()))
                .sum::<usize>(),
            172
        );
    }

    #[test]
    fn keeps_known_names_and_confirmation_locations() {
        let (_, ops) = embedded_spec();
        let commands = derive_commands(&ops);
        let find = |resource: &str, action: &str| {
            commands
                .iter()
                .find(|c| c.resource == resource && c.action == action)
                .unwrap_or_else(|| panic!("missing generated command {resource} {action}"))
        };
        assert_eq!(
            find("placements", "list").app_op.as_ref().unwrap().path,
            "/management/v1/applications/{application_id}/placements"
        );
        assert_eq!(
            find("placements", "list").global_op.as_ref().unwrap().path,
            "/management/v1/placements"
        );
        assert_eq!(
            find("placements", "delete-audience")
                .app_op
                .as_ref()
                .unwrap()
                .method,
            "DELETE"
        );
        assert_eq!(
            find("ab-tests", "start").app_op.as_ref().unwrap().method,
            "PUT"
        );
        assert!(find("web-activations", "flow").app_op.is_some());
        assert!(find("web-activations", "screen").app_op.is_some());
        assert!(
            find("distributions", "get")
                .options
                .iter()
                .any(|o| o.name == "plan_id")
        );
        assert!(
            find("distributions", "get")
                .options
                .iter()
                .any(|o| o.name == "store")
        );
        assert!(
            commands
                .iter()
                .any(|c| c.resource == "applications" && c.action == "list")
        );
        assert_eq!(
            find("placements", "delete")
                .app_op
                .as_ref()
                .unwrap()
                .confirm
                .as_deref(),
            Some("query")
        );
        assert_eq!(
            find("webhooks", "update")
                .app_op
                .as_ref()
                .unwrap()
                .confirm
                .as_deref(),
            Some("body")
        );
        assert_eq!(
            find("plans", "create")
                .app_op
                .as_ref()
                .unwrap()
                .confirm
                .as_deref(),
            Some("body")
        );
        assert_eq!(
            find("audiences", "discard")
                .app_op
                .as_ref()
                .unwrap()
                .confirm
                .as_deref(),
            Some("body")
        );
        assert!(ops.iter().filter(|o| o.confirm.is_some()).count() >= 12);
    }
}
