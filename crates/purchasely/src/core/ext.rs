use crate::{cli::Command, core::{deps::Deps, runtime::Runtime}};

pub struct Program {
    pub name: &'static str,
    pub version: &'static str,
    pub host: &'static str,
    pub ext: Option<Extension>,
}
pub struct Extension {
    pub modules: Vec<Register>,
    pub surface: &'static str,
    pub surfaces: Vec<(&'static str, &'static str)>,
    pub surface_defs: Vec<SurfaceDef>,
    pub identity: bool,
    pub section: (&'static str, &'static str),
    pub latest: Option<fn(&Deps) -> Result<Option<String>, CliError>>,
    pub npm_update: Option<&'static str>,
}
pub type Register = fn(&mut Command, &Runtime);
pub struct SurfaceDef;
#[derive(Debug)]
pub struct CliError;
