//! Owner: P11a
use std::io::{self, Write};

const CSI: &str = "\x1b[";

pub const BRAND: Brand = Brand {
    dark_green: "#002420",
    light_green: "#ceecd0",
    lavender: "#bdbbff",
    coral: "#ffb6b2",
    cream: "#fff3d6",
    sage: "#90b09b",
    white: "#ffffff",
};

pub struct Brand {
    pub dark_green: &'static str,
    pub light_green: &'static str,
    pub lavender: &'static str,
    pub coral: &'static str,
    pub cream: &'static str,
    pub sage: &'static str,
    pub white: &'static str,
}

fn rgb(hex: &str) -> String {
    let bytes = hex.as_bytes();
    if bytes.len() != 7 || bytes[0] != b'#' {
        return "0;0;0".into();
    }
    (1..7)
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .unwrap_or(0)
                .to_string()
        })
        .collect::<Vec<_>>()
        .join(";")
}

pub fn fg(hex: &str, text: &str) -> String {
    format!("{CSI}38;2;{}m{text}{CSI}39m", rgb(hex))
}

pub fn bg(hex: &str, text: &str) -> String {
    format!("{CSI}48;2;{}m{text}{CSI}49m", rgb(hex))
}

const FONT: [(&str, [&str; 5]); 10] = [
    ("P", ["███ ", "█  █", "███ ", "█   ", "█   "]),
    ("U", ["█  █", "█  █", "█  █", "█  █", " ██ "]),
    ("R", ["███ ", "█  █", "███ ", "█ █ ", "█  █"]),
    ("C", [" ███", "█   ", "█   ", "█   ", " ███"]),
    ("H", ["█  █", "█  █", "████", "█  █", "█  █"]),
    ("A", [" ██ ", "█  █", "████", "█  █", "█  █"]),
    ("S", [" ███", "█   ", " ██ ", "   █", "███ "]),
    ("E", ["████", "█   ", "███ ", "█   ", "████"]),
    ("L", ["█   ", "█   ", "█   ", "█   ", "████"]),
    ("Y", ["█  █", "█  █", " ██ ", " █  ", " █  "]),
];

pub fn banner() -> Vec<String> {
    let colors = [
        BRAND.light_green,
        BRAND.light_green,
        BRAND.light_green,
        BRAND.lavender,
        BRAND.lavender,
        BRAND.lavender,
        BRAND.lavender,
        BRAND.coral,
        BRAND.coral,
        BRAND.coral,
    ];
    (0..5)
        .map(|row| {
            FONT.iter()
                .zip(colors)
                .map(|((_, glyph), color)| fg(color, glyph[row]))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

pub trait Game {
    fn tick(&self) -> u64;
    fn key(&mut self, name: &str);
    fn step(&mut self);
    fn draw(&self) -> Vec<String>;
    fn over(&self) -> Option<&str>;
    fn typing(&self) -> bool {
        false
    }
    fn resize(&mut self, _cols: usize, _rows: usize) {}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    Text(char),
    Named(&'static str),
    Quit,
}

#[derive(Default)]
struct PlayState {
    result: Option<String>,
}

impl PlayState {
    fn key(&mut self, game: &mut dyn Game, key: Key) {
        if let Some(message) = game.over() {
            self.result = Some(message.to_owned());
            return;
        }
        match key {
            Key::Quit => self.result = Some("Bye.".into()),
            Key::Text('q') if !game.typing() => self.result = Some("Bye.".into()),
            Key::Text(c) => game.key(&c.to_string()),
            Key::Named(name) => game.key(name),
        }
    }

    fn resized(&mut self) {
        self.result = Some("The terminal changed size.".into());
    }
}

pub fn decode_key(bytes: &[u8]) -> Option<Key> {
    match bytes {
        b"\x1b[A" | b"\x1bOA" => Some(Key::Named("up")),
        b"\x1b[B" | b"\x1bOB" => Some(Key::Named("down")),
        b"\x1b[C" | b"\x1bOC" => Some(Key::Named("right")),
        b"\x1b[D" | b"\x1bOD" => Some(Key::Named("left")),
        b"\x1b" | b"\x03" => Some(Key::Quit),
        b"\x7f" | b"\x08" => Some(Key::Named("backspace")),
        b"\r" | b"\n" => Some(Key::Named("return")),
        b" " => Some(Key::Named("space")),
        [c] if *c >= 0x20 && *c < 0x7f => Some(Key::Text(*c as char)),
        _ => None,
    }
}

pub struct Menu {
    names: Vec<String>,
    title: String,
    verb: String,
    query: String,
    shown: Vec<usize>,
    current: usize,
    chosen: Option<usize>,
    cols: usize,
    rows: usize,
}

impl Menu {
    pub fn new(names: Vec<String>, title: impl Into<String>, verb: impl Into<String>) -> Self {
        let shown = (0..names.len()).collect();
        Self {
            names,
            title: title.into(),
            verb: verb.into(),
            query: String::new(),
            shown,
            current: 0,
            chosen: None,
            cols: 80,
            rows: 24,
        }
    }

    pub fn key(&mut self, key: &str) {
        match key {
            "up" => {
                let n = self.shown.len();
                self.current = if n == 0 {
                    0
                } else {
                    (self.current + n - 1) % n
                };
            }
            "down" => {
                let n = self.shown.len();
                self.current = if n == 0 { 0 } else { (self.current + 1) % n };
            }
            "return" | "enter" => self.chosen = self.shown.get(self.current).copied(),
            "backspace" => {
                self.query.pop();
                self.search();
            }
            text if text.chars().count() == 1 => {
                self.query.push_str(text);
                self.search();
            }
            _ => {}
        }
    }

    fn search(&mut self) {
        let query = self.query.to_lowercase();
        self.shown = self
            .names
            .iter()
            .enumerate()
            .filter_map(|(i, name)| name.to_lowercase().contains(&query).then_some(i))
            .collect();
        self.current = 0;
    }

    pub fn chosen(&self) -> Option<usize> {
        self.chosen
    }

    pub fn set_dimensions(&mut self, cols: usize, rows: usize) {
        self.cols = cols;
        self.rows = rows;
    }

    pub fn draw(&self, cols: usize, rows: usize) -> Vec<String> {
        let mut head = vec![String::new()];
        if cols >= 51 && rows >= 16 {
            head.extend(banner().into_iter().map(|line| format!(" {line}")));
        } else {
            head.push(fg(BRAND.light_green, " PURCHASELY"));
        }
        head.push(fg(BRAND.sage, &format!(" {}", self.title)));
        head.push(format!(" {} {}_", fg(BRAND.sage, "Search:"), self.query));
        head.push(String::new());
        let room = 3.max(rows.saturating_sub(head.len() + 2));
        let top = self
            .current
            .saturating_sub(room >> 1)
            .min(self.shown.len().saturating_sub(room));
        let mut lines = head;
        if self.shown.is_empty() {
            lines.push("    No match.".into());
        } else {
            lines.extend(self.shown.iter().enumerate().skip(top).take(room).map(
                |(index_position, index)| {
                    if index_position == self.current {
                        format!(
                            " {}",
                            bg(
                                BRAND.lavender,
                                &fg(BRAND.dark_green, &format!(" P {} ", self.names[*index]))
                            )
                        )
                    } else {
                        format!("    {}", self.names[*index])
                    }
                },
            ));
        }
        lines.push(String::new());
        lines.push(fg(
            BRAND.sage,
            &format!(
                " ↑↓ choose   Enter {}   type to search   Esc quit",
                self.verb
            ),
        ));
        lines
    }
}

impl Game for Menu {
    fn tick(&self) -> u64 {
        0
    }
    fn key(&mut self, name: &str) {
        Menu::key(self, name);
    }
    fn step(&mut self) {}
    fn draw(&self) -> Vec<String> {
        Menu::draw(self, self.cols, self.rows)
    }
    fn over(&self) -> Option<&str> {
        self.chosen
            .and_then(|index| self.names.get(index).map(String::as_str))
    }
    fn typing(&self) -> bool {
        true
    }
    fn resize(&mut self, cols: usize, rows: usize) {
        self.set_dimensions(cols, rows);
    }
}

pub fn clean(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

pub fn cut(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_owned();
    }
    let prefix = width.saturating_sub(3);
    format!("{}...", text.chars().take(prefix).collect::<String>())
}

/// A menu of lines in the alternate screen: the selected line index, or `None` when the user quits.
pub fn choose(
    lines: &[String],
    title: &str,
    verb: &str,
    io_adapter: Option<&dyn crate::core::deps::Io>,
    say: &mut dyn FnMut(&str),
) -> Option<usize> {
    let (cols, rows) = io_adapter.map_or_else(terminal_dimensions, |io| io.dimensions());
    let width = cols.saturating_sub(8);
    let mut menu = Menu::new(
        lines.iter().map(|line| cut(&clean(line), width)).collect(),
        title,
        verb,
    );
    menu.set_dimensions(cols, rows);
    let result = match io_adapter.and_then(|io| io.play(&mut menu)) {
        Some(result) => result.unwrap_or_else(|error| error),
        None => play(&mut menu, false).unwrap_or_else(|error| error.to_string()),
    };
    if menu.chosen().is_none() && result != "Bye." {
        say(&format!("{result}\n"));
    }
    menu.chosen()
}

pub fn strip_ansi(text: &str) -> String {
    let mut chars = text.chars().peekable();
    let mut out = String::new();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for c in chars.by_ref() {
                if ('@'..='~').contains(&c) {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn dimensions(fd: libc::c_int) -> (usize, usize) {
    let mut size = std::mem::MaybeUninit::<libc::winsize>::zeroed();
    if unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, size.as_mut_ptr()) } == 0 {
        let size = unsafe { size.assume_init() };
        (size.ws_col as usize, size.ws_row as usize)
    } else {
        (80, 24)
    }
}

pub fn terminal_dimensions() -> (usize, usize) {
    dimensions(libc::STDOUT_FILENO)
}

pub fn interactive() -> bool {
    (unsafe { libc::isatty(libc::STDIN_FILENO) }) == 1
        && (unsafe { libc::isatty(libc::STDOUT_FILENO) }) == 1
}

struct Terminal {
    _cleanup: Cleanup,
}

struct Cleanup(Option<Box<dyn FnOnce()>>);

impl Cleanup {
    fn new(action: impl FnOnce() + 'static) -> Self {
        Self(Some(Box::new(action)))
    }
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        if let Some(action) = self.0.take() {
            action();
        }
    }
}

impl Terminal {
    fn enter() -> io::Result<Self> {
        let mut original = std::mem::MaybeUninit::<libc::termios>::uninit();
        if unsafe { libc::tcgetattr(libc::STDIN_FILENO, original.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let original = unsafe { original.assume_init() };
        let mut raw = original;
        unsafe { libc::cfmakeraw(&mut raw) };
        raw.c_cc[libc::VMIN] = 0;
        raw.c_cc[libc::VTIME] = 0;
        if unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &raw) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            _cleanup: Cleanup::new(move || {
                let _ = unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &original) };
                let _ = io::stdout().write_all(b"\x1b[0m\x1b[?25h\x1b[?1049l");
                let _ = io::stdout().flush();
            }),
        })
    }
}

static SIGNAL: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
extern "C" fn mark_signal(signal: libc::c_int) {
    SIGNAL.store(signal, std::sync::atomic::Ordering::SeqCst);
}

fn signal_exit_code(signal: i32) -> i32 {
    128 + signal
}

pub fn play(game: &mut dyn Game, wait: bool) -> io::Result<String> {
    if !interactive() {
        return Err(io::Error::new(
            io::ErrorKind::NotConnected,
            "games needs an interactive terminal",
        ));
    }
    let (cols, rows) = dimensions(libc::STDOUT_FILENO);
    game.resize(cols, rows.saturating_sub(1));
    let _terminal = Terminal::enter()?;
    SIGNAL.store(0, std::sync::atomic::Ordering::SeqCst);
    let mut old = Vec::new();
    for sig in [libc::SIGHUP, libc::SIGINT, libc::SIGTERM] {
        old.push((sig, unsafe {
            libc::signal(sig, mark_signal as *const () as libc::sighandler_t)
        }));
    }
    struct RestoreSignals(Vec<(libc::c_int, libc::sighandler_t)>);
    impl Drop for RestoreSignals {
        fn drop(&mut self) {
            for (sig, handler) in self.0.drain(..) {
                unsafe {
                    libc::signal(sig, handler);
                }
            }
        }
    }
    let _signals = RestoreSignals(old);
    let mut stdout = io::stdout().lock();
    stdout.write_all(b"\x1b[?1049h\x1b[?25l")?;
    let mut elapsed = std::time::Instant::now();
    let mut input = [0u8; 1];
    let mut pending_escape = None;
    let mut pending_bytes = Vec::new();
    let mut state = PlayState::default();
    loop {
        let signal = SIGNAL.load(std::sync::atomic::Ordering::SeqCst);
        if signal != 0 {
            let code = signal_exit_code(signal);
            drop(stdout);
            drop(_signals);
            drop(_terminal);
            std::process::exit(code);
        }
        let size = dimensions(libc::STDOUT_FILENO);
        if size != (cols, rows) {
            state.resized();
            return Ok(state.result.take().unwrap_or_default());
        }
        let mut pollfd = libc::pollfd {
            fd: libc::STDIN_FILENO,
            events: libc::POLLIN,
            revents: 0,
        };
        let wait_ms = if game.tick() == 0 {
            100
        } else {
            game.tick()
                .saturating_sub(elapsed.elapsed().as_millis() as u64)
                .min(100) as i32
        };
        unsafe {
            libc::poll(&mut pollfd, 1, wait_ms);
        }
        if pollfd.revents & libc::POLLIN != 0 {
            let n =
                unsafe { libc::read(libc::STDIN_FILENO, input.as_mut_ptr().cast(), input.len()) };
            if n > 0 {
                let bytes = &input[..n as usize];
                if bytes == b"\x1b" {
                    pending_bytes.clear();
                    pending_bytes.extend_from_slice(bytes);
                    pending_escape = Some(std::time::Instant::now());
                } else if pending_escape.is_some() {
                    pending_bytes.extend_from_slice(bytes);
                    if let Some(key) = decode_key(&pending_bytes) {
                        pending_escape = None;
                        pending_bytes.clear();
                        state.key(game, key);
                    }
                } else if let Some(key) = decode_key(bytes) {
                    state.key(game, key);
                }
            }
        }
        if pending_escape.is_some_and(|at| at.elapsed() >= std::time::Duration::from_millis(30)) {
            state.key(game, Key::Quit);
            pending_escape = None;
        }
        if let Some(result) = state.result.take() {
            return Ok(result);
        }
        if game.over().is_none()
            && game.tick() > 0
            && elapsed.elapsed().as_millis() as u64 >= game.tick()
        {
            game.step();
            elapsed = std::time::Instant::now();
        }
        let frame = game.draw().join(&format!("{CSI}K\r\n"));
        let message = game
            .over()
            .map(|text| format!("{CSI}{rows};1H{text} Press a key."))
            .unwrap_or_default();
        stdout.write_all(format!("{CSI}H{frame}{CSI}K{CSI}J{message}").as_bytes())?;
        stdout.flush()?;
        if let Some(message) = game.over()
            && !wait
        {
            return Ok(message.to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fg_and_bg_keep_the_purchasely_rgb_escape_bytes() {
        assert_eq!(fg("#ceecd0", "x"), "\x1b[38;2;206;236;208mx\x1b[39m");
        assert_eq!(bg("#002420", "x"), "\x1b[48;2;0;36;32mx\x1b[49m");
    }

    #[test]
    fn clean_and_cut_remove_controls_and_keep_short_text() {
        assert_eq!(clean("a\x1b[31mb\n"), "a [31mb ");
        assert_eq!(cut("short", 8), "short");
        assert_eq!(cut("abcdefgh", 6), "abc...");
    }

    #[test]
    fn menu_search_wraps_selection_and_returns_original_index() {
        let mut menu = Menu::new(
            vec![
                "Acme Fitness".into(),
                "Blue Radio".into(),
                "Acme Radio".into(),
            ],
            "Arcade",
            "play",
        );
        for c in "RADIO".chars() {
            menu.key(&c.to_string());
        }
        assert!(strip_ansi(&menu.draw(80, 24).join("\n")).contains("Search: RADIO_"));
        menu.key("down");
        menu.key("return");
        assert_eq!(menu.chosen(), Some(2));
    }

    #[test]
    fn menu_backspace_restores_matches_and_empty_matches_select_nothing() {
        let mut menu = Menu::new(vec!["one".into(), "two".into()], "Arcade", "play");
        menu.key("z");
        menu.key("return");
        assert_eq!(menu.chosen(), None);
        assert!(menu.draw(80, 24).join("\n").contains("No match."));
        menu.key("backspace");
        menu.key("return");
        assert_eq!(menu.chosen(), Some(0));
    }

    #[test]
    fn key_decoder_distinguishes_text_keys_and_control_keys() {
        assert_eq!(decode_key(b"q"), Some(Key::Text('q')));
        assert_eq!(decode_key(b"\x1b[A"), Some(Key::Named("up")));
        assert_eq!(decode_key(b"\x03"), Some(Key::Quit));
        assert_eq!(decode_key(b"\x1b"), Some(Key::Quit));
    }

    struct FakeGame {
        typing: bool,
        keys: Vec<String>,
        over: Option<String>,
    }

    impl Game for FakeGame {
        fn tick(&self) -> u64 {
            0
        }
        fn key(&mut self, name: &str) {
            self.keys.push(name.into());
        }
        fn step(&mut self) {}
        fn draw(&self) -> Vec<String> {
            vec!["board".into()]
        }
        fn over(&self) -> Option<&str> {
            self.over.as_deref()
        }
        fn typing(&self) -> bool {
            self.typing
        }
    }

    #[test]
    fn q_ends_play() {
        let mut game = FakeGame {
            typing: false,
            keys: vec![],
            over: None,
        };
        let mut state = PlayState::default();
        state.key(&mut game, Key::Text('q'));
        assert_eq!(state.result.as_deref(), Some("Bye."));
    }

    #[test]
    fn ctrl_c_ends_play() {
        let mut game = FakeGame {
            typing: false,
            keys: vec![],
            over: None,
        };
        let mut state = PlayState::default();
        state.key(&mut game, Key::Quit);
        assert_eq!(state.result.as_deref(), Some("Bye."));
    }

    fn cleanup_runs_when_unwinding(body: impl FnOnce()) -> bool {
        use std::{cell::Cell, rc::Rc};
        let restored = Rc::new(Cell::new(false));
        let flag = Rc::clone(&restored);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _cleanup = Cleanup::new(move || flag.set(true));
            body();
        }));
        assert!(result.is_err());
        restored.get()
    }

    #[test]
    fn restores_the_terminal_when_a_step_throws() {
        assert!(cleanup_runs_when_unwinding(|| panic!("step failed")));
    }

    #[test]
    fn restores_the_terminal_when_the_first_draw_throws() {
        assert!(cleanup_runs_when_unwinding(|| panic!("draw failed")));
    }

    #[test]
    fn signal_exit_code_is_128_plus_the_signal_number() {
        assert_eq!(signal_exit_code(libc::SIGTERM), 143);
        assert_eq!(signal_exit_code(libc::SIGHUP), 129);
        assert_eq!(signal_exit_code(libc::SIGINT), 130);
    }

    #[test]
    fn resize_sets_the_terminal_changed_message() {
        let mut state = PlayState::default();
        state.resized();
        assert_eq!(state.result.as_deref(), Some("The terminal changed size."));
    }

    #[test]
    fn typing_q_and_space_go_to_a_menu_while_escape_quits() {
        let mut menu = FakeGame {
            typing: true,
            keys: vec![],
            over: None,
        };
        let mut state = PlayState::default();
        state.key(&mut menu, Key::Text('q'));
        state.key(&mut menu, Key::Named("space"));
        assert_eq!(menu.keys, ["q", "space"]);
        state.key(&mut menu, Key::Quit);
        assert_eq!(state.result.as_deref(), Some("Bye."));
    }
}
