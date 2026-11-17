use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);
const VERSION: &str = "0.9.0";
const TARGETS: [&str; 4] = ["darwin-arm64", "darwin-x64", "linux-arm64", "linux-x64"];

struct Sandbox(PathBuf);

impl Sandbox {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "purchasely-release-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        let bins = path.join("bins");
        for target in TARGETS {
            let dir = bins.join(target);
            fs::create_dir_all(&dir).unwrap();
            executable(
                &dir.join("purchasely"),
                "#!/bin/sh\nif [ \"$1\" = reference ]; then case \"$3\" in html) echo '<html>reference</html>' ;; man) echo '.TH PURCHASELY 1' ;; esac; else echo 0.9.0; fi\n",
            );
        }
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    fn pack(&self, args: &[&str]) -> Output {
        Command::new("sh")
            .arg(repo().join("scripts/pack-release.sh"))
            .args(args)
            .env("P12B_BIN_DIR", self.path("bins"))
            .output()
            .unwrap()
    }
}

fn executable(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

#[test]
fn bridge_tgz_accepted_by_old_installer() {
    let sandbox = Sandbox::new();
    let out = sandbox.path("out");
    let result = sandbox.pack(&["--bridge", out.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let archive = out.join("purchasely-0.9.0.tgz");
    assert!(archive.is_file());
    let listing = Command::new("tar")
        .args(["-tzf", archive.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(listing.status.success());
    let files = String::from_utf8_lossy(&listing.stdout);
    for expected in [
        "purchasely-0.9.0/dist/bin/purchasely.js",
        "purchasely-0.9.0/bin/purchasely",
        "purchasely-0.9.0/bin/linux-x64/purchasely",
        "purchasely-0.9.0/install.sh",
    ] {
        assert!(
            files.lines().any(|line| line == expected),
            "missing {expected}: {files}"
        );
    }
    let old_installer = repo().join("crates/contract/tests/fixtures/old-install.sh");
    let site = sandbox.path("site");
    fs::create_dir_all(&site).unwrap();
    fs::copy(&archive, site.join("purchasely-0.9.0.tgz")).unwrap();
    fs::copy(out.join("latest.json"), site.join("latest.json")).unwrap();
    fs::copy(out.join("SHA256SUMS"), site.join("SHA256SUMS")).unwrap();
    let installer = fs::read_to_string(old_installer)
        .unwrap()
        .replace("HOST=\n", "HOST=https://cli.test\n");
    let install_path = sandbox.path("old-install.sh");
    executable(&install_path, &installer);
    let fakebin = sandbox.path("fakebin");
    fs::create_dir_all(&fakebin).unwrap();
    fs::create_dir_all(sandbox.path("tmp")).unwrap();
    fs::create_dir_all(sandbox.path("home")).unwrap();
    fs::create_dir_all(sandbox.path("xdg")).unwrap();
    fs::create_dir_all(sandbox.path("bin")).unwrap();
    executable(
        &fakebin.join("curl"),
        "#!/bin/sh\nout=; prev=; url=; for a in \"$@\"; do [ \"$prev\" = -o ] && out=$a; prev=$a; url=$a; done; f=${url##*/}; cp \"$FAKE_SITE/$f\" \"$out\"\n",
    );
    let node = Command::new("which").arg("node").output().unwrap();
    let node_path = String::from_utf8_lossy(&node.stdout).trim().to_string();
    std::os::unix::fs::symlink(&node_path, fakebin.join("node")).unwrap();
    let root = sandbox.path("root");
    let result = Command::new("sh")
        .arg(&install_path)
        .args(["--version", VERSION, "--root", root.to_str().unwrap()])
        .env("PATH", format!("{}:/usr/bin:/bin", fakebin.display()))
        .env("FAKE_SITE", &site)
        .env("HOME", sandbox.path("home"))
        .env("XDG_DATA_HOME", sandbox.path("xdg"))
        .env("PURCHASELY_BIN_DIR", sandbox.path("bin"))
        .env("TMPDIR", sandbox.path("tmp"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read_to_string(root.join("current-version"))
            .unwrap()
            .trim(),
        VERSION
    );
    let binary = root.join("releases/0.9.0/bin/linux-x64/purchasely");
    let version = Command::new(binary).arg("--version").output().unwrap();
    assert_eq!(String::from_utf8_lossy(&version.stdout).trim(), VERSION);
}

#[test]
fn latest_json_window() {
    // Release metadata is rendered by the release packer from the bridge window input.
    let sandbox = Sandbox::new();
    let bridge = sandbox.path("bridge");
    let bridge_result = sandbox.pack(&["--bridge", bridge.to_str().unwrap()]);
    assert!(
        bridge_result.status.success(),
        "{}",
        String::from_utf8_lossy(&bridge_result.stderr)
    );
    let output = Command::new("sh")
        .arg(repo().join("scripts/pack-release.sh"))
        .args([
            "--version",
            "1.0.0",
            "--native",
            "1.0.0",
            "--native-release-count",
            "1",
            sandbox.path("window").to_str().unwrap(),
        ])
        .env("P12B_BIN_DIR", sandbox.path("bins"))
        .env(
            "P12B_REFERENCE_BIN",
            sandbox.path("bins/linux-x64/purchasely"),
        )
        .env("P12B_BRIDGE_ARCHIVE", bridge.join("purchasely-0.9.0.tgz"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let latest: serde_json::Value =
        serde_json::from_slice(&fs::read(sandbox.path("window/latest.json")).unwrap()).unwrap();
    assert_eq!(latest["version"], VERSION);
    assert_eq!(latest["native"], "1.0.0");
    let second = Command::new("sh")
        .arg(repo().join("scripts/pack-release.sh"))
        .args([
            "--version",
            "1.0.1",
            "--native",
            "1.0.1",
            "--native-release-count",
            "2",
            sandbox.path("second").to_str().unwrap(),
        ])
        .env("P12B_BIN_DIR", sandbox.path("bins"))
        .env(
            "P12B_REFERENCE_BIN",
            sandbox.path("bins/linux-x64/purchasely"),
        )
        .env("P12B_BRIDGE_ARCHIVE", bridge.join("purchasely-0.9.0.tgz"))
        .output()
        .unwrap();
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    let latest_second: serde_json::Value =
        serde_json::from_slice(&fs::read(sandbox.path("second/latest.json")).unwrap()).unwrap();
    assert_eq!(latest_second["version"], VERSION);
    assert_eq!(latest_second["native"], "1.0.1");
    let third = Command::new("sh")
        .arg(repo().join("scripts/pack-release.sh"))
        .args([
            "--version",
            "1.0.2",
            "--native-release-count",
            "3",
            sandbox.path("third").to_str().unwrap(),
        ])
        .env("P12B_BIN_DIR", sandbox.path("bins"))
        .env(
            "P12B_REFERENCE_BIN",
            sandbox.path("bins/linux-x64/purchasely"),
        )
        .output()
        .unwrap();
    assert!(
        third.status.success(),
        "{}",
        String::from_utf8_lossy(&third.stderr)
    );
    let latest_third: serde_json::Value =
        serde_json::from_slice(&fs::read(sandbox.path("third/latest.json")).unwrap()).unwrap();
    assert_eq!(latest_third["version"], "1.0.2");
    assert!(latest_third.get("native").is_none());
    let sums = sandbox.path("window/SHA256SUMS");
    let formula = Command::new("sh")
        .arg(repo().join("scripts/update-formula.sh"))
        .args(["1.0.0", sums.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        formula.status.success(),
        "{}",
        String::from_utf8_lossy(&formula.stderr)
    );
    let formula_path = sandbox.path("formula.rb");
    fs::write(&formula_path, formula.stdout).unwrap();
    let ruby = Command::new("ruby")
        .args(["-c", formula_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        ruby.status.success(),
        "{}",
        String::from_utf8_lossy(&ruby.stderr)
    );
}

#[test]
fn formula_skips_bridge() {
    let result = Command::new("sh")
        .arg(repo().join("scripts/update-formula.sh"))
        .arg(VERSION)
        .arg("missing-SHA256SUMS")
        .output()
        .unwrap();
    assert!(
        !result.status.success(),
        "bridge formula generation must be skipped"
    );
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("bridge release has no Homebrew formula"),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn public_tarball_holds_only_public_files() {
    let sandbox = Sandbox::new();
    let out = sandbox.path("out");
    let result = sandbox.pack(&["--bridge", out.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let archive = out.join("purchasely-0.9.0.tgz");
    let listing = Command::new("tar")
        .args(["-tzf", archive.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(listing.status.success());
    let files = String::from_utf8_lossy(&listing.stdout);
    for expected in [
        "README.md",
        "LICENSE",
        "man/purchasely.1",
        "bin/darwin-arm64/purchasely",
        "bin/darwin-x64/purchasely",
        "bin/linux-arm64/purchasely",
        "bin/linux-x64/purchasely",
    ] {
        assert!(
            files.lines().any(|line| line.ends_with(expected)),
            "missing {expected}: {files}"
        );
    }
}

#[test]
fn public_artifact_runs() {
    let sandbox = Sandbox::new();
    let out = sandbox.path("out");
    let result = sandbox.pack(&["--bridge", out.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let script = sandbox.path("unpacked");
    fs::create_dir_all(&script).unwrap();
    assert!(
        Command::new("tar")
            .args([
                "-xzf",
                out.join("purchasely-0.9.0.tgz").to_str().unwrap(),
                "-C",
                script.to_str().unwrap()
            ])
            .status()
            .unwrap()
            .success()
    );
    let binary = script.join("purchasely-0.9.0/bin/linux-x64/purchasely");
    let output = Command::new(binary).arg("--version").output().unwrap();
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), VERSION);
}

#[test]
fn site_holds_landing_and_release_files() {
    let sandbox = Sandbox::new();
    let out = sandbox.path("out");
    let result = sandbox.pack(&["--bridge", out.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    for name in [
        "index.html",
        "install.sh",
        "latest.json",
        "SHA256SUMS",
        "_headers",
        "purchasely-0.9.0.tgz",
    ] {
        assert!(out.join(name).exists(), "missing {name}");
    }
    let install = fs::read_to_string(out.join("install.sh")).unwrap();
    assert!(install.contains("BRIDGE_VERSION=0.9.0"));
    assert!(install.contains("HOST=https://cli.purchasely.io"));
    let latest: serde_json::Value =
        serde_json::from_slice(&fs::read(out.join("latest.json")).unwrap()).unwrap();
    assert_eq!(latest["version"], VERSION);
    assert_eq!(
        latest["sha256"],
        fs::read_to_string(out.join("SHA256SUMS"))
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
    );
}

#[test]
fn pack_refuses_site_headers() {
    let sandbox = Sandbox::new();
    let root = sandbox.path("repo-copy");
    fs::create_dir_all(root.join("scripts")).unwrap();
    fs::create_dir_all(root.join("site")).unwrap();
    fs::copy(
        repo().join("scripts/pack-release.sh"),
        root.join("scripts/pack-release.sh"),
    )
    .unwrap();
    fs::create_dir_all(root.join("crates/purchasely")).unwrap();
    fs::copy(
        repo().join("crates/purchasely/Cargo.toml"),
        root.join("crates/purchasely/Cargo.toml"),
    )
    .unwrap();
    fs::write(root.join("site/_headers"), "/x\n  X: y\n").unwrap();
    let output = sandbox.path("refused");
    let result = Command::new("sh")
        .arg(root.join("scripts/pack-release.sh"))
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("site/_headers"));
}

#[test]
fn reference_page_and_man_page_are_bundled() {
    let sandbox = Sandbox::new();
    let output = Command::new("sh")
        .arg(repo().join("scripts/pack-release.sh"))
        .args(["--version", "1.0.0", sandbox.path("out").to_str().unwrap()])
        .env("P12B_BIN_DIR", sandbox.path("bins"))
        .env(
            "P12B_REFERENCE_BIN",
            sandbox.path("bins/linux-x64/purchasely"),
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(sandbox.path("out/reference.html"))
            .unwrap()
            .trim(),
        "<html>reference</html>"
    );
    let man = Command::new("tar")
        .args([
            "-xzOf",
            sandbox
                .path("out/purchasely-1.0.0-linux-x64.tar.gz")
                .to_str()
                .unwrap(),
            "purchasely-1.0.0/man/purchasely.1",
        ])
        .output()
        .unwrap();
    assert!(man.status.success());
    assert_eq!(
        String::from_utf8_lossy(&man.stdout).trim(),
        ".TH PURCHASELY 1"
    );
}
