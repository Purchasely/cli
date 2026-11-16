//! Owner: P11b
use super::term::{self, BRAND, Game};

const W: usize = 10;
const H: usize = 20;
const MIN_COLS: usize = 40;
const MIN_ROWS: usize = H + 1;
type Cell = (usize, usize);
struct Piece {
    size: usize,
    cells: &'static [Cell],
    color: &'static str,
}
const PIECES: [Piece; 7] = [
    Piece {
        size: 4,
        cells: &[(0, 1), (1, 1), (2, 1), (3, 1)],
        color: BRAND.light_green,
    },
    Piece {
        size: 2,
        cells: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        color: BRAND.cream,
    },
    Piece {
        size: 3,
        cells: &[(1, 0), (0, 1), (1, 1), (2, 1)],
        color: BRAND.lavender,
    },
    Piece {
        size: 3,
        cells: &[(1, 0), (2, 0), (0, 1), (1, 1)],
        color: BRAND.sage,
    },
    Piece {
        size: 3,
        cells: &[(0, 0), (1, 0), (1, 1), (2, 1)],
        color: BRAND.coral,
    },
    Piece {
        size: 3,
        cells: &[(0, 0), (0, 1), (1, 1), (2, 1)],
        color: BRAND.white,
    },
    Piece {
        size: 3,
        cells: &[(2, 0), (0, 1), (1, 1), (2, 1)],
        color: "#deb8d8",
    },
];
const POINTS: [u32; 5] = [0, 100, 300, 500, 800];
const KICKS: [i32; 5] = [0, -1, 1, -2, 2];
fn paint(piece: usize) -> String {
    term::fg(PIECES[piece].color, "██")
}

pub struct Tetris {
    board: [[i8; W]; H],
    piece: usize,
    next: usize,
    cells: Vec<Cell>,
    x: usize,
    y: usize,
    score: u32,
    lines: u32,
    frames: u32,
    over: Option<String>,
    random: Box<dyn FnMut() -> f64>,
}

impl Tetris {
    pub fn new(
        cols: usize,
        rows: usize,
        random: impl FnMut() -> f64 + 'static,
    ) -> Result<Self, String> {
        if cols < MIN_COLS || rows < MIN_ROWS {
            return Err(format!(
                "The terminal is too small for tetris: {cols}x{rows}, it needs {MIN_COLS}x{MIN_ROWS}."
            ));
        }
        let mut game = Self {
            board: [[-1; W]; H],
            piece: 0,
            next: 0,
            cells: Vec::new(),
            x: 0,
            y: 0,
            score: 0,
            lines: 0,
            frames: 0,
            over: None,
            random: Box::new(random),
        };
        game.next = game.pick();
        game.spawn();
        Ok(game)
    }
    fn pick(&mut self) -> usize {
        (((self.random)() * PIECES.len() as f64) as usize).min(PIECES.len() - 1)
    }
    fn fits(&self, cells: &[Cell], dx: i32, dy: i32) -> bool {
        cells.iter().all(|(cx, cy)| {
            let x = self.x as i32 + dx + *cx as i32;
            let y = self.y as i32 + dy + *cy as i32;
            x >= 0
                && x < W as i32
                && y >= 0
                && y < H as i32
                && self.board[y as usize][x as usize] == -1
        })
    }
    fn move_by(&mut self, dx: i32, dy: i32) -> bool {
        if !self.fits(&self.cells, dx, dy) {
            return false;
        }
        self.x = (self.x as i32 + dx) as usize;
        self.y = (self.y as i32 + dy) as usize;
        true
    }
    fn spawn(&mut self) {
        self.piece = self.next;
        self.next = self.pick();
        self.cells = PIECES[self.piece].cells.to_vec();
        self.x = (W - PIECES[self.piece].size) / 2;
        self.y = 0;
        if !self.fits(&self.cells, 0, 0) {
            self.over = Some(format!(
                "Game over. Score {}, {} lines.",
                self.score, self.lines
            ));
        }
    }
    fn lock(&mut self) {
        for (cx, cy) in &self.cells {
            self.board[self.y + cy][self.x + cx] = self.piece as i8;
        }
        let kept: Vec<_> = self
            .board
            .iter()
            .copied()
            .filter(|row| row.contains(&-1))
            .collect();
        let cleared = H - kept.len();
        self.board = [[-1; W]; H];
        self.board[cleared..].copy_from_slice(&kept);
        self.score += POINTS[cleared];
        self.lines += cleared as u32;
        self.frames = 0;
        self.spawn();
    }
    fn rotate(&mut self) {
        let size = PIECES[self.piece].size;
        let turned: Vec<_> = self.cells.iter().map(|(x, y)| (size - 1 - y, *x)).collect();
        if let Some(dx) = KICKS.iter().copied().find(|dx| self.fits(&turned, *dx, 0)) {
            self.cells = turned;
            self.x = (self.x as i32 + dx) as usize;
        }
    }
    pub fn over(&self) -> Option<&str> {
        self.over.as_deref()
    }
    pub fn draw(&self) -> Vec<String> {
        let mut grid = self.board;
        for (cx, cy) in &self.cells {
            grid[self.y + cy][self.x + cx] = self.piece as i8;
        }
        let mut preview = vec![String::new(); 2];
        for (py, line) in preview.iter_mut().enumerate() {
            for px in 0..4 {
                if PIECES[self.next].cells.contains(&(px, py)) {
                    line.push_str(&paint(self.next));
                } else {
                    line.push_str("  ");
                }
            }
        }
        let side = [
            term::fg(BRAND.lavender, "PURCHASELY"),
            "Tetris".into(),
            "".into(),
            "Next".into(),
            preview[0].clone(),
            preview[1].clone(),
            "".into(),
            format!("Score {}", self.score),
            format!("Lines {}", self.lines),
            format!("Level {}", 1 + self.lines / 10),
            "".into(),
            "←→ move".into(),
            "↑ rotate".into(),
            "↓ soft drop".into(),
            "space hard drop".into(),
            "q to quit".into(),
        ];
        let mut output: Vec<_> = grid
            .iter()
            .enumerate()
            .map(|(i, row)| {
                format!(
                    "│{}│  {}",
                    row.iter()
                        .map(|cell| if *cell < 0 {
                            " .".to_owned()
                        } else {
                            paint(*cell as usize)
                        })
                        .collect::<String>(),
                    side.get(i).cloned().unwrap_or_default()
                )
            })
            .collect();
        output.push(format!("└{}┘", "──".repeat(W)));
        output
    }
}
impl Game for Tetris {
    fn tick(&self) -> u64 {
        50
    }
    fn key(&mut self, name: &str) {
        match name {
            "left" => {
                self.move_by(-1, 0);
            }
            "right" => {
                self.move_by(1, 0);
            }
            "up" => self.rotate(),
            "down" if !self.move_by(0, 1) => self.lock(),
            "space" => {
                while self.move_by(0, 1) {}
                self.lock();
            }
            _ => {}
        }
    }
    fn step(&mut self) {
        self.frames += 1;
        if self.frames < (16u32.saturating_sub(self.lines / 10)).max(1) {
            return;
        }
        self.frames = 0;
        if !self.move_by(0, 1) {
            self.lock();
        }
    }
    fn draw(&self) -> Vec<String> {
        Tetris::draw(self)
    }
    fn over(&self) -> Option<&str> {
        self.over()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::term::Game;

    const I: usize = 0;
    const O: usize = 1;

    fn seq(pieces: &[usize]) -> impl FnMut() -> f64 + 'static {
        let pieces = pieces.to_vec();
        let mut i = 0;
        move || {
            let piece = pieces[i % pieces.len()];
            i += 1;
            (piece as f64 + 0.5) / 7.0
        }
    }

    fn plain_rows(game: &impl Game) -> Vec<String> {
        game.draw()
            .iter()
            .take(20)
            .map(|line| {
                let plain = crate::games::term::strip_ansi(line);
                plain
                    .chars()
                    .skip(1)
                    .take(20)
                    .collect::<String>()
                    .replace("██", "#")
                    .replace(" .", ".")
            })
            .collect()
    }

    fn key(game: &mut impl Game, keys: &[&str]) {
        for key in keys {
            game.key(key);
        }
    }

    #[test]
    fn refuses_a_small_terminal() {
        assert!(
            Tetris::new(30, 10, || 0.0)
                .err()
                .unwrap()
                .contains("too small for tetris")
        );
    }

    #[test]
    fn hard_drops_a_piece_to_the_floor() {
        let mut game = Tetris::new(80, 24, seq(&[I])).unwrap();
        game.key("space");
        assert_eq!(plain_rows(&game)[19], "...####...");
        assert!(game.draw().join("\n").contains("q to quit"));
    }

    #[test]
    fn clears_a_full_line_and_scores_it() {
        let mut game = Tetris::new(80, 24, seq(&[I, I, O])).unwrap();
        key(
            &mut game,
            &[
                "left", "left", "left", "space", "right", "right", "right", "space", "space",
            ],
        );
        assert_eq!(plain_rows(&game)[19], "....##....");
        let screen = game.draw().join("\n");
        assert!(screen.contains("Score 100"));
        assert!(screen.contains("Lines 1"));
    }

    #[test]
    fn kicks_a_rotation_away_from_the_wall() {
        let mut game = Tetris::new(80, 24, seq(&[I])).unwrap();
        key(
            &mut game,
            &["up", "left", "left", "left", "left", "left", "up", "space"],
        );
        assert_eq!(plain_rows(&game)[19], "####......");
    }

    #[test]
    fn ends_when_a_new_piece_cannot_spawn() {
        let mut game = Tetris::new(80, 24, seq(&[O])).unwrap();
        for _ in 0..20 {
            if game.over().is_some() {
                break;
            }
            game.key("space");
        }
        assert_eq!(game.over(), Some("Game over. Score 0, 0 lines."));
    }
}
