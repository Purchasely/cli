use serde_json::Value;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Case {
    pub test: String,
    #[serde(default)]
    pub steps: Vec<Step>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Step {
    #[serde(flatten)]
    pub data: serde_json::Map<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_case() {
        let source = r#"{"test":"search > finds","steps":[{"argv":["search"]}],"extra":true}"#;
        let case: Case = serde_json::from_str(source).unwrap();
        let encoded = serde_json::to_value(case).unwrap();
        assert_eq!(encoded["test"], "search > finds");
        assert_eq!(encoded["steps"][0]["argv"][0], "search");
        assert_eq!(encoded["extra"], true);
    }

    #[test]
    fn pending_field_is_optional_and_roundtrips() {
        let case: Case = serde_json::from_str(r#"{"test":"x","steps":[],"pending":"P4"}"#).unwrap();
        assert_eq!(case.pending.as_deref(), Some("P4"));
        assert_eq!(serde_json::to_value(case).unwrap()["pending"], "P4");
    }
}
