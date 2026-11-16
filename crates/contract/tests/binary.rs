use contract::binary::{BinaryResult, compare_recorded};
use serde_json::json;

#[test]
fn binary_transcript_compares_stdout_stderr_and_status() {
    let actual = BinaryResult {
        code: 0,
        stdout: b"ready\n".to_vec(),
        stderr: vec![],
    };
    assert!(compare_recorded(&json!({"code":0,"stdout":"ready\n","stderr":""}), &actual).is_ok());
}
