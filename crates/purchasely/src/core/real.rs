//! Owner: P5a
use super::{deps::*, exec};
use std::{
    collections::BTreeMap,
    io::{self, Read, Write},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub struct RealHost {
    env: BTreeMap<String, String>,
    warned_tls: AtomicBool,
    /// One agent per process: requests share its connection pool.
    agent: std::sync::OnceLock<ureq::Agent>,
    /// TS `main.ts` `keychain: defaultKeychain()`: the real keychain belongs to the real host only.
    keychain: Option<super::credentials::SecurityKeychain>,
}

impl Default for RealHost {
    fn default() -> Self {
        Self::new(process_env())
    }
}

/// The environment as Node `process.env` gives it: an invalid UTF-8 byte becomes U+FFFD. `std::env::vars`
/// panics on such an entry, with its value in the message.
pub fn process_env() -> BTreeMap<String, String> {
    std::env::vars_os()
        .map(|(k, v)| {
            (
                k.to_string_lossy().into_owned(),
                v.to_string_lossy().into_owned(),
            )
        })
        .collect()
}

impl RealHost {
    pub fn new(env: BTreeMap<String, String>) -> Self {
        Self {
            env,
            warned_tls: AtomicBool::new(false),
            agent: std::sync::OnceLock::new(),
            keychain: super::credentials::default_keychain(if cfg!(target_os = "macos") {
                "darwin"
            } else {
                "linux"
            }),
        }
    }

    fn agent(&self) -> &ureq::Agent {
        self.agent.get_or_init(|| {
            let tls = super::tls::config(&self.env, |s| {
                if !self.warned_tls.swap(true, Ordering::Relaxed) {
                    let _ = writeln!(io::stderr(), "{s}");
                }
            });
            ureq::Agent::config_builder()
                // Node `fetch` read no proxy variable: ureq would read ALL_PROXY, HTTPS_PROXY, HTTP_PROXY.
                .proxy(None)
                .http_status_as_error(false)
                .tls_config(tls)
                .build()
                .new_agent()
        })
    }

    fn write_stdout(&self, value: &str) {
        if let Err(error) = io::stdout().write_all(value.as_bytes())
            && stdout_error_is_fatal(&error)
        {
            let _ = writeln!(io::stderr(), "{error}");
            std::process::exit(1);
        }
    }
}

/// TS `main()`: EPIPE on stdout (`reference | head`) is ignored; any other stdout error is fatal.
fn stdout_error_is_fatal(error: &io::Error) -> bool {
    error.kind() != io::ErrorKind::BrokenPipe
}

impl Host for RealHost {
    fn fetch(&self, req: Request) -> Result<Response, NetError> {
        let agent = self.agent();
        let mut request = ureq::http::Request::builder()
            .method(req.method.as_str())
            .uri(req.url.as_str());
        for (name, value) in req.headers {
            request = request.header(name, value);
        }
        let request = request
            .body(req.body.unwrap_or_default())
            .map_err(|e| NetError {
                message: e.to_string(),
                ..NetError::default()
            })?;
        let request = agent
            .configure_request(request)
            .max_redirects(if req.refuse_redirect { 0 } else { 20 })
            .timeout_global(Some(Duration::from_millis(req.timeout_ms)))
            .build();
        let response = agent.run(request).map_err(net_error)?;
        let (parts, body) = response.into_parts();
        let headers = parts
            .headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let mut bytes = Vec::new();
        body.into_reader()
            .read_to_end(&mut bytes)
            .map_err(|e| NetError {
                timed_out: e.kind() == io::ErrorKind::TimedOut,
                code: io_code(&e),
                message: e.to_string(),
            })?;
        Ok(Response {
            status: parts.status.as_u16(),
            headers,
            body: bytes,
        })
    }

    fn exec(&self, file: &str, args: &[String], opts: ExecOpts) -> Result<ExecResult, ExecError> {
        exec::run(file, args, opts)
    }

    fn prompt(&self, question: &str, hidden: bool) -> String {
        let _ = io::stderr().write_all(question.as_bytes());
        let _ = io::stderr().flush();
        let fd = libc::STDIN_FILENO;
        let mut old = std::mem::MaybeUninit::<libc::termios>::uninit();
        let changed = hidden && unsafe { libc::tcgetattr(fd, old.as_mut_ptr()) == 0 };
        if changed {
            let mut new = unsafe { old.assume_init() };
            new.c_lflag &= !libc::ECHO;
            unsafe {
                libc::tcsetattr(fd, libc::TCSAFLUSH, &new);
            }
        }
        let mut answer = String::new();
        let _ = io::stdin().read_line(&mut answer);
        if changed {
            unsafe {
                libc::tcsetattr(fd, libc::TCSAFLUSH, old.as_ptr());
            }
            let _ = writeln!(io::stderr());
        }
        answer.trim_end_matches(['\r', '\n']).to_owned()
    }

    fn read_stdin(&self) -> String {
        let mut value = String::new();
        let _ = io::stdin().read_to_string(&mut value);
        value
    }
    fn now(&self) -> i64 {
        self.wall_now()
    }
    fn wall_now(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .min(i64::MAX as u128) as i64
    }
    fn pid_alive(&self, pid: u32) -> bool {
        pid != 0
            && unsafe {
                libc::kill(pid as i32, 0) == 0
                    || io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
            }
    }
    fn sleep(&self, ms: i64) {
        if ms > 0 {
            std::thread::sleep(Duration::from_millis(ms as u64));
        }
    }
    fn stdout(&self, s: &str) {
        self.write_stdout(s);
    }
    fn stderr(&self, s: &str) {
        let _ = io::stderr().write_all(s.as_bytes());
    }
    fn keychain(&self) -> Option<&dyn Keychain> {
        self.keychain.as_ref().map(|k| k as &dyn Keychain)
    }
    fn io(&self) -> Option<&dyn Io> {
        None
    }
}

fn net_error(error: ureq::Error) -> NetError {
    NetError {
        timed_out: matches!(error, ureq::Error::Timeout(_)),
        code: match &error {
            ureq::Error::HostNotFound => Some("ENOTFOUND".into()),
            ureq::Error::Io(e) => io_code(e),
            _ => None,
        },
        message: error.to_string(),
    }
}

/// The Node `cause.code` of a socket error.
fn io_code(e: &io::Error) -> Option<String> {
    let code = match e.kind() {
        io::ErrorKind::ConnectionRefused => "ECONNREFUSED",
        io::ErrorKind::ConnectionReset => "ECONNRESET",
        _ => return None,
    };
    Some(code.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{net::TcpListener, thread};

    /// main.test.ts "ignores EPIPE on stdout": EPIPE is ignored, EIO is still fatal. The binary test
    /// `reference_closed_stdout_exits_quietly` runs the EPIPE case end to end.
    #[test]
    fn epipe_on_stdout_is_ignored_and_another_error_is_fatal() {
        assert!(!stdout_error_is_fatal(&io::Error::from_raw_os_error(
            libc::EPIPE
        )));
        assert!(stdout_error_is_fatal(&io::Error::from_raw_os_error(
            libc::EIO
        )));
    }
    /// update.test.ts "detach > a blocked child does not delay the parent's exit".
    #[test]
    fn a_detached_child_that_blocks_does_not_delay_the_caller() {
        let start = std::time::Instant::now();
        let result = RealHost::default().exec(
            "sh",
            &["-c".into(), "sleep 20".into()],
            ExecOpts {
                timeout_ms: 0,
                detach: true,
                ..ExecOpts::default()
            },
        );
        assert_eq!(
            result,
            Ok(ExecResult {
                code: 0,
                stdout: String::new(),
                stderr: String::new(),
                timed_out: false,
            })
        );
        assert!(start.elapsed() < std::time::Duration::from_secs(5));
    }
    #[test]
    fn net_error_carries_the_node_cause_code() {
        assert_eq!(
            net_error(ureq::Error::HostNotFound).code.as_deref(),
            Some("ENOTFOUND")
        );
        let refused = io::Error::from(io::ErrorKind::ConnectionRefused);
        assert_eq!(
            net_error(ureq::Error::Io(refused)).code.as_deref(),
            Some("ECONNREFUSED")
        );
        assert_eq!(net_error(ureq::Error::ConnectionFailed).code, None);
    }

    fn server(response: Vec<u8>, delay: Duration) -> String {
        server_requests(response, delay, 1)
    }

    fn server_requests(response: Vec<u8>, delay: Duration, requests: usize) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            for _ in 0..requests {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0; 4096];
                let _ = stream.read(&mut request);
                thread::sleep(delay);
                let _ = stream.write_all(&response);
            }
        });
        format!("http://{addr}")
    }

    fn request(url: String) -> Request {
        Request {
            method: "GET".into(),
            url,
            ..Request::default()
        }
    }

    /// Node `fetch` ignored proxy variables: a proxy in the environment must not reroute or break requests.
    #[test]
    fn proxy_variables_change_nothing() {
        let names = [
            "ALL_PROXY",
            "HTTPS_PROXY",
            "HTTP_PROXY",
            "all_proxy",
            "https_proxy",
            "http_proxy",
        ];
        // SAFETY: a test-only write; the other tests of this crate do not read these variables.
        unsafe {
            for name in names {
                std::env::set_var(name, "http://127.0.0.1:9");
            }
        }
        let url = server(
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".to_vec(),
            Duration::ZERO,
        );
        let result = RealHost::default().fetch(request(url));
        unsafe {
            for name in names {
                std::env::remove_var(name);
            }
        }
        assert_eq!(result.unwrap().body, b"ok");
    }
    /// One agent per process: a second request reuses the agent (and its connection pool).
    #[test]
    fn one_agent_per_host() {
        let host = RealHost::default();
        let first: *const ureq::Agent = host.agent();
        assert!(std::ptr::eq(first, host.agent()));
        assert!(host.agent().config().proxy().is_none());
    }
    #[test]
    fn status_and_headers_are_preserved() {
        let url = server(b"HTTP/1.1 201 Created\r\nX-Trace: abc\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".to_vec(), Duration::ZERO);
        let result = RealHost::default().fetch(request(url)).unwrap();
        assert_eq!(result.status, 201);
        assert!(
            result
                .headers
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("x-trace") && v == "abc")
        );
        assert_eq!(result.body, b"ok");
    }

    #[test]
    fn gzip_body_is_decoded() {
        let payload = b"hello";
        let body = [
            0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x03, 0xcb, 0x48, 0xcd, 0xc9,
            0xc9, 0x07, 0x00, 0x86, 0xa6, 0x10, 0x36, 0x05, 0x00, 0x00, 0x00,
        ];
        let mut response = format!("HTTP/1.1 200 OK\r\nContent-Encoding: gzip\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).into_bytes();
        response.extend(body);
        let result = RealHost::default()
            .fetch(request(server(response, Duration::ZERO)))
            .unwrap();
        assert_eq!(result.body, payload);
    }

    #[test]
    fn redirects_are_followed_unless_refused() {
        let target = server(
            b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\ndone".to_vec(),
            Duration::ZERO,
        );
        let mut response = format!("HTTP/1.1 302 Found\r\nLocation: {target}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").into_bytes();
        let source = server_requests(std::mem::take(&mut response), Duration::ZERO, 2);
        assert_eq!(
            RealHost::default()
                .fetch(request(source.clone()))
                .unwrap()
                .body,
            b"done"
        );
        let result = RealHost::default()
            .fetch(Request {
                refuse_redirect: true,
                ..request(source)
            })
            .unwrap();
        assert_eq!(result.status, 302);
    }

    #[test]
    fn timeout_covers_body_read() {
        let url = server(
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".to_vec(),
            Duration::from_millis(150),
        );
        let err = RealHost::default()
            .fetch(Request {
                timeout_ms: 20,
                ..request(url)
            })
            .unwrap_err();
        assert!(err.timed_out, "{err:?}");
    }

    #[test]
    fn exec_maps_missing_program_and_spawn_error() {
        assert_eq!(
            RealHost::default().exec("/missing/purchasely-test", &[], ExecOpts::default()),
            Err(ExecError::NotFound)
        );
        #[cfg(unix)]
        assert!(matches!(
            RealHost::default().exec("/", &[], ExecOpts::default()),
            Err(ExecError::Other(_))
        ));
    }

    #[test]
    fn exec_kills_child_after_termination_grace() {
        #[cfg(unix)]
        {
            let result = RealHost::default()
                .exec(
                    "sh",
                    &["-c".into(), "trap '' TERM; while :; do :; done".into()],
                    ExecOpts {
                        timeout_ms: 20,
                        ..ExecOpts::default()
                    },
                )
                .unwrap();
            assert!(result.timed_out);
        }
    }

    #[test]
    fn exec_rejects_output_above_limit() {
        #[cfg(unix)]
        assert!(matches!(
            RealHost::default().exec(
                "sh",
                &["-c".into(), "head -c 268435457 /dev/zero".into()],
                ExecOpts::default()
            ),
            Err(ExecError::Other(_))
        ));
    }
}
