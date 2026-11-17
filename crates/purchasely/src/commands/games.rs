//! Owner: P11a
use crate::{
    cli::{Argument, Command},
    core::{
        errors::{CliError, usage},
        runtime::Runtime,
    },
    games::{
        g2048::Game2048,
        invaders::Invaders,
        minesweeper::Minesweeper,
        pong::Pong,
        snake::Snake,
        term::{self, Game, Menu},
        tetris::Tetris,
    },
};

pub const GAMES: [&str; 6] = ["2048", "invaders", "minesweeper", "pong", "snake", "tetris"];

pub fn validate_game(name: &str) -> Result<(), CliError> {
    if GAMES.contains(&name) {
        Ok(())
    } else {
        Err(usage(format!(
            "Unknown game \"{name}\". Games: {}.",
            GAMES.join(", ")
        )))
    }
}

pub fn require_terminal(label: &str, is_tty: bool) -> Result<(), CliError> {
    if is_tty {
        Ok(())
    } else {
        Err(usage(format!("{label} needs an interactive terminal.")))
    }
}

fn random_unit() -> f64 {
    let mut bytes = [0u8; 8];
    if getrandom::fill(&mut bytes).is_err() {
        return 0.5;
    }
    (u64::from_ne_bytes(bytes) >> 11) as f64 / (1u64 << 53) as f64
}

/// TS `Date.now()`: the minesweeper clock.
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

fn report_terminal_size(message: String) -> String {
    let mut out = String::new();
    let mut chars = message.chars().peekable();
    while let Some(ch) = chars.next() {
        out.push(ch);
        if ch == 'x' {
            let mut digits = String::new();
            while chars.peek().is_some_and(char::is_ascii_digit) {
                digits.push(chars.next().unwrap());
            }
            if !digits.is_empty() {
                out.push_str(&(digits.parse::<usize>().unwrap_or(0) + 1).to_string());
            }
        }
    }
    out
}

fn make_game(name: &str, cols: usize, rows: usize) -> Result<Box<dyn Game>, CliError> {
    match name {
        "2048" => {
            Game2048::new(cols, rows, random_unit).map(|game| Box::new(game) as Box<dyn Game>)
        }
        "snake" => Snake::new(cols, rows, random_unit).map(|game| Box::new(game) as Box<dyn Game>),
        "pong" => Pong::new(cols, rows, random_unit).map(|game| Box::new(game) as Box<dyn Game>),
        "invaders" => {
            Invaders::new(cols, rows, random_unit).map(|game| Box::new(game) as Box<dyn Game>)
        }
        "minesweeper" => Minesweeper::new(cols, rows, random_unit, now_ms)
            .map(|game| Box::new(game) as Box<dyn Game>),
        "tetris" => {
            Tetris::new(cols, rows, random_unit).map(|game| Box::new(game) as Box<dyn Game>)
        }
        _ => Err(format!("Game {name} is not available in this build.")),
    }
    .map_err(|message| usage(report_terminal_size(message)))
}

pub fn play(name: Option<&str>, label: &str) -> Result<String, CliError> {
    if let Some(name) = name {
        validate_game(name)?;
    }
    require_terminal(label, term::interactive())?;
    let selected = if let Some(name) = name {
        name.to_owned()
    } else {
        let mut menu = Menu::new(
            GAMES.iter().map(|name| (*name).to_owned()).collect(),
            "Arcade",
            "play",
        );
        let result = term::play(&mut menu, false).map_err(|error| usage(error.to_string()))?;
        if result == "Bye." {
            return Ok(result);
        }
        result
    };
    let (cols, rows) = term::terminal_dimensions();
    let mut game = make_game(&selected, cols, rows.saturating_sub(1))?;
    term::play(game.as_mut(), true).map_err(|error| usage(error.to_string()))
}

pub fn commands(name: &str) -> (Command, Command) {
    let arg = Argument::optional(
        "[game]",
        "The name of the game. Without a name, a menu shows the games",
    );
    let after_help = format!(
        "\nGames: {}.\nKeys: arrows or WASD move, space acts, q, Esc or Ctrl-C quits.\nIt needs an interactive terminal. It uses no key and sends no request.\nExample:\n  {name} games snake\n",
        GAMES.join(", ")
    );
    let action = |label: &'static str| {
        move |p: &crate::cli::Parsed, rt: &mut Runtime| {
            let result = play(p.arguments.first().map(String::as_str), label)?;
            rt.d.host.stdout(&format!("{result}\n"));
            Ok(())
        }
    };
    let visible = Command::new("games")
        .description("Play a game in the terminal")
        .argument(arg.clone())
        .after_help(after_help)
        .action(action("games"));
    let hidden = Command::new("play")
        .description("Play a game in the terminal")
        .argument(arg)
        .hide()
        .action(action("play"));
    (visible, hidden)
}

pub fn register(program: &mut Command, rt: &mut Runtime) {
    let (visible, hidden) = commands(&rt.d.name);
    program.commands.push(visible);
    program.commands.push(hidden);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn games_is_visible_play_is_hidden_and_both_share_the_optional_argument() {
        let (games, play) = commands("purchasely");
        assert_eq!(games.name, "games");
        assert!(!games.hidden);
        assert_eq!(games.arguments.len(), 1);
        assert_eq!(games.arguments[0].syntax, "[game]");
        assert_eq!(play.name, "play");
        assert!(play.hidden);
        assert_eq!(play.arguments[0].syntax, "[game]");
    }

    #[test]
    fn games_help_lists_names_keys_terminal_limit_and_example() {
        let (games, _) = commands("acme");
        for text in [
            "2048",
            "invaders",
            "minesweeper",
            "pong",
            "snake",
            "tetris",
            "arrows or WASD",
            "space",
            "Esc",
            "Ctrl-C",
            "needs an interactive terminal",
            "uses no key and sends no request",
            "acme games snake",
        ] {
            assert!(games.after_help.contains(text), "missing {text}");
        }
    }

    #[test]
    fn unknown_game_is_a_usage_error_with_the_available_names() {
        let error = validate_game("nope").unwrap_err();
        assert_eq!(error.exit, 2);
        assert!(error.message().contains("Unknown game \"nope\""));
        assert!(
            error
                .message()
                .contains("2048, invaders, minesweeper, pong, snake, tetris")
        );
    }

    #[test]
    fn menu_requires_a_terminal_before_starting() {
        assert_eq!(
            require_terminal("games", false).unwrap_err().message(),
            "games needs an interactive terminal."
        );
        assert!(require_terminal("games", true).is_ok());
    }

    #[test]
    /// game-term.test.ts "play on a small terminal > ${name} reports the size the terminal needs, and that
    /// size works", for its 6 rows. `play` gives the game one row less than the terminal: so does this.
    #[test]
    fn each_game_reports_the_size_the_terminal_needs_and_that_size_works() {
        let start = |name: &str, cols: usize, rows: usize| make_game(name, cols, rows - 1);
        for name in GAMES {
            let error = start(name, 10, 5).err().unwrap().messages[0].clone();
            assert!(error.contains(&format!("too small for {name}")), "{error}");
            assert!(error.contains("10x5"), "{error}");
            let need = error.split("needs ").nth(1).unwrap();
            let (cols, rest) = need.split_once('x').unwrap();
            let rows = rest
                .chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>();
            let (cols, rows) = (cols.parse().unwrap(), rows.parse().unwrap());
            let mut game = start(name, cols, rows).unwrap_or_else(|e| panic!("{name}: {e:?}"));
            game.key("q");
            game.draw();
            let tight = start(name, cols, rows - 1).err().unwrap().messages[0].clone();
            assert!(tight.contains("too small"), "{name}: {tight}");
        }
    }
    #[test]
    fn small_terminal_messages_report_the_full_terminal_height() {
        assert_eq!(
            report_terminal_size("needs 22x8; got 10x5".into()),
            "needs 22x9; got 10x6"
        );
    }
}
