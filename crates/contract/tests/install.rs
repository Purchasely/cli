use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);
const NAME: &str = "purchasely";
const VER: &str = "1.2.3";

struct Sandbox {
    dir: PathBuf,
    home: PathBuf,
    xdg: PathBuf,
    bindir: PathBuf,
    site: PathBuf,
    fakebin: PathBuf,
    script: PathBuf,
    log: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "purchasely-install-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let home = dir.join("home");
        let xdg = dir.join("xdg");
        let bindir = dir.join("bin");
        let site = dir.join("site");
        let fakebin = dir.join("fakebin");
        for path in [&home, &xdg, &bindir, &site, &fakebin, &dir.join("tmp")] {
            fs::create_dir_all(path).unwrap();
        }
        let script = dir.join("install.sh");
        fs::write(
            &script,
            fs::read_to_string(file("install.sh"))
                .unwrap()
                .replace("HOST=\n", "HOST=https://cli.test\n"),
        )
        .unwrap();
        executable(
            &fakebin.join("curl"),
            "#!/bin/sh\necho \"$*\" >> \"$CURL_LOG\"\nout=; url=; prev=\nfor arg in \"$@\"; do [ \"$prev\" = -o ] && out=$arg; prev=$arg; url=$arg; done\nif [ -n \"${FAKE_CURL_STDERR:-}\" ]; then echo \"$FAKE_CURL_STDERR\" >&2; exit 22; fi\nf=${url##*/}; [ -f \"$FAKE_SITE/$f\" ] || exit 22; cp \"$FAKE_SITE/$f\" \"$out\"\n",
        );
        executable(
            &fakebin.join("uname"),
            "#!/bin/sh\ncase \"$1\" in -s) echo \"${FAKE_OS:-Linux}\" ;; -m) echo \"${FAKE_ARCH:-x86_64}\" ;; esac\n",
        );
        executable(
            &fakebin.join("sysctl"),
            "#!/bin/sh\n[ \"${FAKE_TRANSLATED:-0}\" = 1 ] && echo 1 || echo 0\n",
        );
        let log = dir.join("curl.log");
        Self {
            dir,
            home,
            xdg,
            bindir,
            site,
            fakebin,
            script,
            log,
        }
    }
    fn env(&self) -> Vec<(String, String)> {
        vec![
            ("HOME".into(), self.home.display().to_string()),
            ("XDG_DATA_HOME".into(), self.xdg.display().to_string()),
            (
                "PURCHASELY_BIN_DIR".into(),
                self.bindir.display().to_string(),
            ),
            ("TMPDIR".into(), self.dir.join("tmp").display().to_string()),
            ("FAKE_SITE".into(), self.site.display().to_string()),
            ("CURL_LOG".into(), self.log.display().to_string()),
            (
                "PATH".into(),
                format!("{}:/usr/bin:/bin", self.fakebin.display()),
            ),
            ("SHELL".into(), "/bin/bash".into()),
        ]
    }
    fn run(&self, args: &[&str], extra: &[(&str, &str)]) -> Output {
        if let Some(i) = args.iter().position(|a| *a == "--version")
            && let Some(version) = args
                .get(i + 1)
                .filter(|v| v.chars().next().is_some_and(|c| c.is_ascii_digit()))
        {
            self.latest(&format!("{{\"version\":\"{version}\"}}"));
        }
        let mut c = Command::new("/bin/sh");
        c.arg(&self.script)
            .args(args)
            .envs(self.env())
            .envs(extra.iter().copied());
        c.output().unwrap()
    }
    fn root(&self) -> PathBuf {
        self.xdg.join(NAME).join("install")
    }
    fn downloads(&self) -> String {
        fs::read_to_string(&self.log).unwrap_or_default()
    }
    fn releases(&self) -> Vec<String> {
        let p = self.root().join("releases");
        if !p.exists() {
            return vec![];
        }
        let mut v = fs::read_dir(p)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        v.sort();
        v
    }
    fn latest(&self, json: &str) {
        fs::write(self.site.join("latest.json"), json).unwrap();
    }
    fn native(&self, version: &str, target: &str, printed: &str) -> PathBuf {
        self.native_with_man(version, target, printed, None)
    }
    fn native_with_man(
        &self,
        version: &str,
        target: &str,
        printed: &str,
        man: Option<&str>,
    ) -> PathBuf {
        let work = self.dir.join(format!("work-{version}-{target}"));
        let top = work.join(format!("{NAME}-{version}"));
        fs::create_dir_all(top.join("bin")).unwrap();
        executable(
            &top.join("bin").join(NAME),
            &format!(
                "#!/bin/sh\n[ \"$1\" = --version ] && {{ echo {printed}; exit 0; }}\necho native:$*\n"
            ),
        );
        if let Some(page) = man {
            fs::create_dir_all(top.join("man")).unwrap();
            fs::write(top.join("man/purchasely.1"), page).unwrap();
        }
        let name = format!("{NAME}-{version}-{target}.tar.gz");
        let archive = self.site.join(&name);
        assert!(
            Command::new("tar")
                .args(["-czf"])
                .arg(&archive)
                .args(["-C"])
                .arg(&work)
                .arg(format!("{NAME}-{version}"))
                .status()
                .unwrap()
                .success()
        );
        self.sum(&name, &archive);
        self.latest(&format!("{{\"version\":\"{version}\"}}"));
        archive
    }
    fn sum(&self, name: &str, archive: &Path) {
        let out = Command::new("shasum")
            .args(["-a", "256"])
            .arg(archive)
            .output()
            .unwrap();
        assert!(out.status.success());
        let hash = String::from_utf8(out.stdout)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .to_owned();
        let file = self.site.join("SHA256SUMS");
        let mut data = fs::read_to_string(&file).unwrap_or_default();
        data.push_str(&format!("{hash}  {name}\n"));
        fs::write(file, data).unwrap();
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn file(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}
fn executable(path: &Path, data: &str) {
    fs::write(path, data).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}
fn output(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

#[test]
fn fresh_install_native() {
    let s = Sandbox::new();
    s.native(VER, "linux-x64", VER);
    s.latest("{\"version\":\"0.9.0\",\"native\":\"1.2.3\"}");
    let o = s.run(&[], &[]);
    assert!(o.status.success(), "{}", output(&o));
    assert_eq!(
        fs::read_to_string(s.root().join("current-version"))
            .unwrap()
            .trim(),
        VER
    );
    let run = Command::new(s.bindir.join(NAME))
        .arg("--version")
        .envs(s.env())
        .output()
        .unwrap();
    assert!(run.status.success());
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), VER);
    assert!(s.downloads().contains("purchasely-1.2.3-linux-x64.tar.gz"));
}

#[test]
fn target_pick() {
    for (os, arch, translated, target) in [
        ("Darwin", "arm64", "0", "darwin-arm64"),
        ("Darwin", "x86_64", "1", "darwin-arm64"),
        ("Linux", "aarch64", "0", "linux-arm64"),
        ("Linux", "x86_64", "0", "linux-x64"),
    ] {
        let s = Sandbox::new();
        s.native(VER, target, VER);
        let o = s.run(
            &[],
            &[
                ("FAKE_OS", os),
                ("FAKE_ARCH", arch),
                ("FAKE_TRANSLATED", translated),
            ],
        );
        assert!(o.status.success(), "{os}/{arch}: {}", output(&o));
        assert!(
            s.downloads()
                .contains(&format!("{NAME}-{VER}-{target}.tar.gz"))
        );
    }
    let s = Sandbox::new();
    s.native(VER, "linux-x64", VER);
    let o = s.run(&[], &[("FAKE_OS", "Plan9")]);
    assert_eq!(o.status.code(), Some(2));
    assert!(output(&o).contains("unsupported system"));
}

#[test]
fn no_node_on_path_and_native_launcher_runs() {
    let s = Sandbox::new();
    s.native(VER, "linux-x64", VER);
    let path = format!("{}:/usr/bin:/bin", s.fakebin.display());
    let o = s.run(&[], &[("PATH", &path)]);
    assert!(o.status.success(), "{}", output(&o));
    let launch = Command::new(s.bindir.join(NAME))
        .arg("update")
        .env("PATH", path)
        .output()
        .unwrap();
    assert!(launch.status.success());
    assert_eq!(
        String::from_utf8_lossy(&launch.stdout).trim(),
        "native:update"
    );
    let tools = s.dir.join("no-curl");
    fs::create_dir_all(&tools).unwrap();
    for tool in ["tar", "gzip", "uname", "shasum"] {
        let candidate = if tool == "uname" {
            s.fakebin.join(tool)
        } else {
            PathBuf::from("/usr/bin").join(tool)
        };
        if candidate.exists() {
            symlink(candidate, tools.join(tool)).unwrap();
        }
    }
    let missing = s.run(&[], &[("PATH", tools.to_str().unwrap())]);
    assert_eq!(missing.status.code(), Some(2));
    assert!(output(&missing).contains("curl is required"));
}

#[test]
fn bad_checksum_leaves_no_release_or_switch() {
    let s = Sandbox::new();
    let a = s.native(VER, "linux-x64", VER);
    fs::write(
        s.site.join("SHA256SUMS"),
        format!(
            "{}  {}\n",
            "0".repeat(64),
            a.file_name().unwrap().to_string_lossy()
        ),
    )
    .unwrap();
    let o = s.run(&[], &[]);
    assert_eq!(o.status.code(), Some(1));
    assert!(!s.root().join("current-version").exists());
    assert!(s.releases().is_empty());
}

#[test]
fn smoke_failure_preserves_current_release() {
    let s = Sandbox::new();
    s.native("1.0.0", "linux-x64", "1.0.0");
    assert!(s.run(&["--version", "1.0.0"], &[]).status.success());
    s.native("1.2.3", "linux-x64", "0.0.0");
    let o = s.run(&["--version", "1.2.3"], &[]);
    assert_eq!(o.status.code(), Some(1));
    assert!(output(&o).contains("smoke test failed"));
    assert_eq!(
        fs::read_to_string(s.root().join("current-version"))
            .unwrap()
            .trim(),
        "1.0.0"
    );
}

#[test]
fn already_current_does_not_download_again() {
    let s = Sandbox::new();
    s.native(VER, "linux-x64", VER);
    assert!(s.run(&[], &[]).status.success());
    let before = asset_download_count(&s);
    let o = s.run(&[], &[]);
    assert!(o.status.success(), "{}", output(&o));
    assert!(output(&o).contains("up to date"));
    assert_eq!(asset_download_count(&s), before);
}

#[test]
fn prune_keeps_three_newest_current_and_keep() {
    let s = Sandbox::new();
    for v in ["1.0.0", "1.1.0", "1.2.0", "1.3.0", "1.4.0", "1.5.0"] {
        s.native(v, "linux-x64", v);
    }
    for v in ["1.0.0", "1.1.0", "1.2.0", "1.3.0"] {
        let o = s.run(&["--version", v], &[]);
        assert!(o.status.success(), "{}", output(&o));
    }
    assert_eq!(s.releases(), ["1.1.0", "1.2.0", "1.3.0"]);
    let o = s.run(&["--version", "1.4.0", "--keep", "1.1.0"], &[]);
    assert!(o.status.success(), "{}", output(&o));
    assert_eq!(s.releases(), ["1.1.0", "1.2.0", "1.3.0", "1.4.0"]);
    let o = s.run(&["--version", "1.5.0", "--keep", "1.1.0"], &[]);
    assert!(o.status.success(), "{}", output(&o));
    assert_eq!(s.releases(), ["1.1.0", "1.3.0", "1.4.0", "1.5.0"]);
}

#[test]
fn never_prunes_current_when_current_is_old() {
    let s = Sandbox::new();
    for v in ["1.1.0", "1.2.0", "1.3.0", "1.4.0"] {
        s.native(v, "linux-x64", v);
    }
    for v in ["1.2.0", "1.3.0", "1.4.0", "1.1.0"] {
        let o = s.run(&["--version", v], &[]);
        assert!(o.status.success(), "{}", output(&o));
    }
    assert_eq!(
        fs::read_to_string(s.root().join("current-version"))
            .unwrap()
            .trim(),
        "1.1.0"
    );
    assert_eq!(s.releases(), ["1.1.0", "1.2.0", "1.3.0", "1.4.0"]);
}

#[test]
fn upgrade_only_never_downgrades() {
    let s = Sandbox::new();
    s.native("1.1.0", "linux-x64", "1.1.0");
    s.native("1.2.0", "linux-x64", "1.2.0");
    assert!(s.run(&["--version", "1.2.0"], &[]).status.success());
    let o = s.run(&["--version", "1.1.0", "--upgrade-only"], &[]);
    assert!(o.status.success(), "{}", output(&o));
    assert_eq!(
        fs::read_to_string(s.root().join("current-version"))
            .unwrap()
            .trim(),
        "1.2.0"
    );
}

#[test]
fn custom_root_launcher_preserves_managed_root() {
    let s = Sandbox::new();
    s.native(VER, "linux-x64", VER);
    let root = s.dir.join("custom-root");
    let o = s.run(
        &["--version", VER],
        &[("PURCHASELY_INSTALL_ROOT", root.to_str().unwrap())],
    );
    assert!(o.status.success(), "{}", output(&o));
    assert_eq!(
        fs::read_to_string(root.join("current-version"))
            .unwrap()
            .trim(),
        VER
    );
    assert!(!s.root().join("current-version").exists());
}

#[test]
fn log_file_captures_output() {
    let s = Sandbox::new();
    s.native(VER, "linux-x64", VER);
    let log = s.dir.join("install.log");
    let o = s.run(&["--log", log.to_str().unwrap()], &[]);
    assert!(o.status.success(), "{}", output(&o));
    assert!(o.stdout.is_empty());
    assert!(fs::read_to_string(log).unwrap().contains("installed"));
}

#[test]
fn rejects_escaping_versions() {
    let s = Sandbox::new();
    s.latest("{\"version\":\"1.2.3\"}");
    for bad in ["..", "../x", "1.0/../..", "v", "-1"] {
        let o = s.run(&["--version", bad], &[]);
        assert_eq!(o.status.code(), Some(2), "{}", output(&o));
    }
    assert_eq!(asset_download_count(&s), 0);
}

#[test]
fn public_channel_uses_curl_and_refuses_old_release() {
    let s = Sandbox::new();
    s.native(VER, "linux-x64", VER);
    assert!(s.run(&[], &[]).status.success());
    assert!(s.downloads().contains("--proto =https --tlsv1.2 -fsSL"));
    s.latest("{\"version\":\"1.2.3\"}");
    let o = Command::new("/bin/sh")
        .arg(&s.script)
        .args(["--version", "1.0.0"])
        .envs(s.env())
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(output(&o).contains("only the latest version"));
}

#[test]
fn curl_errors_are_fixed_and_redacted() {
    let s = Sandbox::new();
    s.native(VER, "linux-x64", VER);
    let o = s.run(&[], &[("FAKE_CURL_STDERR", "SENSITIVE_ERROR")]);
    assert_eq!(o.status.code(), Some(1));
    assert!(output(&o).contains("download failed from https://cli.test"));
    assert!(!output(&o).contains("SENSITIVE_ERROR"));
}

#[test]
fn bridge_layout_counts_as_present() {
    let s = Sandbox::new();
    let path = s.root().join(format!("releases/{VER}/bin"));
    fs::create_dir_all(&path).unwrap();
    executable(&path.join(NAME), "#!/bin/sh\necho 1.2.3\n");
    fs::write(s.root().join("current-version"), format!("{VER}\n")).unwrap();
    s.latest("{\"version\":\"1.2.3\"}");
    let o = s.run(&[], &[]);
    assert!(o.status.success(), "{}", output(&o));
    assert!(!s.downloads().contains(".tar.gz"));
}

#[test]
fn relaunch_writes_native_launcher_without_network() {
    let s = Sandbox::new();
    let path = s.root().join("releases/0.9.0/bin");
    fs::create_dir_all(&path).unwrap();
    executable(
        &path.join(NAME),
        "#!/bin/sh\nprintf 'native:%s\\n' \"$*\"\n",
    );
    fs::write(s.root().join("current-version"), "0.9.0\n").unwrap();
    let o = s.run(&["--relaunch", "--root", s.root().to_str().unwrap()], &[]);
    assert!(o.status.success(), "{}", output(&o));
    let run = Command::new(s.root().join("bin/purchasely"))
        .arg("hello")
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "native:hello");
    assert!(!s.log.exists());
}

#[test]
fn relaunch_does_nothing_without_executable_release() {
    let s = Sandbox::new();
    fs::create_dir_all(s.root().join("releases/0.9.0/bin")).unwrap();
    fs::write(s.root().join("current-version"), "0.9.0\n").unwrap();
    let o = s.run(&["--relaunch", "--root", s.root().to_str().unwrap()], &[]);
    assert!(o.status.success());
    assert!(!s.root().join("bin/purchasely").exists());
    assert!(!s.root().join("staging/lock").exists());
}

#[test]
fn relaunch_returns_three_when_lock_held() {
    let s = Sandbox::new();
    let lock = s.root().join("staging/lock");
    fs::create_dir_all(&lock).unwrap();
    fs::write(lock.join("pid"), "held\n").unwrap();
    let o = s.run(&["--relaunch", "--root", s.root().to_str().unwrap()], &[]);
    assert_eq!(o.status.code(), Some(3));
    assert_eq!(fs::read_to_string(lock.join("pid")).unwrap(), "held\n");
}

#[test]
fn old_installer_accepts_bridge() {
    if !node_supported() {
        eprintln!("skip: node 22.12 or newer is not available");
        return;
    }
    let s = Sandbox::new();
    bridge_archive(&s);
    s.latest("{\"version\":\"0.9.0\",\"sha256\":\"unused\",\"native\":\"1.0.0\"}");
    let old = file("crates/contract/tests/fixtures/old-install.sh");
    let old_script = s.dir.join("old-install.sh");
    fs::write(
        &old_script,
        fs::read_to_string(old)
            .unwrap()
            .replace("HOST=", "HOST=https://cli.test"),
    )
    .unwrap();
    let root = s.root();
    let old_path = format!(
        "{}:{}",
        s.fakebin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let o = Command::new("/bin/sh")
        .arg(old_script)
        .args(["--version", "0.9.0", "--upgrade-only"])
        .envs(s.env())
        .env("PATH", old_path)
        .env("PURCHASELY_MANAGED_INSTALL_ROOT", &root)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", output(&o));
    assert_eq!(
        fs::read_to_string(root.join("current-version"))
            .unwrap()
            .trim(),
        "0.9.0"
    );
    let bridge_path = format!(
        "{}:{}",
        s.fakebin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let launch = Command::new(root.join("bin/purchasely"))
        .arg("--version")
        .envs(s.env())
        .env("PATH", &bridge_path)
        .env("PURCHASELY_MANAGED_INSTALL_ROOT", &root)
        .output()
        .unwrap();
    assert!(launch.status.success(), "{}", output(&launch));
    assert_eq!(String::from_utf8_lossy(&launch.stdout).trim(), "0.9.0");
    let native_path = format!("{}:/usr/bin:/bin", s.fakebin.display());
    let launch = Command::new(root.join("bin/purchasely"))
        .arg("update")
        .env("PATH", native_path)
        .output()
        .unwrap();
    assert!(launch.status.success(), "{}", output(&launch));
    assert!(
        fs::read_to_string(root.join("bin/purchasely"))
            .unwrap()
            .contains("exec \"$root/releases/$v/bin/$name\"")
    );
}

#[test]
fn fresh_install_bridge_only() {
    let s = Sandbox::new();
    bridge_archive(&s);
    s.latest("{\"version\":\"0.9.0\",\"sha256\":\"unused\"}");
    let script = fs::read_to_string(&s.script)
        .unwrap()
        .replace("BRIDGE_VERSION=\n", "BRIDGE_VERSION=0.9.0\n");
    fs::write(&s.script, script).unwrap();
    let o = s.run(&[], &[]);
    assert!(o.status.success(), "{}", output(&o));
    assert!(s.downloads().contains("purchasely-0.9.0.tgz"));
    assert_eq!(
        fs::read_to_string(s.root().join("current-version"))
            .unwrap()
            .trim(),
        "0.9.0"
    );
}

#[test]
fn shim_relaunch_custom_root() {
    let s = Sandbox::new();
    bridge_archive(&s);
    s.latest("{\"version\":\"0.9.0\"}");
    let script = fs::read_to_string(&s.script)
        .unwrap()
        .replace("BRIDGE_VERSION=\n", "BRIDGE_VERSION=0.9.0\n");
    fs::write(&s.script, script).unwrap();
    let root = s.dir.join("managed");
    let o = s.run(&[], &[("PURCHASELY_INSTALL_ROOT", root.to_str().unwrap())]);
    assert!(o.status.success(), "{}", output(&o));
    assert!(root.join("bin/purchasely").exists());
    let shim = root.join("releases/0.9.0/dist/bin/purchasely.js");
    fs::create_dir_all(shim.parent().unwrap()).unwrap();
    fs::copy(file("packaging/legacy/purchasely.js"), &shim).unwrap();
    fs::write(root.join("current-version"), "0.9.0\n").unwrap();
    fs::write(root.join("bin/purchasely"),"#!/bin/sh\nexec node \"$PURCHASELY_MANAGED_INSTALL_ROOT/releases/0.9.0/dist/bin/purchasely.js\" \"$@\"\n").unwrap();
    fs::set_permissions(
        root.join("bin/purchasely"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    let path = format!(
        "{}:{}",
        s.fakebin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let launch = Command::new(root.join("bin/purchasely"))
        .arg("update")
        .env("PATH", path)
        .env("PURCHASELY_MANAGED_INSTALL_ROOT", &root)
        .output()
        .unwrap();
    assert!(launch.status.success(), "{}", output(&launch));
    assert!(
        fs::read_to_string(root.join("bin/purchasely"))
            .unwrap()
            .contains("exec \"$root/releases/$v/bin/$name\"")
    );
}

#[test]
fn packed_site_install() {
    let s = Sandbox::new();
    s.native(VER, "linux-x64", VER);
    let o = s.run(&[], &[]);
    assert!(o.status.success(), "{}", output(&o));
    assert!(
        s.root()
            .join(format!("releases/{VER}/bin/{NAME}"))
            .is_file()
    );
}

#[test]
fn installer_lock_live_pid_blocks() {
    let s = Sandbox::new();
    s.native(VER, "linux-x64", VER);
    let lock = s.root().join("staging/lock");
    fs::create_dir_all(&lock).unwrap();
    fs::write(lock.join("pid"), format!("{}\n", std::process::id())).unwrap();
    let o = s.run(&[], &[]);
    assert_eq!(o.status.code(), Some(3));
    assert!(lock.exists());
    assert_eq!(asset_download_count(&s), 0);
    fs::write(lock.join("pid"), "99999999\n").unwrap();
    let stale = s.run(&[], &[]);
    assert!(stale.status.success(), "{}", output(&stale));
    assert!(!lock.exists());
}

#[test]
fn missing_gzip_is_a_prerequisite_error() {
    let s = Sandbox::new();
    let path = s.dir.join("without-gzip");
    fs::create_dir_all(&path).unwrap();
    executable(&path.join("tar"), "#!/bin/sh\nexit 0\n");
    let o = s.run(&[], &[("PATH", path.to_str().unwrap())]);
    assert_eq!(o.status.code(), Some(2));
    assert!(output(&o).contains("tar and gzip are required"));
}

#[test]
fn truncated_script_runs_nothing() {
    let s = Sandbox::new();
    let source = fs::read_to_string(&s.script).unwrap();
    let cut = source
        .lines()
        .take(source.lines().count() / 2)
        .collect::<Vec<_>>()
        .join("\n");
    let file = s.dir.join("cut.sh");
    fs::write(&file, cut).unwrap();
    let o = Command::new("/bin/sh")
        .arg(file)
        .envs(s.env())
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert!(!s.log.exists());
    assert!(!s.root().exists());
}

#[test]
fn bin_dir_already_on_path_has_no_profile_edit() {
    let s = Sandbox::new();
    s.native(VER, "linux-x64", VER);
    fs::create_dir_all(s.home.join("bin")).unwrap();
    let path = format!(
        "{}:{}:/usr/bin:/bin",
        s.home.join("bin").display(),
        s.fakebin.display()
    );
    let o = s.run(&[], &[("PURCHASELY_BIN_DIR", ""), ("PATH", &path)]);
    assert!(o.status.success(), "{}", output(&o));
    assert!(s.home.join("bin/purchasely").is_symlink());
    assert!(!s.home.join(".bashrc").exists());
}

#[test]
fn profile_path_added_once() {
    let s = Sandbox::new();
    s.native(VER, "linux-x64", VER);
    let first = s.run(&[], &[("PURCHASELY_BIN_DIR", ""), ("SHELL", "/bin/bash")]);
    assert!(first.status.success(), "{}", output(&first));
    let second = s.run(&[], &[("PURCHASELY_BIN_DIR", ""), ("SHELL", "/bin/bash")]);
    assert!(second.status.success(), "{}", output(&second));
    let profile = fs::read_to_string(s.home.join(".bashrc")).unwrap();
    assert_eq!(
        profile
            .matches("# added by the purchasely installer")
            .count(),
        1
    );
}

#[test]
fn fish_shell_uses_profile() {
    let s = Sandbox::new();
    s.native(VER, "linux-x64", VER);
    let o = s.run(
        &[],
        &[("PURCHASELY_BIN_DIR", ""), ("SHELL", "/usr/bin/fish")],
    );
    assert!(o.status.success(), "{}", output(&o));
    assert!(s.home.join(".profile").exists());
    assert!(!s.home.join(".bashrc").exists());
}

#[test]
fn man_page_installs_and_updates() {
    let s = Sandbox::new();
    s.native_with_man("1.0.0", "linux-x64", "1.0.0", Some("page-one\n"));
    s.native_with_man("1.1.0", "linux-x64", "1.1.0", Some("page-two\n"));
    let o = s.run(&["--version", "1.0.0"], &[]);
    assert!(o.status.success(), "{}", output(&o));
    let page = s.xdg.join("man/man1/purchasely.1");
    assert_eq!(fs::read_to_string(&page).unwrap(), "page-one\n");
    let o = s.run(&["--version", "1.1.0"], &[]);
    assert!(o.status.success(), "{}", output(&o));
    assert_eq!(fs::read_to_string(page).unwrap(), "page-two\n");
}

#[test]
fn man_page_absence_is_noop() {
    let s = Sandbox::new();
    s.native(VER, "linux-x64", VER);
    let o = s.run(&[], &[]);
    assert!(o.status.success(), "{}", output(&o));
    assert!(!s.xdg.join("man").exists());
}

#[test]
fn man_page_repairs_same_version() {
    let s = Sandbox::new();
    s.native_with_man(VER, "linux-x64", VER, Some("correct\n"));
    assert!(s.run(&[], &[]).status.success());
    let page = s.xdg.join("man/man1/purchasely.1");
    fs::write(&page, "changed\n").unwrap();
    let o = s.run(&[], &[]);
    assert!(o.status.success(), "{}", output(&o));
    assert_eq!(fs::read_to_string(page).unwrap(), "correct\n");
}

#[test]
fn man_page_upgrade_only_restores_page() {
    let s = Sandbox::new();
    s.native_with_man(VER, "linux-x64", VER, Some("page\n"));
    assert!(s.run(&[], &[]).status.success());
    let page = s.xdg.join("man/man1/purchasely.1");
    fs::remove_file(&page).unwrap();
    let o = s.run(&["--version", VER, "--upgrade-only"], &[]);
    assert!(o.status.success(), "{}", output(&o));
    assert_eq!(fs::read_to_string(page).unwrap(), "page\n");
    assert!(!output(&o).contains("MANPATH"));
}

#[test]
fn man_page_copy_failure_is_nonfatal() {
    let s = Sandbox::new();
    s.native_with_man(VER, "linux-x64", VER, Some("page\n"));
    fs::write(s.xdg.join("man"), "block\n").unwrap();
    let o = s.run(&[], &[]);
    assert!(o.status.success(), "{}", output(&o));
    assert!(output(&o).contains("warning: could not install the man page"));
    fs::remove_file(s.xdg.join("man")).unwrap();
    let retry = s.run(&[], &[]);
    assert!(retry.status.success());
    assert_eq!(
        fs::read_to_string(s.xdg.join("man/man1/purchasely.1")).unwrap(),
        "page\n"
    );
}

#[test]
fn man_page_searches_with_bin_path() {
    let s = Sandbox::new();
    s.native_with_man(VER, "linux-x64", VER, Some("page\n"));
    let found = s.xdg.join("man/man1/purchasely.1");
    let fake = s.fakebin.join("man");
    executable(
        &fake,
        "#!/bin/sh\necho \"$*|$PATH\" >> \"$MAN_LOG\"\necho \"$FAKE_MAN_FOUND\"\n",
    );
    let log = s.dir.join("man.log");
    let o = s.run(
        &[],
        &[
            ("MAN_LOG", log.to_str().unwrap()),
            ("FAKE_MAN_FOUND", found.to_str().unwrap()),
        ],
    );
    assert!(o.status.success(), "{}", output(&o));
    assert!(!output(&o).contains("MANPATH"));
    let lines = fs::read_to_string(log).unwrap();
    assert!(lines.starts_with(&format!("-w purchasely|{}:", s.bindir.display())));
}

#[test]
fn manpath_empty_components_use_bin_layout() {
    let s = Sandbox::new();
    s.native_with_man(VER, "linux-x64", VER, Some("page\n"));
    let bindir = s.dir.join("derived/bin");
    let xdg = s.dir.join("derived/share");
    for value in [
        "",
        ":/usr/share/man",
        "/usr/share/man:",
        "/usr/share/man::/opt/man",
    ] {
        let o = s.run(
            &[],
            &[
                ("PURCHASELY_BIN_DIR", bindir.to_str().unwrap()),
                ("XDG_DATA_HOME", xdg.to_str().unwrap()),
                ("MANPATH", value),
            ],
        );
        assert!(o.status.success(), "{}", output(&o));
        assert!(!output(&o).contains("MANPATH"));
    }
}

#[test]
fn manpath_nonempty_excludes_bin_layout() {
    let s = Sandbox::new();
    s.native_with_man(VER, "linux-x64", VER, Some("page\n"));
    let bindir = s.dir.join("derived/bin");
    let xdg = s.dir.join("derived/share");
    for value in ["/usr/share/man", "/usr/share/man:/opt/man"] {
        let o = s.run(
            &[],
            &[
                ("PURCHASELY_BIN_DIR", bindir.to_str().unwrap()),
                ("XDG_DATA_HOME", xdg.to_str().unwrap()),
                ("MANPATH", value),
            ],
        );
        assert!(o.status.success(), "{}", output(&o));
        assert!(output(&o).contains("MANPATH"));
    }
}

#[test]
fn manpath_explicit_man_dir_counts() {
    let s = Sandbox::new();
    s.native_with_man(VER, "linux-x64", VER, Some("page\n"));
    let xdg = s.dir.join("x/share");
    let bindir = s.dir.join("x/bin");
    let mandir = xdg.join("man");
    let o = s.run(
        &[],
        &[
            ("PURCHASELY_BIN_DIR", bindir.to_str().unwrap()),
            ("XDG_DATA_HOME", xdg.to_str().unwrap()),
            ("MANPATH", mandir.to_str().unwrap()),
        ],
    );
    assert!(o.status.success(), "{}", output(&o));
    assert!(!output(&o).contains("MANPATH"));
}

fn bridge_archive(s: &Sandbox) {
    let work = s.dir.join("bridge-work");
    let top = work.join("purchasely-0.9.0");
    let bin = top.join("bin");
    for target in ["darwin-arm64", "darwin-x64", "linux-arm64", "linux-x64"] {
        let p = bin.join(target);
        fs::create_dir_all(&p).unwrap();
        executable(
            &p.join(NAME),
            "#!/bin/sh\n[ \"$1\" = --version ] && echo 0.9.0 || echo native:$*\n",
        );
    }
    fs::create_dir_all(top.join("dist/bin")).unwrap();
    fs::copy(file("packaging/legacy/dispatch.sh"), bin.join(NAME)).unwrap();
    fs::set_permissions(bin.join(NAME), fs::Permissions::from_mode(0o755)).unwrap();
    fs::copy(
        file("packaging/legacy/purchasely.js"),
        top.join("dist/bin/purchasely.js"),
    )
    .unwrap();
    let install = fs::read_to_string(file("install.sh"))
        .unwrap()
        .replace("HOST=\n", "HOST=https://cli.test\n")
        .replace("BRIDGE_VERSION=\n", "BRIDGE_VERSION=0.9.0\n");
    fs::write(top.join("install.sh"), install).unwrap();
    let name = "purchasely-0.9.0.tgz";
    let archive = s.site.join(name);
    assert!(
        Command::new("tar")
            .args(["-czf"])
            .arg(&archive)
            .args(["-C"])
            .arg(&work)
            .arg("purchasely-0.9.0")
            .status()
            .unwrap()
            .success()
    );
    s.sum(name, &archive);
}

fn node_supported() -> bool {
    Command::new("node").args(["-e","const [a,b]=process.versions.node.split('.').map(Number);process.exit(a>22||(a===22&&b>=12)?0:1)"]).status().map(|s|s.success()).unwrap_or(false)
}

fn asset_download_count(s: &Sandbox) -> usize {
    s.downloads()
        .lines()
        .filter(|line| line.contains(".tar.gz") || line.contains(".tgz"))
        .count()
}
