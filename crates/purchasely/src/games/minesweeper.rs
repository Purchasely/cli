//! Owner: P11b
use super::term::{self, BRAND, Game};
const COLORS: [&str; 9] = ["", "94", "32", "91", "34", "31", "36", "35", "90"];
const MOVES: [(&str, i32, i32); 8] = [
    ("up", 0, -1),
    ("w", 0, -1),
    ("down", 0, 1),
    ("s", 0, 1),
    ("left", -1, 0),
    ("a", -1, 0),
    ("right", 1, 0),
    ("d", 1, 0),
];
pub struct Minesweeper {
    cols: usize,
    width: usize,
    height: usize,
    mines: Vec<bool>,
    open: Vec<bool>,
    flags: Vec<bool>,
    x: usize,
    y: usize,
    start: Option<u64>,
    opened: usize,
    over: Option<String>,
    random: Box<dyn FnMut() -> f64>,
    now: Box<dyn FnMut() -> u64>,
}
impl Minesweeper {
    pub fn new(
        cols: usize,
        rows: usize,
        random: impl FnMut() -> f64 + 'static,
        now: impl FnMut() -> u64 + 'static,
    ) -> Result<Self, String> {
        if cols < 20 || rows < 9 {
            return Err(format!(
                "The terminal is too small for minesweeper: it needs 20x9 characters, it has {cols}x{rows}."
            ));
        }
        let width = (cols / 2).min(16);
        let height = (rows - 1).min(16);
        Ok(Self {
            cols,
            width,
            height,
            mines: vec![false; width * height],
            open: vec![false; width * height],
            flags: vec![false; width * height],
            x: width >> 1,
            y: height >> 1,
            start: None,
            opened: 0,
            over: None,
            random: Box::new(random),
            now: Box::new(now),
        })
    }
    fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }
    fn around(&self, i: usize) -> Vec<usize> {
        let x = i % self.width;
        let y = i / self.width;
        let mut cells = Vec::new();
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0 && nx < self.width as i32 && ny >= 0 && ny < self.height as i32 {
                    cells.push((ny as usize) * self.width + nx as usize);
                }
            }
        }
        cells
    }
    fn adjacent(&self, i: usize) -> usize {
        self.around(i)
            .into_iter()
            .filter(|j| self.mines[*j])
            .count()
    }
    fn place(&mut self, first: usize) {
        let mut safe = self.around(first);
        safe.push(first);
        let mut cells = (0..self.mines.len())
            .filter(|i| !safe.contains(i))
            .collect::<Vec<_>>();
        let n = (self.width * self.height * 40 + 128) / 256;
        for k in 0..n {
            let j = k
                + (((self.random)() * (cells.len() - k) as f64) as usize).min(cells.len() - k - 1);
            cells.swap(k, j);
            self.mines[cells[k]] = true;
        }
        self.start = Some((self.now)());
    }
    fn reveal(&mut self, i: usize) {
        if self.open[i] || self.flags[i] {
            return;
        }
        if self.opened == 0 {
            self.place(i);
        }
        if self.mines[i] {
            self.over = Some("Boom! You hit a mine.".into());
            return;
        }
        let mut todo = vec![i];
        while let Some(j) = todo.pop() {
            if self.open[j] || self.flags[j] {
                continue;
            }
            self.open[j] = true;
            self.opened += 1;
            if self.adjacent(j) == 0 {
                todo.extend(self.around(j));
            }
        }
        let count = (self.width * self.height * 40 + 128) / 256;
        if self.opened == self.width * self.height - count {
            let elapsed = ((self.now)() - self.start.unwrap_or(0) + 500) / 1000;
            self.over = Some(format!("You cleared the board in {elapsed} s."));
        }
    }
    pub fn over(&self) -> Option<&str> {
        self.over.as_deref()
    }
    pub fn draw(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for y in 0..self.height {
            let mut line = String::new();
            for x in 0..self.width {
                let i = self.index(x, y);
                let text = if self.flags[i] {
                    term::fg(BRAND.coral, "P")
                } else if !self.open[i] {
                    "\x1b[2m·\x1b[22m".into()
                } else {
                    let n = self.adjacent(i);
                    if n == 0 {
                        " ".into()
                    } else {
                        format!("\x1b[{}m{n}\x1b[39m", COLORS[n])
                    }
                };
                let cell = format!("{text} ");
                if x == self.x && y == self.y {
                    line.push_str(&format!("\x1b[7m{cell}\x1b[27m"));
                } else {
                    line.push_str(&cell);
                }
            }
            lines.push(line);
        }
        lines.push(format!(
            "{}Mines {}  q to quit",
            if self.cols >= 32 {
                format!("{}  ", term::fg(BRAND.lavender, "Purchasely"))
            } else {
                String::new()
            },
            (self.width * self.height * 40 + 128) / 256
                - self.flags.iter().filter(|flag| **flag).count()
        ));
        lines
    }
}
impl Game for Minesweeper {
    fn tick(&self) -> u64 {
        0
    }
    fn key(&mut self, name: &str) {
        if let Some((_, dx, dy)) = MOVES.iter().find(|(key, _, _)| *key == name) {
            self.x = (self.x as i32 + dx).clamp(0, self.width as i32 - 1) as usize;
            self.y = (self.y as i32 + dy).clamp(0, self.height as i32 - 1) as usize;
        } else {
            let i = self.index(self.x, self.y);
            match name {
                "space" | "return" => self.reveal(i),
                "f" if !self.open[i] => self.flags[i] = !self.flags[i],
                _ => {}
            }
        }
    }
    fn step(&mut self) {}
    fn draw(&self) -> Vec<String> {
        Minesweeper::draw(self)
    }
    fn over(&self) -> Option<&str> {
        self.over()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::term::{self, Game};

    fn make(random: impl FnMut() -> f64 + 'static) -> Minesweeper {
        Minesweeper::new(20, 9, random, || 0).unwrap()
    }
    fn plain(game: &impl Game) -> Vec<String> {
        game.draw()
            .iter()
            .map(|line| term::strip_ansi(line))
            .collect()
    }
    fn board(game: &impl Game) -> Vec<String> {
        plain(game)
            .iter()
            .take(8)
            .map(|line| line.chars().step_by(2).collect())
            .collect()
    }
    fn go(game: &mut impl Game, x: usize, y: usize) {
        for _ in 0..10 {
            game.key("a");
        }
        for _ in 0..8 {
            game.key("w");
        }
        for _ in 0..x {
            game.key("d");
        }
        for _ in 0..y {
            game.key("s");
        }
    }

    #[test]
    fn makes_the_first_reveal_safe_and_flood_fills_its_area() {
        for seed in 1..=20 {
            let mut state = seed as u64;
            let mut game = make(move || {
                state = state.wrapping_mul(16807) % 2147483647;
                state as f64 / 2147483647.0
            });
            game.key("space");
            assert!(!game.over().unwrap_or("").contains("Boom"));
            for row in board(&game).iter().skip(3).take(3) {
                assert!(
                    !row.chars()
                        .skip(4)
                        .take(3)
                        .collect::<String>()
                        .contains('·')
                );
            }
        }
    }
    #[test]
    fn blocks_a_reveal_on_a_flag() {
        let mut state = 1u64;
        let mut game = make(move || {
            state = state.wrapping_mul(16807) % 2147483647;
            state as f64 / 2147483647.0
        });
        game.key("f");
        game.key("return");
        assert_eq!(board(&game)[4].chars().nth(5), Some('P'));
        assert_eq!(
            board(&game)
                .iter()
                .map(|row| row.matches('·').count())
                .sum::<usize>(),
            79
        );
        assert_eq!(plain(&game).last().unwrap(), "Mines 12  q to quit");
        game.key("f");
        assert_eq!(plain(&game).last().unwrap(), "Mines 13  q to quit");
    }
    #[test]
    fn ends_the_game_on_a_mine() {
        let mut state = 1u64;
        let mut game = make(move || {
            state = state.wrapping_mul(16807) % 2147483647;
            state as f64 / 2147483647.0
        });
        game.key("space");
        for i in 0..80 {
            if game.over().is_some() {
                break;
            }
            let current = board(&game);
            let x = i % 10;
            let y = i / 10;
            if current[y].chars().nth(x) == Some('·') {
                go(&mut game, x, y);
                game.key("space");
            }
        }
        assert_eq!(game.over(), Some("Boom! You hit a mine."));
    }
    #[test]
    fn wins_when_all_the_safe_cells_are_open() {
        let mut times = [1_000, 43_000].into_iter();
        let mut game = Minesweeper::new(20, 9, || 0.0, move || times.next().unwrap()).unwrap();
        go(&mut game, 9, 7);
        game.key("return");
        assert_eq!(game.over(), Some("You cleared the board in 42 s."));
    }
    #[test]
    fn refuses_a_terminal_that_is_too_small() {
        assert!(
            Minesweeper::new(19, 30, || 0.0, || 0)
                .err()
                .unwrap()
                .contains("too small for minesweeper")
        );
    }
}
