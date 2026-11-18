//! Owner: P11a
use super::term::{self, BRAND, Game};

const PADDLE: i32 = 4;
const STEP: i32 = 2;
const CPU: f64 = 0.5;
const WIN: u8 = 7;

pub struct Pong {
    width: i32,
    height: i32,
    player: i32,
    cpu: f64,
    player_score: u8,
    cpu_score: u8,
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    over: Option<String>,
    random: Box<dyn FnMut() -> f64>,
}

impl Pong {
    pub fn new(
        cols: usize,
        rows: usize,
        random: impl FnMut() -> f64 + 'static,
    ) -> Result<Self, String> {
        let width = (cols.saturating_sub(2)).min(78) as i32;
        let height = (rows.saturating_sub(3)).min(22) as i32;
        if width < 44 || height < 10 {
            return Err(format!(
                "The terminal is too small for pong: {cols}x{rows}, it needs 46x13."
            ));
        }
        let mut game = Self {
            width,
            height,
            player: (height - PADDLE) >> 1,
            cpu: ((height - PADDLE) >> 1) as f64,
            player_score: 0,
            cpu_score: 0,
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            over: None,
            random: Box::new(random),
        };
        let direction = if (game.random)() < 0.5 { 1.0 } else { -1.0 };
        game.serve(direction);
        Ok(game)
    }

    fn serve(&mut self, direction: f64) {
        self.x = (self.width >> 1) as f64;
        self.y = (self.height >> 1) as f64;
        self.vx = direction;
        self.vy = (self.random)() - 0.5;
    }

    pub fn key(&mut self, key: &str) {
        if key == "up" || key == "w" {
            self.player = 0.max(self.player - STEP);
        } else if key == "down" || key == "s" {
            self.player = (self.height - PADDLE).min(self.player + STEP);
        }
    }

    pub fn step(&mut self) {
        let right = self.width - 2;
        let left = 1;
        self.cpu = (self.cpu + (self.y - (PADDLE - 1) as f64 / 2.0 - self.cpu).clamp(-CPU, CPU))
            .clamp(0.0, (self.height - PADDLE) as f64);
        self.y += self.vy;
        if self.y < 0.0 || self.y > (self.height - 1) as f64 {
            self.vy = -self.vy;
            self.y = if self.y < 0.0 {
                -self.y
            } else {
                2.0 * (self.height - 1) as f64 - self.y
            };
        }
        let nx = self.x + self.vx;
        let paddle = if nx == left as f64 {
            Some(self.player)
        } else if nx == right as f64 {
            Some(self.cpu.round() as i32)
        } else {
            None
        };
        let offset = paddle.map(|p| self.y.round() as i32 - p);
        if offset.is_some_and(|off| (0..PADDLE).contains(&off)) {
            self.vx = -self.vx;
            self.vy = (offset.unwrap() as f64 - (PADDLE - 1) as f64 / 2.0) / 2.0;
        } else {
            self.x = nx;
        }
        if self.x >= 0.0 && self.x < self.width as f64 {
            return;
        }
        if self.x < 0.0 {
            self.cpu_score += 1;
        } else {
            self.player_score += 1;
        }
        if self.player_score == WIN || self.cpu_score == WIN {
            self.over = Some(format!(
                "You {} {}-{}",
                if self.player_score > self.cpu_score {
                    "win"
                } else {
                    "lose"
                },
                self.player_score,
                self.cpu_score
            ));
        } else {
            self.serve(if self.x < 0.0 { -1.0 } else { 1.0 });
        }
    }

    pub fn draw(&self) -> Vec<String> {
        let left = 1;
        let right = self.width - 2;
        let center = self.width >> 1;
        let score = format!(
            " Purchasely Pong   You {} : {} CPU",
            self.player_score, self.cpu_score
        );
        let suffix = "q to quit";
        let gap = (self.width as usize + 2).saturating_sub(score.len() + suffix.len());
        let mut lines = vec![format!("{score}{}{suffix}", " ".repeat(gap))];
        lines.push(format!("┌{}┐", "─".repeat(self.width as usize)));
        for row in 0..self.height {
            let mut cells = vec![" ".to_owned(); self.width as usize];
            if row % 2 == 0 {
                cells[center as usize] = "│".into();
            }
            if (self.player..self.player + PADDLE).contains(&row) {
                cells[left as usize] = term::fg(BRAND.light_green, "█");
            }
            if ((self.cpu.round() as i32)..(self.cpu.round() as i32) + PADDLE).contains(&row) {
                cells[right as usize] = term::fg(BRAND.lavender, "█");
            }
            if self.x >= 0.0 && self.x < self.width as f64 && self.y.round() as i32 == row {
                cells[self.x.round() as usize] = term::fg(BRAND.coral, "P");
            }
            lines.push(format!("│{}│", cells.concat()));
        }
        lines.push(format!("└{}┘", "─".repeat(self.width as usize)));
        lines
    }

    pub fn over(&self) -> Option<&str> {
        self.over.as_deref()
    }
}

impl Game for Pong {
    fn tick(&self) -> u64 {
        40
    }
    fn key(&mut self, name: &str) {
        Pong::key(self, name);
    }
    fn step(&mut self) {
        Pong::step(self);
    }
    fn draw(&self) -> Vec<String> {
        Pong::draw(self)
    }
    fn over(&self) -> Option<&str> {
        self.over()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn court(game: &Pong) -> Vec<String> {
        game.draw()[2..12]
            .iter()
            .map(|s| term::strip_ansi(s))
            .collect()
    }
    fn paddle(game: &Pong) -> Vec<usize> {
        court(game)
            .iter()
            .enumerate()
            .filter_map(|(r, line)| (line.chars().nth(2) == Some('█')).then_some(r))
            .collect()
    }
    fn ball(game: &Pong) -> usize {
        court(game)
            .iter()
            .position(|line| line.contains('P'))
            .unwrap()
    }

    #[test]
    fn refuses_a_small_terminal() {
        assert!(
            Pong::new(20, 10, || 0.5)
                .err()
                .unwrap()
                .contains("too small for pong")
        );
    }

    #[test]
    fn keeps_the_paddle_in_the_court() {
        let mut game = Pong::new(50, 13, || 0.5).unwrap();
        for _ in 0..20 {
            game.key("down");
        }
        assert_eq!(paddle(&game), vec![6, 7, 8, 9]);
        for _ in 0..20 {
            game.key("w");
        }
        assert_eq!(paddle(&game), vec![0, 1, 2, 3]);
    }

    #[test]
    fn bounces_the_ball_on_the_top_wall() {
        let mut game = Pong::new(50, 13, || 0.0).unwrap();
        let mut rows = Vec::new();
        for _ in 0..15 {
            game.step();
            rows.push(ball(&game));
        }
        assert!(rows.contains(&0));
        assert!(rows.last().unwrap() > &0);
    }

    #[test]
    fn bounces_the_ball_on_the_paddle() {
        let mut game = Pong::new(50, 13, || 0.5).unwrap();
        for _ in 0..30 {
            game.step();
        }
        assert!(game.draw()[0].contains("You 0 : 0 CPU"));
        assert!(court(&game)[ball(&game)].find('P').unwrap() > 3);
    }

    #[test]
    fn scores_a_miss_and_ends_at_seven() {
        let mut game = Pong::new(50, 13, || 0.5).unwrap();
        for _ in 0..5 {
            game.key("up");
        }
        for _ in 0..25 {
            game.step();
        }
        assert!(game.draw()[0].contains("You 0 : 1 CPU"));
        for _ in 0..1000 {
            if game.over.is_some() {
                break;
            }
            game.step();
        }
        assert_eq!(game.over.as_deref(), Some("You lose 0-7"));
    }
}
