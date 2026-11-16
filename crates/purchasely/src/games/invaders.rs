//! Owner: P11b
use super::term::{self, BRAND, Game};

const ROWS: usize = 5;
const NAME: &str = "PURCHASELY";
#[derive(Clone, Copy)]
struct Spot {
    x: usize,
    y: usize,
}
#[derive(Clone, Copy)]
struct Invader {
    spot: Spot,
    row: usize,
    letter: char,
}
pub struct Invaders {
    width: usize,
    height: usize,
    ship: usize,
    score: u32,
    lives: u8,
    wave: u32,
    direction: i32,
    clock: usize,
    frame: usize,
    army: Vec<Invader>,
    shots: Vec<Spot>,
    bombs: Vec<Spot>,
    over: Option<String>,
    random: Box<dyn FnMut() -> f64>,
}
impl Invaders {
    pub fn new(
        cols: usize,
        rows: usize,
        random: impl FnMut() -> f64 + 'static,
    ) -> Result<Self, String> {
        let width = cols.saturating_sub(2).min(50);
        let height = rows.saturating_sub(2).min(22);
        if width < 24 || height < 12 {
            return Err(format!(
                "The terminal is too small for invaders: {cols}x{rows}, it needs 26x14."
            ));
        }
        let mut game = Self {
            width,
            height,
            ship: width >> 1,
            score: 0,
            lives: 3,
            wave: 0,
            direction: 1,
            clock: 0,
            frame: 0,
            army: Vec::new(),
            shots: Vec::new(),
            bombs: Vec::new(),
            over: None,
            random: Box::new(random),
        };
        game.next_wave();
        Ok(game)
    }
    fn next_wave(&mut self) {
        self.wave += 1;
        self.army.clear();
        for row in 0..ROWS {
            for (i, letter) in NAME.chars().enumerate() {
                self.army.push(Invader {
                    spot: Spot {
                        x: i * 2,
                        y: row + 1,
                    },
                    row,
                    letter,
                });
            }
        }
        self.shots.clear();
        self.bombs.clear();
        self.direction = 1;
    }
    fn hit(&mut self) {
        let mut shots = Vec::new();
        for shot in self.shots.drain(..) {
            if let Some(i) = self
                .army
                .iter()
                .position(|a| a.spot.x == shot.x && a.spot.y == shot.y)
            {
                let invader = self.army.remove(i);
                self.score += ((ROWS - invader.row) * 10) as u32;
            } else {
                shots.push(shot);
            }
        }
        self.shots = shots;
    }
    pub fn over(&self) -> Option<&str> {
        self.over.as_deref()
    }
    pub fn draw(&self) -> Vec<String> {
        let colors = [
            BRAND.coral,
            BRAND.lavender,
            BRAND.cream,
            BRAND.light_green,
            BRAND.sage,
        ];
        let mut field = vec![vec![" ".to_owned(); self.width]; self.height];
        for invader in &self.army {
            field[invader.spot.y][invader.spot.x] =
                term::fg(colors[invader.row], &invader.letter.to_string());
        }
        for shot in &self.shots {
            if shot.y < self.height {
                field[shot.y][shot.x] = "|".into();
            }
        }
        for bomb in &self.bombs {
            if bomb.y < self.height {
                field[bomb.y][bomb.x] = term::fg(BRAND.coral, "!");
            }
        }
        field[self.height - 1][self.ship] = term::fg(BRAND.light_green, "▲");
        let mut lines = vec![format!(
            "{}   Score {}   Lives {}   Wave {}   q to quit",
            term::fg(BRAND.lavender, "Purchasely Invaders"),
            self.score,
            self.lives,
            self.wave
        )];
        lines.extend(field.into_iter().map(|row| format!("│{}│", row.concat())));
        lines.push(format!("└{}┘", "─".repeat(self.width)));
        lines
    }
}
impl Game for Invaders {
    fn tick(&self) -> u64 {
        50
    }
    fn key(&mut self, name: &str) {
        match name {
            "left" | "a" => self.ship = self.ship.saturating_sub(1),
            "right" | "d" => self.ship = (self.ship + 1).min(self.width - 1),
            "space" if self.shots.len() < 2 => self.shots.push(Spot {
                x: self.ship,
                y: self.height - 2,
            }),
            _ => {}
        }
    }
    fn step(&mut self) {
        self.frame += 1;
        for shot in &mut self.shots {
            shot.y = shot.y.saturating_sub(1);
        }
        self.shots.retain(|s| s.y > 0);
        self.hit();
        let period = (((20i32 - 2 * self.wave as i32).max(6) as usize * self.army.len()) as f64
            / (ROWS * NAME.len()) as f64)
            .round()
            .max(2.0) as usize;
        if !self.army.is_empty() {
            self.clock += 1;
            if self.clock >= period {
                self.clock = 0;
                if self.army.iter().any(|a| {
                    (a.spot.x as i32 + self.direction) < 0
                        || a.spot.x as i32 + self.direction >= self.width as i32
                }) {
                    self.direction = -self.direction;
                    for a in &mut self.army {
                        a.spot.y += 1;
                    }
                } else {
                    for a in &mut self.army {
                        a.spot.x = (a.spot.x as i32 + self.direction) as usize;
                    }
                }
                self.hit();
                if self.army.iter().any(|a| a.spot.y >= self.height - 1) {
                    self.over = Some(format!(
                        "The invaders landed. Game over. Score {}, wave {}.",
                        self.score, self.wave
                    ));
                    return;
                }
            }
        }
        if self.frame.is_multiple_of(2) {
            for bomb in &mut self.bombs {
                bomb.y += 1;
            }
            self.bombs.retain(|b| b.y < self.height);
        }
        if !self.army.is_empty() && (self.random)() < 0.02 + 0.005 * self.wave as f64 {
            let i = ((self.random)() * self.army.len() as f64) as usize;
            let invader = self.army[i.min(self.army.len() - 1)];
            self.bombs.push(Spot {
                x: invader.spot.x,
                y: invader.spot.y + 1,
            });
        }
        if self
            .bombs
            .iter()
            .any(|b| b.x == self.ship && b.y == self.height - 1)
        {
            self.bombs.clear();
            self.lives -= 1;
            if self.lives == 0 {
                self.over = Some(format!(
                    "Game over. Score {}, wave {}.",
                    self.score, self.wave
                ));
            }
        }
        if self.army.is_empty() {
            self.next_wave();
        }
    }
    fn draw(&self) -> Vec<String> {
        Invaders::draw(self)
    }
    fn over(&self) -> Option<&str> {
        self.over()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::term::{self, Game};

    fn plain(game: &impl Game) -> Vec<String> {
        game.draw()
            .iter()
            .map(|line| term::strip_ansi(line))
            .collect()
    }
    fn score(game: &impl Game) -> u32 {
        let screen = plain(game);
        screen[0]
            .split("Score ")
            .nth(1)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .parse()
            .unwrap()
    }
    fn count(game: &impl Game) -> usize {
        plain(game)
            .iter()
            .skip(1)
            .flat_map(|line| line.chars())
            .filter(|c| "PURCHASELY".contains(*c))
            .count()
    }
    fn ship_line(game: &impl Game) -> String {
        plain(game)
            .into_iter()
            .find(|line| line.contains('▲'))
            .unwrap()
    }

    #[test]
    fn refuses_a_small_terminal() {
        assert!(
            Invaders::new(20, 10, || 0.99)
                .err()
                .unwrap()
                .contains("too small for invaders")
        );
    }
    #[test]
    fn keeps_the_ship_in_the_field() {
        let mut game = Invaders::new(40, 24, || 0.99).unwrap();
        for _ in 0..60 {
            game.key("left");
        }
        assert!(ship_line(&game).starts_with("│▲"));
        for _ in 0..60 {
            game.key("d");
        }
        assert!(ship_line(&game).ends_with("▲│"));
    }
    #[test]
    fn kills_an_invader_with_a_shot_and_scores() {
        let mut game = Invaders::new(40, 24, || 0.99).unwrap();
        for _ in 0..300 {
            if score(&game) > 0 {
                break;
            }
            game.key("space");
            game.step();
        }
        assert!(score(&game) > 0);
        assert!(count(&game) < 50);
    }
    #[test]
    fn takes_a_life_when_a_bomb_hits_the_ship() {
        let mut game = Invaders::new(40, 24, || 0.0).unwrap();
        for _ in 0..30 {
            game.key("left");
        }
        for _ in 0..100 {
            if plain(&game)[0].contains("Lives 2") {
                break;
            }
            game.step();
        }
        assert!(plain(&game)[0].contains("Lives 2"));
    }
    #[test]
    fn ends_the_game_when_the_invaders_land() {
        let mut game = Invaders::new(40, 24, || 0.99).unwrap();
        for _ in 0..100_000 {
            if game.over().is_some() {
                break;
            }
            game.step();
        }
        assert_eq!(
            game.over(),
            Some("The invaders landed. Game over. Score 0, wave 1.")
        );
    }
    #[test]
    fn starts_a_new_wave_when_the_last_invader_dies() {
        let mut game = Invaders::new(40, 24, || 0.99).unwrap();
        for _ in 0..100_000 {
            if game.over().is_some() || plain(&game)[0].contains("Wave 2") {
                break;
            }
            let lines = plain(&game);
            let target = lines
                .iter()
                .skip(1)
                .rfind(|line| line.chars().any(|c| "PURCHASELY".contains(c)))
                .unwrap()
                .chars()
                .position(|c| "PURCHASELY".contains(c))
                .unwrap();
            let ship = ship_line(&game).chars().position(|c| c == '▲').unwrap();
            game.key(if target < ship {
                "left"
            } else if target > ship {
                "right"
            } else {
                "space"
            });
            game.step();
        }
        assert_eq!(game.over(), None);
        assert!(plain(&game)[0].contains("Wave 2"));
        assert_eq!(count(&game), 50);
    }
}
