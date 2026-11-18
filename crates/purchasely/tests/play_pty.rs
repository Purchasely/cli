//! The play loop of `test/game-term.test.ts` on a real pseudo-terminal: each way out restores the terminal
//! (raw mode off, alternate screen left) and gives the TS exit code.

use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const ENTER: &str = "\x1b[?1049h";
const LEAVE: &str = "\x1b[?1049l";

struct Pty {
    master: std::fs::File,
    slave: OwnedFd,
    out: Arc<Mutex<Vec<u8>>>,
}

/// A pseudo-terminal of 80x24 in its default (cooked) mode, and a thread that keeps its output.
fn pty() -> Pty {
    unsafe {
        let master = libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY);
        assert!(master >= 0);
        assert_eq!(libc::grantpt(master), 0);
        assert_eq!(libc::unlockpt(master), 0);
        let name = std::ffi::CStr::from_ptr(libc::ptsname(master)).to_owned();
        let slave = libc::open(name.as_ptr(), libc::O_RDWR | libc::O_NOCTTY);
        assert!(slave >= 0);
        let size = libc::winsize {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        assert_eq!(libc::ioctl(slave, libc::TIOCSWINSZ, &size), 0);
        let master = std::fs::File::from_raw_fd(master);
        let out = Arc::new(Mutex::new(Vec::new()));
        let (mut reader, sink) = (master.try_clone().unwrap(), out.clone());
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                sink.lock().unwrap().extend_from_slice(&buf[..n]);
            }
        });
        Pty {
            master,
            slave: OwnedFd::from_raw_fd(slave),
            out,
        }
    }
}

fn termios(fd: &OwnedFd) -> libc::termios {
    let mut t = std::mem::MaybeUninit::<libc::termios>::uninit();
    assert_eq!(
        unsafe { libc::tcgetattr(fd.as_raw_fd(), t.as_mut_ptr()) },
        0
    );
    unsafe { t.assume_init() }
}

fn text(p: &Pty) -> String {
    String::from_utf8_lossy(&p.out.lock().unwrap()).into_owned()
}

fn wait_for(p: &Pty, what: &str) {
    let start = Instant::now();
    while !text(p).contains(what) {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "no {what:?} in {:?}",
            text(p)
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// `purchasely games snake` on the slave side, once the game is drawn.
fn start(p: &Pty) -> Child {
    let home = std::env::temp_dir().join(format!("play-pty-{}", std::process::id()));
    let io = || Stdio::from(p.slave.try_clone().unwrap());
    let child = Command::new(env!("CARGO_BIN_EXE_purchasely"))
        .args(["games", "snake"])
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_STATE_HOME", home.join("state"))
        .env("XDG_CACHE_HOME", home.join("cache"))
        .env("PURCHASELY_DISABLE_AUTOUPDATE", "1")
        .stdin(io())
        .stdout(io())
        .stderr(io())
        .spawn()
        .unwrap();
    wait_for(p, ENTER);
    child
}

fn finish(mut child: Child) -> i32 {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status.code().unwrap_or(-1);
        }
        if start.elapsed() > Duration::from_secs(10) {
            let _ = child.kill();
            panic!("the game did not end");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// The TS `restored(r)`: raw mode is off again and the alternate screen is left, as the last thing drawn.
fn assert_restored(p: &Pty, before: &libc::termios, what: &str) {
    let after = termios(&p.slave);
    assert_eq!(after.c_lflag, before.c_lflag, "{what}: local flags");
    assert_eq!(after.c_iflag, before.c_iflag, "{what}: input flags");
    // Let the reader thread drain the last bytes.
    std::thread::sleep(Duration::from_millis(100));
    let out = text(p);
    let leave = out
        .rfind(LEAVE)
        .unwrap_or_else(|| panic!("{what}: no {LEAVE:?}"));
    assert!(leave > out.rfind(ENTER).unwrap(), "{what}");
}

fn key_ends_play(key: &[u8], what: &str) {
    let mut p = pty();
    let before = termios(&p.slave);
    let child = start(&p);
    p.master.write_all(key).unwrap();
    assert_eq!(finish(child), 0, "{what}");
    assert_restored(&p, &before, what);
}

/// game-term.test.ts "play loop > restores the terminal on q".
#[test]
fn restores_the_terminal_on_q() {
    key_ends_play(b"q", "q");
}

/// game-term.test.ts "play loop > restores the terminal on Ctrl-C".
#[test]
fn restores_the_terminal_on_ctrl_c() {
    key_ends_play(b"\x03", "Ctrl-C");
}

/// game-term.test.ts "play loop > restores the terminal and exits 128 + the signal number on SIGTERM,
/// SIGHUP and SIGINT".
#[test]
fn restores_the_terminal_and_exits_128_plus_the_signal_number() {
    for (signal, code) in [
        (libc::SIGTERM, 143),
        (libc::SIGHUP, 129),
        (libc::SIGINT, 130),
    ] {
        let p = pty();
        let before = termios(&p.slave);
        let child = start(&p);
        assert_eq!(unsafe { libc::kill(child.id() as i32, signal) }, 0);
        assert_eq!(finish(child), code, "signal {signal}");
        assert_restored(&p, &before, &format!("signal {signal}"));
    }
}

/// game-term.test.ts "play loop > ends with a message when the terminal is resized".
#[test]
fn ends_with_a_message_when_the_terminal_is_resized() {
    let p = pty();
    let before = termios(&p.slave);
    let child = start(&p);
    let size = libc::winsize {
        ws_row: 30,
        ws_col: 100,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    assert_eq!(
        unsafe { libc::ioctl(p.master.as_raw_fd(), libc::TIOCSWINSZ, &size) },
        0
    );
    assert_eq!(finish(child), 0);
    assert_restored(&p, &before, "resize");
    wait_for(&p, "The terminal changed size.");
}
