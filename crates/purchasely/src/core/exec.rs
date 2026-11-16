//! Bounded subprocess execution without a shell.
use super::deps::{ExecError, ExecOpts, ExecResult};
use std::{
    io::Write,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const MAX_OUTPUT: usize = 256 * 1024 * 1024;

/// `Command::spawn` that tries again on ETXTBSY: a script that was just written can still be
/// open for write in a child that another thread forked at the same time.
pub fn spawn(c: &mut Command) -> std::io::Result<std::process::Child> {
    let mut tries = 0;
    loop {
        match c.spawn() {
            Err(e) if e.raw_os_error() == Some(libc::ETXTBSY) && tries < 20 => {
                tries += 1;
                std::thread::sleep(Duration::from_millis(10));
            }
            other => return other,
        }
    }
}
pub fn run(file: &str, args: &[String], mut opts: ExecOpts) -> Result<ExecResult, ExecError> {
    let mut c = Command::new(file);
    c.args(args);
    if let Some(cwd) = opts.cwd {
        c.current_dir(cwd);
    }
    if opts.detach {
        c.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            unsafe {
                c.pre_exec(|| {
                    if libc::setsid() == -1 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }
        return spawn(&mut c)
            .map(|_| ExecResult {
                code: 0,
                ..ExecResult::default()
            })
            .map_err(exec_error);
    }
    c.stdin(if opts.input.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    })
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
    let mut child = spawn(&mut c).map_err(exec_error)?;
    let input = opts.input.take();
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| ExecError::Other("missing child stdout".into()))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| ExecError::Other("missing child stderr".into()))?;
    let out_thread = std::thread::spawn(move || read_bounded(&mut stdout));
    let err_thread = std::thread::spawn(move || read_bounded(&mut stderr));
    if let Some(input) = input
        && let Some(mut stdin) = child.stdin.take()
    {
        let _ = stdin.write_all(input.as_bytes());
    }
    let start = Instant::now();
    let mut timed = false;
    let mut status = None;
    loop {
        if let Some(s) = child
            .try_wait()
            .map_err(|e| ExecError::Other(e.to_string()))?
        {
            status = Some(s);
            break;
        }
        if opts.timeout_ms > 0 && start.elapsed() >= Duration::from_millis(opts.timeout_ms) {
            timed = true;
            #[cfg(unix)]
            unsafe {
                libc::kill(child.id() as i32, libc::SIGTERM);
            }
            let grace = Instant::now();
            while grace.elapsed() < Duration::from_secs(2) {
                if let Some(s) = child
                    .try_wait()
                    .map_err(|e| ExecError::Other(e.to_string()))?
                {
                    status = Some(s);
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            if status.is_none() {
                let _ = child.kill();
                status = Some(child.wait().map_err(|e| ExecError::Other(e.to_string()))?);
            }
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let (out_res, out) = out_thread
        .join()
        .map_err(|_| ExecError::Other("stdout reader panicked".into()))?;
    let (err_res, err) = err_thread
        .join()
        .map_err(|_| ExecError::Other("stderr reader panicked".into()))?;
    out_res.map_err(|e| ExecError::Other(e.to_string()))?;
    err_res.map_err(|e| ExecError::Other(e.to_string()))?;
    if out.len() > MAX_OUTPUT || err.len() > MAX_OUTPUT {
        return Err(ExecError::Other(
            "subprocess output exceeded 256 MiB".into(),
        ));
    }
    Ok(ExecResult {
        code: status.and_then(|s| s.code()).unwrap_or(1),
        stdout: String::from_utf8_lossy(&out).into(),
        stderr: String::from_utf8_lossy(&err).into(),
        timed_out: timed,
    })
}
fn read_bounded(r: &mut impl std::io::Read) -> (std::io::Result<()>, Vec<u8>) {
    let mut kept = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        match r.read(&mut buf) {
            Ok(0) => return (Ok(()), kept),
            Ok(n) => {
                let take = (MAX_OUTPUT + 1).saturating_sub(kept.len()).min(n);
                kept.extend_from_slice(&buf[..take]);
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return (Err(e), kept),
        }
    }
}
fn exec_error(e: std::io::Error) -> ExecError {
    if e.kind() == std::io::ErrorKind::NotFound {
        ExecError::NotFound
    } else {
        ExecError::Other(e.to_string())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn returns_output_and_status() {
        let r = run(
            "sh",
            &["-c".into(), "printf hello; exit 7".into()],
            ExecOpts::default(),
        )
        .unwrap();
        assert_eq!(r.stdout, "hello");
        assert_eq!(r.code, 7)
    }
    #[test]
    fn missing_program_maps_to_not_found() {
        assert_eq!(
            run("/path/that/does/not/exist", &[], ExecOpts::default()).unwrap_err(),
            ExecError::NotFound
        )
    }
    #[test]
    fn timeout_terminates_child() {
        let r = run(
            "sh",
            &["-c".into(), "trap '' TERM; while :; do :; done".into()],
            ExecOpts {
                timeout_ms: 20,
                ..ExecOpts::default()
            },
        )
        .unwrap();
        assert!(r.timed_out)
    }
}
