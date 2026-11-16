use contract::binary::{BinaryResult, compare_recorded, run_binary_directory};
use serde_json::json;
use std::process::{Command, Stdio};

#[test]
fn binary_transcript_compares_stdout_stderr_and_status() {
    let actual = BinaryResult {
        code: 0,
        stdout: b"ready\n".to_vec(),
        stderr: vec![],
    };
    assert!(compare_recorded(&json!({"code":0,"stdout":"ready\n","stderr":""}), &actual).is_ok());
}

/// CONTRACT_BIN, else the debug Rust binary. A relative path word (`target/debug/purchasely`, the
/// `dist/bin/purchasely.js` of `node dist/bin/purchasely.js`) is relative to the workspace root; `node` is looked
/// up in PATH.
fn contract_bin() -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let command =
        std::env::var("CONTRACT_BIN").unwrap_or_else(|_| "target/debug/purchasely".into());
    let words: Vec<String> = command
        .split_whitespace()
        .map(|w| {
            if w.contains('/') && !w.starts_with('/') {
                root.join(w).to_string_lossy().into_owned()
            } else {
                w.to_owned()
            }
        })
        .collect();
    let program = words.first().map(String::as_str).unwrap_or_default();
    assert!(
        !program.contains('/') || std::path::Path::new(program).exists(),
        "missing {program}: run `cargo build -p purchasely --bins` first"
    );
    words.join(" ")
}

/// The program and its prefix arguments (`node dist/bin/purchasely.js`), with an isolated HOME.
fn bin_command(home: &std::path::Path, args: &[&str]) -> Command {
    let command = contract_bin();
    let mut words = command.split_whitespace();
    let mut cmd = Command::new(words.next().unwrap());
    cmd.args(words).args(args);
    for (name, dir) in [
        ("HOME", "home"),
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_STATE_HOME", "state"),
        ("XDG_CACHE_HOME", "cache"),
    ] {
        cmd.env(name, home.join(dir));
    }
    cmd.env("PURCHASELY_DISABLE_AUTOUPDATE", "1");
    cmd
}

fn temp_home(tag: &str) -> std::path::PathBuf {
    let home = std::env::temp_dir().join(format!("contract-bin-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&home).unwrap();
    home
}

#[test]
fn binary_contract_directory_supports_record_replay_and_bless() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contract/bin");
    let command = contract_bin();
    let record = std::env::var("CONTRACT_RECORD").is_ok_and(|v| v == "1");
    let bless = std::env::var("CONTRACT_BLESS").is_ok_and(|v| v == "1");
    let filter = std::env::var("CONTRACT_FILTER").ok();
    run_binary_directory(&directory, &command, filter.as_deref(), record, bless).unwrap();
}

/// `reference | head`: the reader closes stdout early; the CLI ends quietly with exit 0 (TS ignores EPIPE).
#[test]
fn reference_closed_stdout_exits_quietly() {
    use std::io::Read;
    let home = temp_home("epipe");
    let mut child = bin_command(&home, &["reference"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut head = [0u8; 64];
    child.stdout.take().unwrap().read_exact(&mut head).unwrap();
    // The pipe is dropped here: the rest of the reference (about 400 KB) hits a closed pipe.
    let output = child.wait_with_output().unwrap();
    let _ = std::fs::remove_dir_all(home);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        head.starts_with(b"# "),
        "{:?}",
        String::from_utf8_lossy(&head)
    );
    assert_eq!(output.status.code(), Some(0), "stderr: {stderr}");
    assert!(!stderr.contains("panicked"), "stderr: {stderr}");
}

/// `mandoc -T lint -W warning` finds nothing in the man page (as test/reference.test.ts). Skipped without mandoc,
/// a failure in CI.
#[test]
fn reference_man_page_passes_mandoc_lint() {
    if Command::new("mandoc").arg("-V").output().is_err() {
        assert!(std::env::var("CI").is_err(), "mandoc is missing in CI");
        return;
    }
    let home = temp_home("man");
    let man = bin_command(&home, &["reference", "--format", "man"])
        .output()
        .unwrap();
    assert_eq!(man.status.code(), Some(0));
    let file = home.join("purchasely.1");
    std::fs::write(&file, &man.stdout).unwrap();
    let lint = Command::new("mandoc")
        .args(["-T", "lint", "-W", "warning"])
        .arg(&file)
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(home);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&lint.stdout),
        String::from_utf8_lossy(&lint.stderr)
    );
    assert_eq!((lint.status.code(), text.as_str()), (Some(0), ""));
}

/// `docs/commands.md` is the header of `scripts/gen-docs.sh` and then `reference --format markdown`.
#[test]
fn docs_commands_md_is_the_generated_reference() {
    let home = temp_home("docs");
    let out = bin_command(&home, &["reference", "--format", "markdown"])
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(home);
    assert_eq!(out.status.code(), Some(0));
    let expected = format!(
        "<!-- Generated from purchasely reference --format markdown. Do not edit. -->\n\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/commands.md");
    let file = std::fs::read_to_string(path).unwrap();
    assert!(
        file == expected,
        "docs/commands.md is stale: run scripts/gen-docs.sh"
    );
}
