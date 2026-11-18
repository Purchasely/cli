//! Owner: P7b
//!
//! How a lot registers a command:
//! - Fill `pub fn register(program: &mut Command, rt: &mut Runtime)` in the module file (the `Register` type of
//!   `core/ext.rs`). `run.rs` already calls it in the TS `MODULES` order: do not edit `run.rs`.
//! - Reserve each hand-written name with `rt.taken.insert("resource action")` before Management registers.
//! - Give each leaf `.action(|p, rt| { ... })`: `p` is the `cli::Parsed` (`p.opt("env")`, `p.flag("dry-run")`,
//!   `p.arguments`), `rt` the `Runtime`. A closure may capture data (an operation id).
//! - Return `Err(core::errors::CliError)`: `run` writes it as one JSON line on stderr and exits with its code.
//! - `run` already did the preAction (env, `--output`, credential and the stderr status line), except for the
//!   offline tops of `run::OFFLINE`. A group without an action prints its help.
pub mod ai;
pub mod api;
pub mod apply;
pub mod auth;
pub mod completion;
pub mod dashboard;
pub mod demo;
pub mod doctor;
pub mod games;
pub mod reference;
pub mod search;
pub mod update;
pub mod usage;
