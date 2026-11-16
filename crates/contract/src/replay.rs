//! Owner: P1b
use serde_json::{Value, json};
use std::{fs, path::Path};

pub fn pending_count(cases: &[Value]) -> usize {
    cases.iter().filter(|c| c.get("pending").is_some()).count()
}
pub fn selected<'a>(cases: &'a [Value], filter: Option<&str>) -> Result<Vec<&'a Value>, String> {
    let Some(filter) = filter.filter(|s| !s.is_empty()) else {
        return Ok(cases.iter().collect());
    };
    let prefixes: Vec<_> = filter.split(',').collect();
    let found: Vec<_> = cases
        .iter()
        .filter(|c| {
            prefixes
                .iter()
                .any(|p| c["test"].as_str().unwrap_or_default().starts_with(p))
        })
        .collect();
    for p in prefixes {
        if !found
            .iter()
            .any(|c| c["test"].as_str().unwrap_or_default().starts_with(p))
        {
            return Err(format!("CONTRACT_FILTER prefix matched no case: {p}"));
        }
    }
    Ok(found)
}
pub fn bless_case(path: &Path, value: &Value) -> std::io::Result<()> {
    let mut value = value.clone();
    value["source"] = json!("bless");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(&value).map_err(std::io::Error::other)?,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pending_is_counted() {
        assert_eq!(pending_count(&[json!({"pending":"P2"})]), 1);
    }
    #[test]
    fn bless_writes_case() {
        let p = std::env::temp_dir().join(format!("contract-bless-{}", std::process::id()));
        bless_case(&p, &json!({"test":"x"})).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&fs::read(&p).unwrap()).unwrap()["source"],
            "bless"
        );
        let _ = fs::remove_file(p);
    }
    #[test]
    fn filter_rejects_unmatched_prefix() {
        assert!(selected(&[json!({"test":"auth::x"})], Some("search::")).is_err());
    }
}
