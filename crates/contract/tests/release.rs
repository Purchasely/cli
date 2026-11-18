use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);
const VERSION: &str = "0.9.0";
const TARGETS: [&str; 4] = ["darwin-arm64", "darwin-x64", "linux-arm64", "linux-x64"];

struct Sandbox(PathBuf);

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

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
        "purchasely-0.9.0/package.json",
    ] {
        assert!(
            files.lines().any(|line| line == expected),
            "missing {expected}: {files}"
        );
    }
    // A package.json with "type": "module" above the install root (~/package.json) must not turn the
    // CommonJS shim into an ES module: the bridge ships its own "type": "commonjs".
    fs::write(sandbox.path("package.json"), r#"{"type":"module"}"#).unwrap();
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

/// The `run:` script of the step `name` in a workflow, dedented.
fn step_script(workflow: &str, name: &str) -> String {
    let lines: Vec<&str> = workflow.lines().collect();
    let at = lines
        .iter()
        .position(|l| l.trim() == format!("- name: {name}"))
        .unwrap_or_else(|| panic!("no step {name}"));
    let run = (at..lines.len())
        .find(|&i| lines[i].trim() == "run: |")
        .unwrap();
    let indent = |l: &str| l.len() - l.trim_start().len();
    let body: Vec<&str> = lines[run + 1..]
        .iter()
        .take_while(|l| l.trim().is_empty() || indent(l) > indent(lines[run]))
        .copied()
        .collect();
    let cut = body
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| indent(l))
        .min()
        .unwrap();
    body.iter()
        .map(|l| l.get(cut..).unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The jobs of a workflow, by name, with their text.
fn jobs(workflow: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut inside = false;
    for line in workflow.lines() {
        if line == "jobs:" {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        let name = line
            .strip_prefix("  ")
            .filter(|l| !l.starts_with(' ') && l.ends_with(':'));
        match name {
            Some(n) => out.push((n.trim_end_matches(':').to_owned(), String::new())),
            None => {
                if let Some((_, text)) = out.last_mut() {
                    text.push_str(line);
                    text.push('\n');
                }
            }
        }
    }
    out
}

/// A job that runs `gh release` outside a checkout must name its repository, or gh stops with
/// "not a git repository".
#[test]
fn release_jobs_give_gh_its_repository() {
    for file in ["release.yml", "build.yml"] {
        let workflow = fs::read_to_string(repo().join(".github/workflows").join(file)).unwrap();
        for (job, text) in jobs(&workflow) {
            if text.contains("gh release ") {
                assert!(
                    text.contains("actions/checkout@")
                        || text.contains("GH_REPO: ${{ github.repository }}"),
                    "{file} job {job} runs gh release without a repository"
                );
            }
        }
    }
}

/// The pack step of build.yml, run as on a runner: the window build downloads the immutable v0.9.0
/// bridge next to a freshly packed one, and ships the downloaded one.
#[test]
fn bridge_window_step_ships_the_published_bridge() {
    let sandbox = Sandbox::new();
    let work = sandbox.path("work");
    fs::create_dir_all(&work).unwrap();
    for entry in fs::read_dir(repo()).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name != "out" && name != "target" && name != ".git" {
            std::os::unix::fs::symlink(entry.path(), work.join(&name)).unwrap();
        }
    }
    let published = sandbox.path("published");
    fs::create_dir_all(&published).unwrap();
    fs::write(
        published.join("purchasely-0.9.0.tgz"),
        "published v0.9.0 bridge",
    )
    .unwrap();
    let fakebin = sandbox.path("fakebin");
    fs::create_dir_all(&fakebin).unwrap();
    // As gh: a download refuses to replace an existing file without --clobber.
    executable(
        &fakebin.join("gh"),
        "#!/bin/sh\n[ \"$1 $2\" = 'release download' ] || exit 1\ndest=.; pat=; prev=\nfor a in \"$@\"; do [ \"$prev\" = -D ] && dest=$a; [ \"$prev\" = -p ] && pat=$a; prev=$a; done\nmkdir -p \"$dest\"\nif [ -e \"$dest/$pat\" ]; then echo \"$dest/$pat already exists\" >&2; exit 1; fi\ncp \"$FAKE_RELEASE/$pat\" \"$dest/$pat\"\n",
    );
    let runner_temp = sandbox.path("runner-temp");
    fs::create_dir_all(&runner_temp).unwrap();
    let workflow = fs::read_to_string(repo().join(".github/workflows/build.yml")).unwrap();
    let script = step_script(&workflow, "Pack release and bridge");
    let output = Command::new("bash")
        .args(["-e", "-c", &script])
        .current_dir(&work)
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin:/usr/sbin:/sbin", fakebin.display()),
        )
        .env("FAKE_RELEASE", &published)
        .env("RUNNER_TEMP", &runner_temp)
        .env("GH_TOKEN", "dummy")
        .env("P12B_BIN_DIR", sandbox.path("bins"))
        .env(
            "P12B_REFERENCE_BIN",
            sandbox.path("bins/linux-x64/purchasely"),
        )
        .env("VERSION", "1.0.0")
        .env("PREVIOUS_NATIVE_RELEASES", "0")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(work.join("out/native/purchasely-0.9.0.tgz")).unwrap(),
        "published v0.9.0 bridge"
    );
}

/// The real-bridge acceptance step of build.yml, run as on an Ubuntu runner (dash as sh, no SHELL, no
/// token) with crate version 0.9.0: the frozen installer installs the packed bridge.
#[test]
fn bridge_acceptance_step_runs_in_a_runner_layout() {
    let which = |program: &str| {
        let out = Command::new("sh")
            .args(["-c", &format!("command -v {program}")])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    };
    let node = which("node");
    if node.is_empty() {
        eprintln!("skip: node is unavailable for the frozen installer check");
        return;
    }
    let sandbox = Sandbox::new();
    let work = sandbox.path("work");
    fs::create_dir_all(work.join("crates/purchasely")).unwrap();
    fs::create_dir_all(work.join("crates/contract/tests")).unwrap();
    for entry in fs::read_dir(repo()).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if !["out", "target", ".git", "crates"].contains(&name.to_str().unwrap()) {
            std::os::unix::fs::symlink(entry.path(), work.join(&name)).unwrap();
        }
    }
    fs::write(
        work.join("crates/purchasely/Cargo.toml"),
        "[package]\nname = \"purchasely\"\nversion = \"0.9.0\"\n",
    )
    .unwrap();
    std::os::unix::fs::symlink(
        repo().join("crates/contract/tests/fixtures"),
        work.join("crates/contract/tests/fixtures"),
    )
    .unwrap();
    let packed = sandbox.pack(&["--bridge", work.join("out/bridge").to_str().unwrap()]);
    assert!(
        packed.status.success(),
        "{}",
        String::from_utf8_lossy(&packed.stderr)
    );
    let shbin = sandbox.path("shbin");
    fs::create_dir_all(&shbin).unwrap();
    let dash = which("dash");
    if !dash.is_empty() {
        std::os::unix::fs::symlink(&dash, shbin.join("sh")).unwrap();
    }
    let runner_temp = sandbox.path("runner-temp");
    fs::create_dir_all(&runner_temp).unwrap();
    let node_dir = Path::new(&node).parent().unwrap().to_path_buf();
    let workflow = fs::read_to_string(repo().join(".github/workflows/build.yml")).unwrap();
    let script = step_script(
        &workflow,
        "Verify bridge with the frozen installer and real binaries",
    );
    let output = Command::new("bash")
        .args(["-e", "-c", &script])
        .current_dir(&work)
        .env_clear()
        .env(
            "PATH",
            format!(
                "{}:{}:/usr/bin:/bin:/usr/sbin:/sbin",
                shbin.display(),
                node_dir.display()
            ),
        )
        .env("RUNNER_TEMP", &runner_temp)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(runner_temp.join("bridge-root/current-version"))
            .unwrap()
            .trim(),
        "0.9.0"
    );
}

/// Every `uses: <action>@<sha> # <version>` pin: the same action and version must have the same commit
/// (a swapped character makes a commit that does not exist, and the job fails when it starts).
fn check_action_pins(folders: &[PathBuf]) {
    let mut seen: std::collections::BTreeMap<(String, String), (String, String)> =
        Default::default();
    let mut count = 0;
    for folder in folders {
        for entry in fs::read_dir(folder).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "yml") {
                continue;
            }
            for line in fs::read_to_string(&path).unwrap().lines() {
                let Some(rest) = line.trim().trim_start_matches("- ").strip_prefix("uses: ") else {
                    continue;
                };
                if rest.starts_with("./") {
                    continue;
                }
                let (spec, version) = rest.split_once(" # ").unwrap_or((rest, ""));
                let (action, sha) = spec.split_once('@').unwrap();
                // A tag ref (not a commit) has nothing to compare.
                if !(sha.len() == 40 && sha.bytes().all(|b| b.is_ascii_hexdigit())) {
                    continue;
                }
                count += 1;
                let key = (action.to_owned(), version.trim().to_owned());
                let here = format!("{}", path.display());
                if let Some((other, at)) = seen.get(&key) {
                    assert_eq!(
                        sha, other,
                        "{action} {} differs between {at} and {here}",
                        key.1
                    );
                } else {
                    seen.insert(key, (sha.to_owned(), here));
                }
            }
        }
    }
    assert!(count > 0);
}

#[test]
fn action_pins_agree() {
    check_action_pins(&[repo().join(".github/workflows")]);
}

/// The release count of the prepare job, with two pages of releases: gh applies --jq to each page,
/// so the step must add the per-page counts.
#[test]
fn release_count_adds_the_pages() {
    if !Command::new("sh")
        .args(["-c", "command -v jq"])
        .output()
        .unwrap()
        .status
        .success()
    {
        eprintln!("skip: jq is unavailable for the fake gh");
        return;
    }
    let sandbox = Sandbox::new();
    let pages = sandbox.path("pages");
    fs::create_dir_all(&pages).unwrap();
    let page = |n: usize, from: usize| {
        (from..from + n)
            .map(|i| format!(r#"{{"tag_name":"v1.{i}.0","draft":false,"prerelease":false}}"#))
            .collect::<Vec<_>>()
            .join(",")
    };
    fs::write(pages.join("1.json"), format!("[{}]", page(30, 0))).unwrap();
    fs::write(pages.join("2.json"), format!("[{}]", page(2, 30))).unwrap();
    let fakebin = sandbox.path("fakebin");
    fs::create_dir_all(&fakebin).unwrap();
    executable(
        &fakebin.join("gh"),
        "#!/bin/sh\nf=; prev=\nfor a in \"$@\"; do [ \"$prev\" = --jq ] && f=$a; prev=$a; done\nfor p in \"$PAGES\"/*.json; do jq -r \"$f\" \"$p\"; done\n",
    );
    let work = sandbox.path("work");
    fs::create_dir_all(work.join("packaging")).unwrap();
    fs::write(
        work.join("packaging/bridge.env"),
        "BRIDGE_VERSION=0.9.0\nBRIDGE_RELEASES=2\n",
    )
    .unwrap();
    let output_file = sandbox.path("github-output");
    let workflow = fs::read_to_string(repo().join(".github/workflows/release.yml")).unwrap();
    let o = Command::new("bash")
        .args([
            "-e",
            "-o",
            "pipefail",
            "-c",
            &step_script(&workflow, "Read the release"),
        ])
        .current_dir(&work)
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin:/opt/homebrew/bin", fakebin.display()),
        )
        .env("PAGES", &pages)
        .env("GITHUB_REF_NAME", "v1.40.0")
        .env("GITHUB_REPOSITORY", "Purchasely/x")
        .env("GITHUB_OUTPUT", &output_file)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let out = fs::read_to_string(&output_file).unwrap();
    assert!(out.contains("native_count=32\n"), "{out}");
}

/// The bridge is the Pages snapshot while 0.9.0 is the latest: it carries the reference page.
#[test]
fn bridge_pack_writes_the_reference_page() {
    let sandbox = Sandbox::new();
    let out = sandbox.path("out");
    let result = sandbox.pack(&["--bridge", out.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read_to_string(out.join("reference.html"))
            .unwrap()
            .trim(),
        "<html>reference</html>"
    );
}

/// Plan 4.2: each native archive holds README.md and LICENSE.
#[test]
fn native_archives_hold_readme_and_license() {
    let sandbox = Sandbox::new();
    let out = sandbox.path("out");
    let result = sandbox.pack(&["--version", "1.0.0", out.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    for target in TARGETS {
        let archive = out.join(format!("purchasely-1.0.0-{target}.tar.gz"));
        let listing = Command::new("tar")
            .arg("-tzf")
            .arg(&archive)
            .output()
            .unwrap();
        let files = String::from_utf8_lossy(&listing.stdout);
        for name in ["README.md", "LICENSE"] {
            assert!(
                files
                    .lines()
                    .any(|l| l == format!("purchasely-1.0.0/{name}")),
                "{target}: missing {name}: {files}"
            );
        }
    }
}
