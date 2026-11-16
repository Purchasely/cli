use super::json::{self, Value};
use std::collections::BTreeMap;

#[allow(non_snake_case)]
pub mod EXIT {
    pub const OK: i32 = 0;
    pub const GENERIC: i32 = 1;
    pub const USAGE: i32 = 2;
    pub const AUTH: i32 = 3;
    pub const FORBIDDEN: i32 = 4;
    pub const VALIDATION: i32 = 5;
    pub const NOT_FOUND: i32 = 6;
    pub const RATE_LIMITED: i32 = 7;
    pub const CONFIRMATION: i32 = 8;
    pub const PARTIAL: i32 = 9;
    pub const REFUSED: i32 = 10;
}

#[derive(Clone, Debug, PartialEq)]
pub struct CliError {
    pub exit: i32,
    pub code: String,
    pub messages: Vec<String>,
    pub extra: BTreeMap<String, Value>,
    pub status: Option<u16>,
}
impl CliError {
    pub fn new(exit: i32, code: impl Into<String>, messages: Vec<String>) -> Self {
        Self {
            exit,
            code: code.into(),
            messages,
            extra: BTreeMap::new(),
            status: None,
        }
    }
    pub fn message(&self) -> String {
        self.messages.join("; ")
    }
    pub fn to_value(&self) -> Value {
        let mut error = vec![
            ("code".into(), Value::from(self.code.clone())),
            (
                "messages".into(),
                Value::Array(self.messages.iter().cloned().map(Value::from).collect()),
            ),
        ];
        if let Some(status) = self.status {
            error.push(("status".into(), Value::Number(status as f64)));
        }
        let token = self.extra.get("confirm_token").cloned();
        for (key, value) in &self.extra {
            if key != "confirm_token" {
                error.push((key.clone(), value.clone()));
            }
        }
        let mut root = vec![("error".into(), Value::Object(error))];
        if let Some(token) = token
            && token != Value::Null
        {
            root.push(("confirm_token".into(), token));
        }
        Value::Object(root)
    }
    pub fn to_json(&self) -> String {
        json::stringify(&self.to_value(), None).unwrap_or_else(|_| "{}".into())
    }
}
pub fn usage(msg: impl Into<String>) -> CliError {
    CliError::new(EXIT::USAGE, "USAGE", vec![msg.into()])
}
pub fn exit_for(code: Option<&str>, status: u16) -> i32 {
    match code {
        Some("UNAUTHORIZED") => EXIT::AUTH,
        Some("FORBIDDEN" | "FEATURE_NOT_ENABLED") => EXIT::FORBIDDEN,
        Some("VALIDATION_FAILED") => EXIT::VALIDATION,
        Some("NOT_FOUND") => EXIT::NOT_FOUND,
        Some("RATE_LIMITED") => EXIT::RATE_LIMITED,
        Some("CONFIRMATION_REQUIRED") => EXIT::CONFIRMATION,
        _ => match status {
            401 => EXIT::AUTH,
            403 => EXIT::FORBIDDEN,
            404 => EXIT::NOT_FOUND,
            422 => EXIT::VALIDATION,
            429 => EXIT::RATE_LIMITED,
            _ => EXIT::GENERIC,
        },
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exit_codes_cover_cli_contract() {
        assert_eq!(
            [
                EXIT::OK,
                EXIT::GENERIC,
                EXIT::USAGE,
                EXIT::AUTH,
                EXIT::FORBIDDEN,
                EXIT::VALIDATION,
                EXIT::NOT_FOUND,
                EXIT::RATE_LIMITED,
                EXIT::CONFIRMATION,
                EXIT::PARTIAL,
                EXIT::REFUSED
            ],
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        );
    }
    #[test]
    fn serializes_error_shape() {
        let mut e = usage("bad");
        e.status = Some(422);
        assert_eq!(e.message(), "bad");
        assert_eq!(
            e.to_json(),
            r#"{"error":{"code":"USAGE","messages":["bad"],"status":422}}"#
        );
    }
    #[test]
    fn maps_error_code_before_status() {
        assert_eq!(exit_for(Some("RATE_LIMITED"), 403), 7);
        assert_eq!(exit_for(None, 429), 7);
    }
}
