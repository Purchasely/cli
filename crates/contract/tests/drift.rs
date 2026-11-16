use serde_json::Value;
use std::{fs, path::Path};

fn normalized_case(path: &Path) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    let mut value: Value = serde_json::from_str(&text).ok()?;
    if value.get("intended_diff").is_some() {
        value = value.get("expected_ts")?.clone();
    }
    if let Some(obj) = value.as_object_mut() {
        obj.remove("pid");
    }
    Some(value)
}

#[test]
fn committed_cases_match_fresh_recordings() {
    let Some(dir) = std::env::var_os("CONTRACT_DRIFT_DIR").map(std::path::PathBuf::from) else {
        eprintln!("drift skipped: CONTRACT_DRIFT_DIR unset");
        return;
    };
    let committed = Path::new("contract/cases");
    if !committed.exists() {
        return;
    }
    let mut checked = 0usize;
    let mut skipped = 0usize;
    for entry in fs::read_dir(committed).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|v| v.to_str()) != Some("json") {
            continue;
        }
        let Some(expected) = normalized_case(&path) else {
            continue;
        };
        let Some(name) = path.file_name() else {
            continue;
        };
        let fresh = dir.join(name);
        if !fresh.exists() {
            if expected.get("unit").is_some() {
                skipped += 1;
                continue;
            } else {
                eprintln!("no fresh recording: {}", path.display());
                continue;
            }
        }
        assert_eq!(
            normalized_case(&fresh).unwrap(),
            expected,
            "drift in {}",
            path.display()
        );
        checked += 1;
    }
    eprintln!("drift checked {checked}, skipped {skipped}");
}
