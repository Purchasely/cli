//! Owner: P1b
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

#[derive(Debug)]
pub struct BinaryResult {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}
pub fn run_binary(
    binary: &Path,
    argv: &[String],
    env: &[(String, String)],
    stdin: &[u8],
) -> std::io::Result<BinaryResult> {
    let mut cmd = Command::new(binary);
    cmd.args(argv).env("PURCHASELY_DISABLE_AUTOUPDATE", "1");
    for (k, v) in env {
        cmd.env(k, v);
    }
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
pub fn record_path(directory: &Path, name: &str, result: &BinaryResult) -> std::io::Result<()> {
    fs::create_dir_all(directory)?;
    let val = json!({"test":name,"code":result.code,"stdout":String::from_utf8_lossy(&result.stdout),"stderr":String::from_utf8_lossy(&result.stderr)});
    fs::write(
        directory.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&val).map_err(std::io::Error::other)?,
    )
}
pub fn compare_recorded(expected: &Value, actual: &BinaryResult) -> Result<(), String> {
    let got = json!({"code":actual.code,"stdout":String::from_utf8_lossy(&actual.stdout),"stderr":String::from_utf8_lossy(&actual.stderr)});
    for k in ["code", "stdout", "stderr"] {
        if expected.get(k) != got.get(k) {
            return Err(format!(
                "binary {k} mismatch: expected {}, got {}",
                expected[k], got[k]
            ));
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compares_recorded_output() {
        let r = BinaryResult {
            code: 0,
            stdout: b"ok\n".to_vec(),
            stderr: vec![],
        };
        assert!(compare_recorded(&json!({"code":0,"stdout":"ok\n","stderr":""}), &r).is_ok());
    }
}
