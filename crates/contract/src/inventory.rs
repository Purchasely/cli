use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Inventory {
    pub base: String,
    pub ids: Vec<String>,
    #[serde(default)]
    pub exclusions: Vec<Exclusion>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Exclusion {
    pub id: String,
    pub reason: String,
}

impl Inventory {
    pub fn unmapped_count(&self, mapped: &BTreeSet<String>) -> usize {
        self.ids
            .iter()
            .filter(|id| !mapped.contains(*id) && !self.exclusions.iter().any(|e| &e.id == *id))
            .count()
    }

    pub fn validate_unit(
        unit: &str,
        listed: &BTreeSet<String>,
        ignored: &BTreeSet<String>,
    ) -> Result<(), String> {
        if !listed.contains(unit) || ignored.contains(unit) {
            Err(format!("unit test is missing or ignored: {unit}"))
        } else {
            Ok(())
        }
    }

    pub fn from_vitest(value: &Value, base: &str) -> Result<Self, String> {
        let tests = value
            .as_array()
            .ok_or("vitest list must return a JSON array")?;
        let ids = tests
            .iter()
            .map(|test| {
                let name = test
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or("test has no name")?;
                let file = test
                    .get("file")
                    .and_then(Value::as_str)
                    .ok_or("test has no file")?;
                let stem = std::path::Path::new(file)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or("test file has no stem")?;
                Ok(format!("{stem}::{name}"))
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Self {
            base: base.to_owned(),
            ids,
            exclusions: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unmapped_id_is_counted() {
        let inv = Inventory {
            base: "abc".into(),
            ids: vec!["a::one".into(), "b::two".into()],
            exclusions: vec![],
        };
        assert_eq!(
            inv.unmapped_count(&BTreeSet::from(["a::one".to_owned()])),
            1
        );
    }

    #[test]
    fn missing_unit_test_fails() {
        assert!(Inventory::validate_unit("x::test", &BTreeSet::new(), &BTreeSet::new()).is_err());
        assert!(
            Inventory::validate_unit(
                "x::test",
                &BTreeSet::from(["x::test".into()]),
                &BTreeSet::from(["x::test".into()])
            )
            .is_err()
        );
    }
}
