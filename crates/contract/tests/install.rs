use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn temp_root() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "purchasely-install-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn relaunch_writes_native_launcher_without_network() {
    let root = temp_root();
    let release = root.join("releases/0.9.0/bin");
    fs::create_dir_all(&release).unwrap();
    let binary = release.join("purchasely");
    fs::write(&binary, "#!/bin/sh\nprintf 'native:%s\\n' \"$*\"\n").unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(root.join("current-version"), "0.9.0\n").unwrap();
    let installer = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../install.sh");
    let result = Command::new("sh")
        .arg(installer)
        .arg("--relaunch")
        .arg("--root")
        .arg(&root)
        .env("HOME", &root)
        .env("PURCHASELY_MANAGED_INSTALL_ROOT", &root)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let launched = Command::new(root.join("bin/purchasely"))
        .arg("hello")
        .output()
        .unwrap();
    assert!(launched.status.success());
    assert_eq!(
        String::from_utf8_lossy(&launched.stdout).trim(),
        "native:hello"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn relaunch_does_nothing_without_an_executable_release() {
    let root = temp_root();
    fs::create_dir_all(root.join("releases/0.9.0/bin")).unwrap();
    fs::write(root.join("current-version"), "0.9.0\n").unwrap();
    let installer = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../install.sh");
    let result = Command::new("sh")
        .arg(installer)
        .args(["--relaunch", "--root"])
        .arg(&root)
        .env("HOME", &root)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert!(!root.join("bin/purchasely").exists());
    assert!(!root.join("staging/lock").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn relaunch_returns_lock_status_without_touching_the_lock() {
    let root = temp_root();
    fs::create_dir_all(root.join("staging/lock")).unwrap();
    fs::write(root.join("staging/lock/pid"), "123\n").unwrap();
    let installer = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../install.sh");
    let result = Command::new("sh")
        .arg(installer)
        .args(["--relaunch", "--root"])
        .arg(&root)
        .env("HOME", &root)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(3));
    assert_eq!(
        fs::read_to_string(root.join("staging/lock/pid")).unwrap(),
        "123\n"
    );
    let _ = fs::remove_dir_all(root);
}
