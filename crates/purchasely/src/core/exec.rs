//! Bounded subprocess execution without a shell.
use super::deps::{ExecError, ExecOpts, ExecResult};
use std::{
    io::Write,
    process::{Command, Stdio},
    sync::mpsc::{Receiver, RecvTimeoutError, channel},
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
    // TS `execFile`: the timeout starts at spawn, and the input is written while the parent waits.
    let start = Instant::now();
    let input = opts.input.take();
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ExecError::Other("missing child stdout".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| ExecError::Other("missing child stderr".into()))?;
    let out_rx = reader(stdout);
    let err_rx = reader(stderr);
    // The child may exit or be killed before it reads the input: the write error is ignored, as in TS.
    // The writer is never joined: a descendant that inherited stdin must not hold the call.
    if let (Some(input), Some(mut stdin)) = (input, child.stdin.take()) {
        std::thread::spawn(move || {
            let _ = stdin.write_all(input.as_bytes());
        });
    }
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
    // TS `execFile` destroys the pipes on timeout: a descendant that inherited stdout or stderr
    // must not keep the call waiting past the deadline, or past a short grace once it was killed.
    let deadline = if timed {
        Some(Instant::now() + Duration::from_millis(100))
    } else if opts.timeout_ms > 0 {
        Some(start + Duration::from_millis(opts.timeout_ms))
    } else {
        None
    };
    let (out, out_done) = collect(&out_rx, deadline);
    let (err, err_done) = collect(&err_rx, deadline);
    // TS: the timer that fires after a child exited 0 does not fail the call; any other exit is "killed".
    timed |= !(out_done && err_done) && status.and_then(|s| s.code()) != Some(0);
    let out = out.map_err(|e| ExecError::Other(e.to_string()))?;
    let err = err.map_err(|e| ExecError::Other(e.to_string()))?;
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
/// Reads a pipe on its own thread and sends what it keeps (at most `MAX_OUTPUT + 1` bytes).
fn reader(mut r: impl std::io::Read + Send + 'static) -> Receiver<std::io::Result<Vec<u8>>> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let mut kept = 0usize;
        let mut buf = [0u8; 8192];
        loop {
            match r.read(&mut buf) {
                Ok(0) => return,
                Ok(n) => {
                    let take = (MAX_OUTPUT + 1).saturating_sub(kept).min(n);
                    kept += take;
                    if take > 0 && tx.send(Ok(buf[..take].to_vec())).is_err() {
                        return;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            }
        }
    });
    rx
}
/// Drains a reader until EOF or the deadline. The flag is false when the deadline stopped it.
fn collect(
    rx: &Receiver<std::io::Result<Vec<u8>>>,
    deadline: Option<Instant>,
) -> (std::io::Result<Vec<u8>>, bool) {
    let mut kept = Vec::new();
    loop {
        let next = match deadline {
            None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
            Some(d) => rx.recv_timeout(d.saturating_duration_since(Instant::now())),
        };
        match next {
            Ok(Ok(chunk)) => kept.extend_from_slice(&chunk),
            Ok(Err(e)) => return (Err(e), true),
            Err(RecvTimeoutError::Disconnected) => return (Ok(kept), true),
            Err(RecvTimeoutError::Timeout) => return (Ok(kept), false),
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
    /// TS `execFile` starts the timeout at spawn and writes `input` while it waits: a child that never
    /// reads its stdin still times out.
    #[test]
    fn timeout_covers_the_write_of_stdin() {
        let start = Instant::now();
        let r = run(
            "sh",
            &["-c".into(), "exec sleep 5".into()],
            ExecOpts {
                input: Some("x".repeat(1 << 20)),
                timeout_ms: 50,
                ..ExecOpts::default()
            },
        )
        .unwrap();
        assert!(r.timed_out);
        assert!(
            start.elapsed() < Duration::from_secs(3),
            "{:?}",
            start.elapsed()
        );
    }
    /// A descendant that inherited stdin, stdout and stderr outlives the killed wrapper: the call
    /// still returns at the deadline.
    #[test]
    fn timeout_does_not_wait_for_a_descendant_holding_the_pipes() {
        let start = Instant::now();
        let r = run(
            "sh",
            &["-c".into(), "sleep 5 & sleep 5".into()],
            ExecOpts {
                input: Some("x".repeat(1 << 20)),
                timeout_ms: 50,
                ..ExecOpts::default()
            },
        )
        .unwrap();
        assert!(r.timed_out);
        assert!(
            start.elapsed() < Duration::from_millis(500),
            "{:?}",
            start.elapsed()
        );
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
