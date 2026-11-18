//! Owner: P11a
use super::term::{self, BRAND, Game};

const WIDTH: usize = 35;
const HEIGHT: usize = 17;
const CELL_WIDTH: usize = 8;
const DIRS: [&str; 4] = ["left", "right", "up", "down"];

pub fn slide(line: &[u32]) -> (Vec<u32>, u32) {
    let tiles = line.iter().copied().filter(|v| *v != 0).collect::<Vec<_>>();
    let mut out = Vec::with_capacity(line.len());
    let mut score = 0;
    let mut i = 0;
    while i < tiles.len() {
        if tiles.get(i + 1) == Some(&tiles[i]) {
            let value = tiles[i] * 2;
            out.push(value);
            score += value;
            i += 2;
        } else {
            out.push(tiles[i]);
            i += 1;
        }
    }
    out.resize(line.len(), 0);
    (out, score)
}

fn index(dir: &str, i: usize, j: usize) -> (usize, usize) {
    match dir {
        "right" => (i, 3 - j),
        "up" => (j, i),
        "down" => (3 - j, i),
        _ => (i, j),
    }
}

fn move_grid(grid: &[[u32; 4]; 4], dir: &str) -> ([[u32; 4]; 4], u32) {
    let mut out = *grid;
    let mut score = 0;
    for i in 0..4 {
        let line: [u32; 4] = std::array::from_fn(|j| {
            let (r, c) = index(dir, i, j);
            grid[r][c]
        });
        let (line, line_score) = slide(&line);
        score += line_score;
        for (j, value) in line.into_iter().enumerate() {
            let (r, c) = index(dir, i, j);
            out[r][c] = value;
        }
    }
    (out, score)
}

pub struct Game2048 {
    grid: [[u32; 4]; 4],
    score: u32,
    cols: usize,
    over: Option<String>,
    random: Box<dyn FnMut() -> f64>,
}

impl Game2048 {
    pub fn new(
        cols: usize,
        rows: usize,
        random: impl FnMut() -> f64 + 'static,
    ) -> Result<Self, String> {
        if cols < WIDTH || rows < HEIGHT {
            return Err(format!(
                "The terminal is too small for 2048: it needs {WIDTH}x{HEIGHT}, it has {cols}x{rows}."
            ));
        }
        let mut game = Self {
            grid: [[0; 4]; 4],
            score: 0,
            cols,
            over: None,
            random: Box::new(random),
        };
        game.spawn();
        game.spawn();
        Ok(game)
    }

    fn best(&self) -> u32 {
        self.grid.iter().flatten().copied().max().unwrap_or(0)
    }

    fn spawn(&mut self) {
        let empty = self
            .grid
            .iter()
            .enumerate()
            .flat_map(|(r, row)| {
                row.iter()
                    .enumerate()
                    .filter_map(move |(c, v)| (*v == 0).then_some((r, c)))
            })
            .collect::<Vec<_>>();
        if empty.is_empty() {
            return;
        }
        let rand = (self.random)();
        let pos = ((rand * empty.len() as f64) as usize).min(empty.len() - 1);
        let value = if (self.random)() < 0.9 { 2 } else { 4 };
        let (r, c) = empty[pos];
        self.grid[r][c] = value;
    }

    pub fn key(&mut self, name: &str) {
        let dir = match name {
            "left" | "a" => "left",
            "right" | "d" => "right",
            "up" | "w" => "up",
            "down" | "s" => "down",
            _ => return,
        };
        let (next, score) = move_grid(&self.grid, dir);
        if next == self.grid {
            return;
        }
        self.grid = next;
        self.score += score;
        self.spawn();
        if DIRS
            .iter()
            .all(|direction| move_grid(&self.grid, direction).0 == self.grid)
        {
            self.over = Some(format!(
                "Game over. Score {}, best tile {}.",
                self.score,
                self.best()
            ));
        }
    }

    pub fn over(&self) -> Option<&str> {
        self.over.as_deref()
    }

    pub fn draw(&self) -> Vec<String> {
        let width = 4 * CELL_WIDTH + 3;
        let pad = " ".repeat(self.cols.saturating_sub(width) >> 1);
        let colors = [
            BRAND.light_green,
            BRAND.cream,
            BRAND.sage,
            BRAND.lavender,
            BRAND.coral,
            "#deb8d8",
            BRAND.white,
        ];
        let mut lines = vec![term::fg(BRAND.lavender, "PURCHASELY 2048")];
        for (i, row) in self.grid.iter().enumerate() {
            if i != 0 {
                lines.push(String::new());
            }
            for subrow in 0..3 {
                let cells = row
                    .iter()
                    .map(|value| {
                        let text = if subrow == 1 && *value != 0 {
                            value.to_string()
                        } else {
                            String::new()
                        };
                        let left = (CELL_WIDTH.saturating_sub(text.len())) >> 1;
                        let content = format!(
                            "{}{}{}",
                            " ".repeat(left),
                            text,
                            " ".repeat(CELL_WIDTH.saturating_sub(left + text.len()))
                        );
                        let color = if *value == 0 {
                            "#3c5852"
                        } else {
                            colors[((value.ilog2() as usize).saturating_sub(1)) % colors.len()]
                        };
                        term::bg(
                            color,
                            &term::fg(BRAND.dark_green, &format!("\x1b[1m{content}\x1b[22m")),
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                lines.push(format!("{pad}{cells}"));
            }
        }
        lines.push(format!(
            "{pad}Score {}{}  q to quit",
            self.score,
            if self.best() >= 2048 {
                "  You win!"
            } else {
                ""
            }
        ));
        lines
    }
}

impl Game for Game2048 {
    fn tick(&self) -> u64 {
        0
    }
    fn key(&mut self, name: &str) {
        Game2048::key(self, name);
    }
    fn step(&mut self) {}
    fn draw(&self) -> Vec<String> {
        Game2048::draw(self)
    }
    fn over(&self) -> Option<&str> {
        self.over()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded(mut state: u64) -> impl FnMut() -> f64 {
        move || {
            state = (state.wrapping_mul(1103515245).wrapping_add(12345)) % (1 << 31);
            state as f64 / (1u64 << 31) as f64
        }
    }

    #[test]
    fn merges_equal_tiles_once_per_move() {
        assert_eq!(slide(&[2, 2, 2, 2]), (vec![4, 4, 0, 0], 8));
        assert_eq!(slide(&[2, 2, 4, 0]), (vec![4, 4, 0, 0], 4));
        assert_eq!(slide(&[0, 4, 0, 4]), (vec![8, 0, 0, 0], 8));
        assert_eq!(slide(&[2, 4, 8, 16]), (vec![2, 4, 8, 16], 0));
    }

    #[test]
    fn spawns_a_tile_only_after_a_move_that_changes_the_grid() {
        let mut game = Game2048::new(80, 24, || 0.0).unwrap();
        game.key("left");
        let before = game.draw().join("\n");
        assert!(before.contains("Score 4"));
        for key in ["left", "a", "up", "w", "x"] {
            game.key(key);
        }
        assert_eq!(game.draw().join("\n"), before);
    }

    #[test]
    fn ends_when_no_move_is_possible() {
        let mut game = Game2048::new(80, 24, seeded(42)).unwrap();
        for n in 0..100_000 {
            if game.over.is_some() {
                break;
            }
            game.key(["up", "left", "down", "right"][n % 4]);
        }
        // TS: /^Game over\. Score \d+, best tile \d+\.$/
        let over = game.over.as_deref().unwrap();
        let rest = over.strip_prefix("Game over. Score ").unwrap();
        let (score, tile) = rest.split_once(", best tile ").unwrap();
        let tile = tile.strip_suffix('.').unwrap();
        for n in [score, tile] {
            assert!(
                !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()),
                "{over}"
            );
        }
    }

    #[test]
    fn refuses_a_terminal_that_is_too_small() {
        assert!(
            Game2048::new(20, 10, || 0.0)
                .err()
                .unwrap()
                .contains("too small for 2048")
        );
    }
}
