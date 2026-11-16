use contract::replay::{pending_count, run_case_directory, selected};
use serde_json::json;

#[test]
fn replay_selection_counts_pending_cases() {
    let cases = vec![
        json!({"test":"auth-status::ok","pending":"P5b"}),
        json!({"test":"search::works"}),
    ];
    assert_eq!(pending_count(&cases), 1);
    assert_eq!(selected(&cases, Some("search::")).unwrap().len(), 1);
}

#[test]
fn replay_contract_directory_honors_filter_and_bless_mode() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contract/cases");
    let filter = std::env::var("CONTRACT_FILTER").ok();
    let bless = std::env::var("CONTRACT_BLESS").is_ok_and(|v| v == "1");
    run_case_directory(&directory, filter.as_deref(), bless).unwrap();
}
