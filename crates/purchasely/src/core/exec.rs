//! Subprocess execution without a shell.
use std::{
    path::Path,
    process::{Command, Stdio},
};
#[derive(Debug)]
pub struct ExecOutput {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}
pub fn run(
    file: &str,
    args: &[String],
    cwd: Option<&Path>,
    input: Option<&[u8]>,
) -> std::io::Result<ExecOutput> {
    let mut c = Command::new(file);
    c.args(args);
    if let Some(dir) = cwd {
        c.current_dir(dir);
    }
    c.stdin(if input.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    let mut child = c.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    if let Some(bytes) = input {
        use std::io::Write;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(bytes)?
        }
    }
    let o = child.wait_with_output()?;
    Ok(ExecOutput {
        code: o.status.code().unwrap_or(1),
        stdout: o.stdout,
        stderr: o.stderr,
    })
}
