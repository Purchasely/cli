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

/// What the case files map: the `test` names, and the `unit` of each case that has one.
#[derive(Debug, Default)]
pub struct Mapping {
    pub tests: BTreeSet<String>,
    /// `(test, unit)` pairs, in file order.
    pub units: Vec<(String, String)>,
    /// Tests that hold steps (a replay or binary case).
    pub recorded: BTreeSet<String>,
}

impl Mapping {
    /// A case maps its test when it has steps or a `unit`. A `steps: []` case with no `unit` maps nothing.
    pub fn from_files(files: &[Value]) -> Result<Self, String> {
        let mut mapping = Self::default();
        for file in files {
            let cases = file["cases"]
                .as_array()
                .ok_or("a case file must be {\"cases\":[...]}")?;
            for case in cases {
                let test = case["test"].as_str().ok_or("a case has no test name")?;
                let recorded = case["steps"].as_array().is_some_and(|s| !s.is_empty());
                let unit = case.get("unit").and_then(Value::as_str);
                if recorded {
                    mapping.recorded.insert(test.to_owned());
                }
                if let Some(unit) = unit {
                    mapping.units.push((test.to_owned(), unit.to_owned()));
                }
                if recorded || unit.is_some() {
                    mapping.tests.insert(test.to_owned());
                }
            }
        }
        Ok(mapping)
    }
}

impl Inventory {
    pub fn unmapped<'a>(&'a self, mapped: &BTreeSet<String>) -> Vec<&'a str> {
        self.ids
            .iter()
            .filter(|id| !mapped.contains(*id) && !self.exclusions.iter().any(|e| &e.id == *id))
            .map(String::as_str)
            .collect()
    }

    pub fn unmapped_count(&self, mapped: &BTreeSet<String>) -> usize {
        self.unmapped(mapped).len()
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

/// The tests of `cargo test --workspace -- --list` (stdout and stderr together), as `<target>::<path>`.
///
/// `<target>` is the name of the test binary that cargo announces in its `Running` line: the crate name
/// for the unit tests of a crate, the file stem for an integration test (`tests/install.rs` is `install`).
/// Doc-tests are left out.
pub fn parse_test_list(text: &str) -> BTreeSet<String> {
    let mut target: Option<String> = None;
    let mut tests = BTreeSet::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Running ") {
            target = trimmed
                .rsplit_once('(')
                .and_then(|(_, path)| path.strip_suffix(')'))
                .and_then(|path| path.rsplit(['/', '\\']).next())
                .map(|file| file.strip_suffix(".exe").unwrap_or(file))
                .and_then(|file| file.rsplit_once('-'))
                .map(|(name, _hash)| name.to_owned());
        } else if trimmed.starts_with("Doc-tests ") {
            target = None;
        } else if let (Some(target), Some(path)) = (&target, line.strip_suffix(": test")) {
            tests.insert(format!("{target}::{path}"));
        }
    }
    tests
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unmapped_id_is_counted() {
        let inv = Inventory {
            base: "abc".into(),
            ids: vec!["a::one".into(), "b::two".into(), "c::three".into()],
            exclusions: vec![Exclusion {
                id: "c::three".into(),
                reason: "why".into(),
            }],
        };
        let mapped = BTreeSet::from(["a::one".to_owned()]);
        assert_eq!(inv.unmapped(&mapped), ["b::two"]);
        assert_eq!(inv.unmapped_count(&mapped), 1);
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
        assert!(
            Inventory::validate_unit(
                "x::test",
                &BTreeSet::from(["x::test".into()]),
                &BTreeSet::new()
            )
            .is_ok()
        );
    }

    #[test]
    fn an_empty_case_without_unit_maps_nothing() {
        let files = [json!({"cases":[
            {"test":"a::recorded","steps":[{"argv":[]}]},
            {"test":"a::unit","steps":[],"unit":"purchasely::x::tests::y"},
            {"test":"a::empty","steps":[]},
        ]})];
        let mapping = Mapping::from_files(&files).unwrap();
        assert_eq!(
            mapping.tests,
            BTreeSet::from(["a::recorded".into(), "a::unit".into()])
        );
        assert_eq!(mapping.recorded, BTreeSet::from(["a::recorded".into()]));
        assert_eq!(
            mapping.units,
            [("a::unit".into(), "purchasely::x::tests::y".into())]
        );
        assert!(Mapping::from_files(&[json!([])]).is_err());
    }

    #[test]
    fn test_list_names_each_test_by_its_binary() {
        let text = "   Compiling contract v0.1.0\n\
            \x20    Running unittests src/lib.rs (target/debug/deps/contract-208dafbaf74292a1)\n\
            case::tests::roundtrip_case: test\n\
            \n\
            1 test, 0 benchmarks\n\
            \x20    Running tests/install.rs (target/debug/deps/install-384ed4b12130eaea)\n\
            fresh_install_native: test\n\
            \x20    Running unittests src/lib.rs (target\\debug\\deps\\purchasely-3b5fea4f0da89f6d.exe)\n\
            games::snake::tests::eats_grows_and_scores: test\n\
            \x20  Doc-tests purchasely\n\
            src/lib.rs - run (line 3): test\n";
        assert_eq!(
            parse_test_list(text),
            BTreeSet::from([
                "contract::case::tests::roundtrip_case".into(),
                "install::fresh_install_native".into(),
                "purchasely::games::snake::tests::eats_grows_and_scores".into(),
            ])
        );
    }
}
