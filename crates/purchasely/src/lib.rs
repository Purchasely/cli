pub mod cli;
pub mod commands;
pub mod core;
pub mod games;
pub mod management;
pub mod run;

pub use core::deps::{Deps, Host, Request};
pub use core::ext::{Extension, Program, Register};

pub fn main(program: Program) {
    run::main(program);
}
