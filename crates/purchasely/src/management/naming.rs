//! Derives stable resource and action names from API paths and tags.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Name {
    pub resource: String,
    pub action: String,
    pub parent: Option<String>,
}

pub fn kebab(value: &str) -> String {
    value.replace('_', "-").to_ascii_lowercase()
}
pub fn tag_slug(tag: &str) -> String {
    tag.to_ascii_lowercase()
        .replace('/', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}
pub fn singular(value: &str) -> String {
    if let Some(prefix) = value.strip_suffix("ies") {
        format!("{prefix}y")
    } else if let Some(prefix) = value.strip_suffix('s') {
        prefix.to_owned()
    } else {
        value.to_owned()
    }
}

fn is_param(value: &str) -> bool {
    value.starts_with('{')
}
fn matches(segment: &str, slug: &str) -> bool {
    let segment = kebab(segment);
    segment == slug || segment.ends_with(&format!("-{slug}")) || slug.starts_with(&segment)
}

pub fn derive_name(method: &str, path: &str, tag: &str) -> Name {
    let slug = tag_slug(tag);
    let mut segments = path
        .strip_prefix("/management/v1/")
        .unwrap_or(path)
        .split('/')
        .collect::<Vec<_>>();
    if segments.len() > 2 && segments[0] == "applications" && segments[1] == "{application_id}" {
        segments.drain(0..2);
    }
    let collection_index = segments
        .iter()
        .enumerate()
        .rev()
        .find(|(_, segment)| !is_param(segment) && matches(segment, &slug))
        .map(|(i, _)| i);
    let literals = segments
        .iter()
        .filter(|s| !is_param(s))
        .copied()
        .collect::<Vec<_>>();
    let ends_with_param = segments.last().is_some_and(|s| is_param(s));
    let mut result = if let Some(ci) = collection_index {
        let extra = &segments[ci + 1..];
        let extra_literals = extra
            .iter()
            .filter(|s| !is_param(s))
            .copied()
            .collect::<Vec<_>>();
        let previous = segments[..ci]
            .iter()
            .rev()
            .find(|s| !is_param(s))
            .map(|s| singular(&kebab(s)));
        let action = if extra_literals.is_empty() {
            if ends_with_param {
                verb(method).to_owned()
            } else if method == "GET"
                && ci > 0
                && is_param(segments[ci - 1])
                && singular(&kebab(segments[ci])) == segments[ci]
            {
                "get".to_owned()
            } else if method == "GET" {
                "list".to_owned()
            } else if method == "POST" {
                "create".to_owned()
            } else {
                "update".to_owned()
            }
        } else if !ends_with_param {
            kebab(&extra_literals.join("-"))
        } else {
            format!(
                "{}-{}",
                verb(method),
                singular(&kebab(&extra_literals.join("-")))
            )
        };
        Name {
            resource: slug.clone(),
            action,
            parent: previous,
        }
    } else {
        let last = literals.last().copied().unwrap_or("");
        let previous = literals.iter().rev().nth(1).copied();
        let action = if slug == "web-activations" && kebab(last) == "web-flows" {
            "flow".to_owned()
        } else if slug == "web-activations" && kebab(last) == "web-screens" {
            "screen".to_owned()
        } else {
            previous.map_or_else(
                || kebab(last),
                |p| format!("{}-{}", kebab(last), singular(&kebab(p))),
            )
        };
        Name {
            resource: slug.clone(),
            action,
            parent: None,
        }
    };
    if result.resource == "dashboard-v2" {
        result.resource = "dashboard".to_owned();
    }
    result
}

fn verb(method: &str) -> &'static str {
    match method {
        "GET" => "get",
        "POST" => "create",
        "PUT" | "PATCH" => "update",
        "DELETE" => "delete",
        _ => "undefined",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_known_names() {
        assert_eq!(
            derive_name(
                "GET",
                "/management/v1/applications/{application_id}/placements",
                "Placements"
            ),
            Name {
                resource: "placements".into(),
                action: "list".into(),
                parent: None
            }
        );
        assert_eq!(
            derive_name(
                "DELETE",
                "/management/v1/applications/{application_id}/placements/{placement_id}/audiences/{audience_id}",
                "Placements"
            ),
            Name {
                resource: "placements".into(),
                action: "delete-audience".into(),
                parent: None
            }
        );
        assert_eq!(
            derive_name(
                "POST",
                "/management/v1/applications/{application_id}/ab_tests/{ab_test_id}/start",
                "AB Tests"
            ),
            Name {
                resource: "ab-tests".into(),
                action: "start".into(),
                parent: None
            }
        );
        assert_eq!(
            derive_name(
                "GET",
                "/management/v1/applications/{application_id}/web_flows/{web_flow_id}",
                "Web Activations"
            ),
            Name {
                resource: "web-activations".into(),
                action: "flow".into(),
                parent: None
            }
        );
        assert_eq!(
            derive_name(
                "GET",
                "/management/v1/applications/{application_id}/dashboard_v2/conversion",
                "Dashboard V2"
            ),
            Name {
                resource: "dashboard".into(),
                action: "conversion".into(),
                parent: None
            }
        );
    }

    #[test]
    fn normalizes_names() {
        assert_eq!(kebab("plan_ID"), "plan-id");
        assert_eq!(tag_slug("AB Tests / V2"), "ab-tests-v2");
        assert_eq!(singular("categories"), "category");
        assert_eq!(singular("status"), "statu");
    }
}
