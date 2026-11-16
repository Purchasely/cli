use contract::replay::{pending_count, selected};
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
