use contract::binary::{BinaryResult, compare_recorded, run_binary_directory};
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

#[test]
fn binary_contract_directory_supports_record_replay_and_bless() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contract/bin");
    let command = std::env::var("CONTRACT_BIN").unwrap_or_else(|_| {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/debug/purchasely")
            .to_string_lossy()
            .into_owned()
    });
    let record = std::env::var("CONTRACT_RECORD").is_ok_and(|v| v == "1");
    let bless = std::env::var("CONTRACT_BLESS").is_ok_and(|v| v == "1");
    let filter = std::env::var("CONTRACT_FILTER").ok();
    run_binary_directory(&directory, &command, filter.as_deref(), record, bless).unwrap();
}
