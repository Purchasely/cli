//! Owner: P11a
use super::term::{self, BRAND, Game};

type Point = (i32, i32);
pub struct Snake {
    width: i32,
    height: i32,
    body: Vec<Point>,
    direction: Point,
    next: Point,
    score: u32,
    food: Point,
    over: Option<String>,
    random: Box<dyn FnMut() -> f64>,
}

impl Snake {
    pub fn new(
        cols: usize,
        rows: usize,
        random: impl FnMut() -> f64 + 'static,
    ) -> Result<Self, String> {
        let width = 30.min(cols.saturating_sub(2) / 2) as i32;
        let height = 20.min(rows.saturating_sub(3)) as i32;
        if width < 10 || height < 5 {
            return Err(format!(
                "The terminal is too small for snake: it needs 22x8, it has {cols}x{rows}."
            ));
        }
        let x = width >> 1;
        let y = height >> 1;
        let body = vec![(x, y), (x - 1, y), (x - 2, y)];
        let mut snake = Self {
            width,
            height,
            body,
            direction: (1, 0),
            next: (1, 0),
            score: 0,
            food: (0, 0),
            over: None,
            random: Box::new(random),
        };
        snake.food = snake.place().unwrap_or((0, 0));
        Ok(snake)
    }

    fn place(&mut self) -> Option<Point> {
        let free = (0..self.height)
            .flat_map(|y| (0..self.width).map(move |x| (x, y)))
            .filter(|point| !self.body.contains(point))
            .collect::<Vec<_>>();
        if free.is_empty() {
            None
        } else {
            Some(free[((self.random)() * free.len() as f64) as usize % free.len()])
        }
    }

    pub fn key(&mut self, name: &str) {
        let next = match name {
            "up" | "w" => (0, -1),
            "down" | "s" => (0, 1),
            "left" | "a" => (-1, 0),
            "right" | "d" => (1, 0),
            _ => return,
        };
        if next.0 + self.direction.0 != 0 || next.1 + self.direction.1 != 0 {
            self.next = next;
        }
    }

    pub fn step(&mut self) {
        self.direction = self.next;
        let head = (
            self.body[0].0 + self.direction.0,
            self.body[0].1 + self.direction.1,
        );
        let eat = head == self.food;
        if !eat {
            self.body.pop();
        }
        if head.0 < 0
            || head.1 < 0
            || head.0 >= self.width
            || head.1 >= self.height
            || self.body.contains(&head)
        {
            self.over = Some(format!("Game over. Score: {}.", self.score));
            return;
        }
        self.body.insert(0, head);
        if eat {
            self.score += 1;
            if let Some(food) = self.place() {
                self.food = food;
            } else {
                self.over = Some(format!("You win! Score: {}.", self.score));
            }
        }
    }

    pub fn draw(&self) -> Vec<String> {
        let border = "─".repeat(self.width as usize * 2);
        let mut lines = vec![format!("┌{border}┐")];
        for y in 0..self.height {
            let mut line = String::new();
            for x in 0..self.width {
                if let Some(index) = self.body.iter().position(|point| *point == (x, y)) {
                    line.push_str(&term::fg(
                        if index == 0 {
                            BRAND.lavender
                        } else {
                            BRAND.light_green
                        },
                        "██",
                    ));
                } else if self.food == (x, y) {
                    line.push_str(&term::bg(BRAND.coral, &term::fg(BRAND.dark_green, "P ")));
                } else {
                    line.push_str("  ");
                }
            }
            lines.push(format!("│{line}│"));
        }
        lines.push(format!("└{border}┘"));
        lines.push(format!(
            "{}   Score: {}   q to quit",
            term::fg(BRAND.lavender, "Purchasely Snake"),
            self.score
        ));
        lines
    }

    pub fn over(&self) -> Option<&str> {
        self.over.as_deref()
    }
}

impl Game for Snake {
    fn tick(&self) -> u64 {
        120
    }
    fn key(&mut self, name: &str) {
        Snake::key(self, name);
    }
    fn step(&mut self) {
        Snake::step(self);
    }
    fn draw(&self) -> Vec<String> {
        Snake::draw(self)
    }
    fn over(&self) -> Option<&str> {
        self.over()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> Snake {
        Snake::new(22, 8, || 0.0).unwrap()
    }
    fn screen(game: &Snake) -> String {
        term::strip_ansi(&game.draw().join("\n"))
    }

    #[test]
    fn moves_one_cell_per_step_and_dies_on_the_wall() {
        let mut game = game();
        for _ in 0..4 {
            game.step();
        }
        assert_eq!(game.over, None);
        game.step();
        assert_eq!(game.over.as_deref(), Some("Game over. Score: 0."));
    }

    #[test]
    fn eats_grows_and_scores() {
        let mut game = game();
        game.key("w");
        for _ in 0..2 {
            game.step();
        }
        game.key("left");
        for _ in 0..5 {
            game.step();
        }
        assert_eq!(game.over, None);
        assert!(screen(&game).contains("Score: 1"));
        assert_eq!(screen(&game).matches("██").count(), 4);
    }

    #[test]
    fn does_not_reverse_onto_itself() {
        let mut game = game();
        game.key("left");
        game.step();
        assert_eq!(game.over, None);
        game.key("up");
        game.key("left");
        game.step();
        assert_eq!(game.over, None);
    }

    #[test]
    fn draws_a_border_board_and_status_line() {
        assert_eq!(screen(&game()).lines().count(), 8);
        assert!(screen(&game()).contains("Purchasely Snake   Score: 0   q to quit"));
    }

    #[test]
    fn refuses_a_small_terminal() {
        assert!(
            Snake::new(21, 8, || 0.0)
                .err()
                .unwrap()
                .contains("too small for snake")
        );
    }
}
