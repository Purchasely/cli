//! Help text for commands generated from the embedded API description.

use super::naming::kebab;
use super::spec::{Command, Op, Param, Schema};

pub fn type_label(schema: &Schema) -> String {
    if !schema.choices.is_empty() {
        return format!("enum: {}", schema.choices.join("|"));
    }
    if schema.kind.as_deref() == Some("array") {
        return format!(
            "array of {}",
            schema
                .items
                .as_deref()
                .map(type_label)
                .unwrap_or_else(|| "any".to_owned())
        );
    }
    if let Some(kind) = &schema.kind {
        return kind.clone();
    }
    if !schema.any_of.is_empty() {
        return schema
            .any_of
            .iter()
            .map(type_label)
            .collect::<Vec<_>>()
            .join(" or ");
    }
    "any".to_owned()
}

pub fn flag_of(param: &Param) -> String {
    if param.name == "application_id" {
        "--app".to_owned()
    } else {
        format!("--{}", kebab(&param.name))
    }
}

pub fn required_flags(op: &Op) -> String {
    let mut parts = op
        .params
        .iter()
        .filter(|p| p.required && p.name != "confirm_token")
        .map(|p| {
            format!(
                "{} {}",
                flag_of(p),
                if p.name == "application_id" {
                    "app_xxx".to_owned()
                } else {
                    format!("<{}>", kebab(&p.name))
                }
            )
        })
        .collect::<Vec<_>>();
    if op.body.as_ref().is_some_and(|b| b.required) {
        parts.push("--data @body.json".to_owned());
    }
    parts.join(" ")
}

pub fn usage_string(command: &Command, name: &str) -> String {
    let op = command
        .app_op
        .as_ref()
        .or(command.global_op.as_ref())
        .expect("command has operation");
    let env = if op.method == "GET" {
        ""
    } else {
        " --env prod"
    };
    format!(
        "{} {} {} {}{}",
        name,
        command.resource,
        command.action,
        required_flags(op),
        env
    )
    .split_whitespace()
    .collect::<Vec<_>>()
    .join(" ")
}

pub fn help_text(command: &Command, name: &str) -> String {
    let ops = command
        .app_op
        .iter()
        .chain(command.global_op.iter())
        .collect::<Vec<_>>();
    let first = ops[0];
    let mut lines = vec![
        String::new(),
        plain(&strip_required_scope(&first.description)),
        String::new(),
        format!("Required scope: {}", first.scope),
        "Request:".to_owned(),
    ];
    for op in &ops {
        let variant = if ops.len() > 1 {
            if op.app {
                "  (with --app)"
            } else {
                "  (without --app)"
            }
        } else {
            ""
        };
        lines.push(format!("  {} {}{}", op.method, op.path, variant));
    }
    if first.method != "GET" {
        lines.push(String::new());
        lines.push(
            "A write needs --env prod or --env staging. Run it with --dry-run first.".to_owned(),
        );
    }
    if let Some(body) = &first.body {
        lines.push(String::new());
        lines.push(format!(
            "Body ({}): --data '<json>' or --data @file.json or --data -",
            if body.required {
                "required"
            } else {
                "optional"
            }
        ));
        for (key, schema) in &body.schema.properties {
            let required = if body.schema.required.contains(key) {
                ", required"
            } else {
                ""
            };
            let desc = schema
                .description
                .as_deref()
                .unwrap_or("")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let line = format!(
                "  {} ({}{}) {}",
                key,
                type_label(schema),
                required,
                plain(&desc)
            );
            lines.push(line.trim_end().to_owned());
        }
    }
    if let Some(op) = ops.iter().find(|o| o.confirm.is_some()) {
        lines.push(String::new());
        lines.push(format!(
            "Confirmation flow: a risky change first returns the impact and a confirm_token ({}).",
            op.confirm.as_deref().unwrap_or("")
        ));
        lines.push("  Add --yes to confirm at once, answer the prompt, or resend with --confirm-token <token>.".to_owned());
    }
    lines.push(String::new());
    lines.push(format!("Example:\n  {}", usage_string(command, name)));
    lines.join("\n")
}

fn strip_required_scope(description: &str) -> String {
    if let Some(index) = description.rfind("\n**Required scope:**") {
        description[..index].trim().to_owned()
    } else {
        description.trim().to_owned()
    }
}
pub(crate) fn plain(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
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
    use crate::management::spec::{derive_commands, embedded_spec};

    fn help(resource: &str, action: &str) -> String {
        let (_, ops) = embedded_spec();
        let command = derive_commands(&ops)
            .into_iter()
            .find(|c| c.resource == resource && c.action == action)
            .unwrap();
        help_text(&command, "purchasely")
    }

    #[test]
    fn placements_list_help_matches_ts() {
        assert_eq!(
            help("placements", "list"),
            "\nList placements for an application.\n\nRequired scope: targeting.placements:view\nRequest:\n  GET /management/v1/applications/{application_id}/placements  (with --app)\n  GET /management/v1/placements  (without --app)\n\nExample:\n  purchasely placements list --app app_xxx"
        );
    }

    #[test]
    fn placements_delete_help_matches_ts() {
        assert_eq!(
            help("placements", "delete"),
            "\nDiscard (soft-delete) a placement. Requires confirm_token on retry if the placement is still associated with running campaigns/audiences. Blocked outright (non-overridable) if the placement has draft/running AB tests or is the app's default placement.\n\nRequired scope: targeting.placements:delete\nRequest:\n  DELETE /management/v1/applications/{application_id}/placements/{id}\n\nA write needs --env prod or --env staging. Run it with --dry-run first.\n\nConfirmation flow: a risky change first returns the impact and a confirm_token (query).\n  Add --yes to confirm at once, answer the prompt, or resend with --confirm-token <token>.\n\nExample:\n  purchasely placements delete --app app_xxx --id <id> --env prod"
        );
    }

    #[test]
    fn applications_list_help_matches_ts() {
        assert_eq!(
            help("applications", "list"),
            "\nList your applications.\n\nRequired scope: setup.apps:view\nRequest:\n  GET /management/v1/applications\n\nExample:\n  purchasely applications list"
        );
    }
}
