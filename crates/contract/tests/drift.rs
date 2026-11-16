use contract::{
    normalize::normalize,
    replay::{compare_dirs, compare_drift_case, normalize_case},
};
use serde_json::json;
use std::path::Path;

#[test]
fn normalizes_pid_without_losing_uuid_correlation() {
    let id = "123e4567-e89b-42d3-a456-426614174000";
    let input = format!("{{\"pid\":42,\"header\":\"{id}\",\"file\":\"{id}\",\"lock\":\"pid=42\"}}");
    let output = normalize(&input, "__unused_root__", 42, "9.9.9");
    assert!(output.contains("<UUID1>\",\"file\":\"<UUID1>"));
    assert!(output.contains("pid=<PID>"));
    assert_eq!(
        normalize("1423 42", "__unused_root__", 42, "9.9.9"),
        "1423 <PID>"
    );
    let two = normalize(
        &format!("{id} 00000000-0000-4000-8000-000000000000 {id}"),
        "__r__",
        7,
        "9.9.9",
    );
    assert_eq!(two, "<UUID1> <UUID2> <UUID1>");
}

#[test]
fn intended_diff_uses_expected_ts_recording() {
    let committed = json!({"test":"api::x","intended_diff":"approved","expected_ts":{"test":"api::x","steps":[{"code":0}]},"steps":[{"code":10}]});
    let fresh = json!({"test":"api::x","steps":[{"code":0}]});
    assert!(compare_drift_case(&committed, &fresh).is_ok());
}

#[test]
fn committed_cases_match_fresh_recordings() {
    let Some(dir) = std::env::var_os("CONTRACT_DRIFT_DIR").map(std::path::PathBuf::from) else {
        eprintln!("drift skipped: CONTRACT_DRIFT_DIR unset");
        return;
    };
    let committed = Path::new("contract/cases");
    let (checked, skipped, new_files) = compare_dirs(committed, &dir).unwrap();
    eprintln!("drift checked {checked}, skipped {skipped}, new files {new_files}");
}

#[test]
fn normalized_json_keeps_test_id_and_removes_runtime_pid() {
    let value = json!({"test":"network::sample","steps":[{"pid":123,"lock":"pid=123","headers":{"X-Request-Id":"123e4567-e89b-42d3-a456-426614174000"}}]});
    let actual = normalize_case(&value);
    assert_eq!(actual["test"], "network::sample");
    assert!(actual.to_string().contains("<UUID1>"));
    assert!(actual.to_string().contains("pid=<PID>"));
    assert!(actual["steps"][0].get("pid").is_none());
}
