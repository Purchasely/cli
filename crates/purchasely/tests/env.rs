//! The process environment of the real binary.

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::process::Command;

/// TS reads `process.env`, which decodes each value: an unrelated variable with an invalid UTF-8 byte
/// changes nothing, and its value never reaches a diagnostic.
#[test]
fn a_non_utf8_variable_does_not_abort_or_leak() {
    let out = Command::new(env!("CARGO_BIN_EXE_purchasely"))
        .arg("--version")
        .env("PURCHASELY_DISABLE_AUTOUPDATE", "1")
        .env("UNRELATED_SECRET", OsStr::from_bytes(b"s3cr3t-\xff-value"))
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        env!("CARGO_PKG_VERSION")
    );
    assert!(!stderr.contains("s3cr3t"), "{stderr}");
}

/// Node decodes argv the same way: an invalid byte in an argument is no panic and no echo of the value.
#[test]
fn a_non_utf8_argument_does_not_abort_or_leak() {
    let home = std::env::temp_dir().join(format!("purchasely-env-{}", std::process::id()));
    std::fs::create_dir_all(&home).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_purchasely"))
        .args(["search".as_ref(), OsStr::from_bytes(b"s3cr3t-\xff")])
        .env_clear()
        .env("HOME", &home)
        .env("PURCHASELY_API_KEY", "test-key")
        .env("PURCHASELY_DISABLE_AUTOUPDATE", "1")
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&home);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert!(!stderr.contains("s3cr3t"), "{stderr}");
}
