//! Owner: P1b
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT_HOME: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub struct BinaryResult {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}
/// The User-Agent runner reads these (`core::http::runner`): the recording was made without them.
const RUNNER_ENV: [&str; 5] = [
    "CLAUDECODE",
    "CODEX_THREAD_ID",
    "GEMINI_CLI",
    "CURSOR_AGENT",
    "CI",
];

fn clear_runner_env(cmd: &mut Command) {
    for name in RUNNER_ENV {
        cmd.env_remove(name);
    }
}

pub fn run_binary(
    binary: &Path,
    argv: &[String],
    env: &[(String, String)],
    stdin: &[u8],
) -> std::io::Result<BinaryResult> {
    let mut cmd = Command::new(binary);
    cmd.args(argv);
    clear_runner_env(&mut cmd);
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.env("PURCHASELY_DISABLE_AUTOUPDATE", "1");
    let output: Output = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(mut pipe) = child.stdin.take() {
                pipe.write_all(stdin)?;
            }
            child.wait_with_output()
        })?;
    Ok(BinaryResult {
        code: output.status.code().unwrap_or(128),
        stdout: output.stdout,
        stderr: output.stderr,
    })
}
pub fn run_binary_case(binary: &Path, case: &Value) -> std::io::Result<BinaryResult> {
    let root = std::env::temp_dir().join(format!(
        "contract-bin-{}-{}",
        std::process::id(),
        NEXT_HOME.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root)?;
    let result = (|| {
        let mut env: BTreeMap<String, String> = case["env"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_owned())))
            .collect();
        env.insert(
            "HOME".into(),
            root.join("home").to_string_lossy().into_owned(),
        );
        env.insert(
            "XDG_CONFIG_HOME".into(),
            root.join("config").to_string_lossy().into_owned(),
        );
        env.insert(
            "XDG_STATE_HOME".into(),
            root.join("state").to_string_lossy().into_owned(),
        );
        env.insert(
            "XDG_CACHE_HOME".into(),
            root.join("cache").to_string_lossy().into_owned(),
        );
        for dir in ["home", "config", "state", "cache"] {
            fs::create_dir_all(root.join(dir))?;
        }
        let argv: Vec<String> = serde_json::from_value(case["argv"].clone()).unwrap_or_default();
        let stdin = case["stdin"].as_str().unwrap_or_default().as_bytes();
        let mut result = run_binary(binary, &argv, &env.into_iter().collect::<Vec<_>>(), stdin)?;
        result.stdout = normalize_bytes(&result.stdout, &root);
        result.stderr = normalize_bytes(&result.stderr, &root);
        Ok(result)
    })();
    let _ = fs::remove_dir_all(root);
    result
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BinaryReport {
    pub run: usize,
    pub pending: usize,
    pub filtered: usize,
}
pub fn run_binary_cases(
    cases: &mut [Value],
    command: &str,
    filter: Option<&str>,
    record: bool,
    bless: bool,
) -> Result<BinaryReport, String> {
    let refs: Vec<Value> = cases.to_vec();
    let selected = crate::replay::selected(&refs, filter)?;
    let mut report = BinaryReport {
        filtered: cases.len() - selected.len(),
        ..BinaryReport::default()
    };
    let (program, prefix) = parse_command(command)?;
    for chosen in selected {
        let index = refs.iter().position(|c| std::ptr::eq(c, chosen)).unwrap();
        if chosen.get("pending").is_some() {
            report.pending += 1;
            continue;
        }
        let actual = run_binary_with_prefix(Path::new(&program), &prefix, chosen)?;
        if bless || record {
            let case = &mut cases[index];
            case["code"] = json!(actual.code);
            set_output(case, "stdout", "out", &actual.stdout);
            set_output(case, "stderr", "err", &actual.stderr);
            if bless {
                case["source"] = json!("bless");
            }
        } else {
            compare_recorded(chosen, &actual)
                .map_err(|e| format!("{}: {e}", chosen["test"].as_str().unwrap_or("<unnamed>")))?;
        }
        report.run += 1;
    }
    eprintln!(
        "run {}, pending {}, filtered {}",
        report.run, report.pending, report.filtered
    );
    Ok(report)
}
pub fn run_binary_directory(
    directory: &Path,
    command: &str,
    filter: Option<&str>,
    record: bool,
    bless: bool,
) -> Result<BinaryReport, String> {
    let mut files = Vec::<(PathBuf, Vec<Value>)>::new();
    if directory.exists() {
        for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let value: Value = serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            let cases = value["cases"].as_array().cloned().unwrap_or_default();
            files.push((path, cases));
        }
    }
    let mut flat = files
        .iter()
        .flat_map(|(_, cases)| cases.iter().cloned())
        .collect::<Vec<_>>();
    let report = run_binary_cases(&mut flat, command, filter, record, bless)?;
    if record || bless {
        let mut offset = 0;
        for (path, cases) in &mut files {
            for item in cases.iter_mut() {
                *item = flat[offset].clone();
                offset += 1;
            }
            let value = json!({"cases":cases});
            fs::write(
                path,
                serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(report)
}
fn run_binary_with_prefix(
    binary: &Path,
    prefix: &[String],
    case: &Value,
) -> Result<BinaryResult, String> {
    let root = std::env::temp_dir().join(format!(
        "contract-bin-{}-{}",
        std::process::id(),
        NEXT_HOME.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let result = (|| {
        let mut cmd = Command::new(binary);
        cmd.args(prefix);
        let argv: Vec<String> = serde_json::from_value(case["argv"].clone()).unwrap_or_default();
        cmd.args(argv);
        clear_runner_env(&mut cmd);
        if let Some(env) = case["env"].as_object() {
            for (k, v) in env {
                if let Some(v) = v.as_str() {
                    cmd.env(k, v);
                }
            }
        }
        cmd.env("PURCHASELY_DISABLE_AUTOUPDATE", "1");
        for (name, path) in [
            ("HOME", "home"),
            ("XDG_CONFIG_HOME", "config"),
            ("XDG_STATE_HOME", "state"),
            ("XDG_CACHE_HOME", "cache"),
        ] {
            let path = root.join(path);
            fs::create_dir_all(&path).map_err(|e| e.to_string())?;
            cmd.env(name, path);
        }
        let output: Output = cmd
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?
            .tap_stdin(case["stdin"].as_str().unwrap_or_default().as_bytes())
            .map_err(|e| e.to_string())?;
        Ok(BinaryResult {
            code: output.status.code().unwrap_or(128),
            stdout: normalize_bytes(&output.stdout, &root),
            stderr: normalize_bytes(&output.stderr, &root),
        })
    })();
    let _ = fs::remove_dir_all(root);
    result
}
trait WriteAndWait {
    fn tap_stdin(self, bytes: &[u8]) -> std::io::Result<Output>;
}
impl WriteAndWait for std::process::Child {
    fn tap_stdin(mut self, bytes: &[u8]) -> std::io::Result<Output> {
        use std::io::Write;
        if let Some(mut stdin) = self.stdin.take() {
            stdin.write_all(bytes)?;
        }
        self.wait_with_output()
    }
}
fn parse_command(command: &str) -> Result<(String, Vec<String>), String> {
    let words = command
        .split_whitespace()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if words.is_empty() {
        return Err("CONTRACT_BIN is empty".into());
    }
    Ok((words[0].clone(), words[1..].to_vec()))
}
fn set_output(case: &mut Value, key: &str, alias: &str, bytes: &[u8]) {
    let text = String::from_utf8_lossy(bytes).into_owned();
    if case.get(alias).is_some() {
        case[alias] = json!(text);
        case.as_object_mut().unwrap().remove(key);
    } else {
        case[key] = json!(text);
        case.as_object_mut().unwrap().remove(alias);
    }
}
pub fn record_path(directory: &Path, name: &str, result: &BinaryResult) -> std::io::Result<()> {
    fs::create_dir_all(directory)?;
    let val = json!({"test":name,"code":result.code,"stdout":String::from_utf8_lossy(&result.stdout),"stderr":String::from_utf8_lossy(&result.stderr)});
    fs::write(
        directory.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&val).map_err(std::io::Error::other)?,
    )
}
pub fn compare_recorded(expected: &Value, actual: &BinaryResult) -> Result<(), String> {
    let got = json!({"code":actual.code,"stdout":normalize_binary_text(&String::from_utf8_lossy(&actual.stdout),None),"stderr":normalize_binary_text(&String::from_utf8_lossy(&actual.stderr),None)});
    for (key, alias) in [("code", "code"), ("stdout", "out"), ("stderr", "err")] {
        let expected_value = expected
            .get(key)
            .or_else(|| expected.get(alias))
            .cloned()
            .map(normalize_expected);
        if expected_value.as_ref() != got.get(key) {
            return Err(format!(
                "binary {key} mismatch: expected {}, got {}",
                expected_value.unwrap_or(Value::Null),
                got[key]
            ));
        }
    }
    Ok(())
}
fn normalize_expected(value: Value) -> Value {
    if let Some(text) = value.as_str() {
        json!(normalize_binary_text(text, None))
    } else {
        value
    }
}
fn normalize_bytes(bytes: &[u8], root: &Path) -> Vec<u8> {
    normalize_binary_text(&String::from_utf8_lossy(bytes), Some(root)).into_bytes()
}
fn normalize_binary_text(text: &str, root: Option<&Path>) -> String {
    let mut value = text.to_owned();
    if let Some(root) = root {
        value = value.replace(&root.to_string_lossy().to_string(), "<ROOT>");
    }
    value = crate::normalize::replace_man_date(&value);
    value = normalize_version_tokens(&value);
    value = normalize_iso_dates(&value);
    normalize_binary_roots(&value)
}
fn normalize_version_tokens(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            let mut dots = 0;
            let mut j = i;
            while j < bytes.len() && (bytes[j].is_ascii_digit() || bytes[j] == b'.') {
                if bytes[j] == b'.' {
                    dots += 1;
                }
                j += 1;
            }
            if dots >= 2 {
                out.push_str("<VERSION>");
                i = j;
                continue;
            }
            out.push_str(&input[start..j]);
            i = j;
        } else {
            let ch = input[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}
fn normalize_iso_dates(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if i + 10 <= bytes.len()
            && bytes[i..i + 4].iter().all(u8::is_ascii_digit)
            && bytes[i + 4] == b'-'
            && bytes[i + 5..i + 7].iter().all(u8::is_ascii_digit)
            && bytes[i + 7] == b'-'
            && bytes[i + 8..i + 10].iter().all(u8::is_ascii_digit)
        {
            out.push_str("<DATE>");
            i += 10;
        } else {
            let c = input[i..].chars().next().unwrap();
            out.push(c);
            i += c.len_utf8();
        }
    }
    out
}
fn normalize_binary_roots(input: &str) -> String {
    let marker = "contract-bin-";
    let mut out = String::new();
    let mut rest = input;
    while let Some(i) = rest.find(marker) {
        let (prefix, tail) = rest.split_at(i);
        out.push_str(prefix);
        let end = tail
            .find(['/', '\\', ' ', '\'', '"', ':'])
            .unwrap_or(tail.len());
        out.push_str("<ROOT>");
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn binary_runner_replays_recorded_output_comparison() {
        let r = BinaryResult {
            code: 0,
            stdout: b"ok\n".to_vec(),
            stderr: vec![],
        };
        assert!(compare_recorded(&json!({"code":0,"stdout":"ok\n","stderr":""}), &r).is_ok());
    }
    #[test]
    fn binary_runner_replays_stdin_and_isolated_home() {
        let case = json!({"argv":["-c","printf '%s:%s' \"$HOME\" \"$(cat)\""],"env":{"TASK_ENV":"ok"},"stdin":"input"});
        let result = run_binary_case(Path::new("/bin/sh"), &case).unwrap();
        let out = String::from_utf8(result.stdout).unwrap();
        assert!(out.contains("<ROOT>/home"));
        assert!(out.ends_with(":input"));
    }
    #[test]
    fn binary_runner_replays_blessed_expectations() {
        let mut cases = vec![
            json!({"test":"binary::echo","argv":["-c","printf ok"],"code":0,"stdout":"ok","stderr":""}),
        ];
        let report =
            run_binary_cases(&mut cases, "/bin/sh", Some("binary::"), false, true).unwrap();
        assert_eq!(report.run, 1);
        assert_eq!(cases[0]["source"], "bless");
        assert_eq!(cases[0]["stdout"], "ok");
    }
    #[test]
    fn binary_runner_replays_record_mode() {
        let mut cases = vec![json!({"test":"binary::echo","argv":["-c","printf ok"]})];
        let report =
            run_binary_cases(&mut cases, "/bin/sh", Some("binary::"), true, false).unwrap();
        assert_eq!(report.run, 1);
        assert_eq!(cases[0]["code"], 0);
        assert_eq!(cases[0]["stdout"], "ok");
    }
}
