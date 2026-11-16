//! Owner: P5a
use crate::Program;

pub fn build(_program: Program) {}
pub fn run(_argv: Vec<String>, _deps: crate::Deps) -> i32 {
    0
}
pub fn main(program: Program) {
    build(program);
}
