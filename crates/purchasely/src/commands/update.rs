//! Owner: P5b
use crate::{
    cli::{Command, OptionDef, Parsed},
    core::{ai, errors::CliError, runtime::Runtime, update::run_update},
};

fn update(p: &Parsed, rt: &mut Runtime) -> Result<(), CliError> {
    let background = p.flag("background");
    let g = crate::management::exec::Globals::from(p);
    if background {
        let _ = run_update(rt, true, &g);
        let _ = ai::update_plugins(rt, true);
    } else {
        run_update(rt, false, &g)?;
        ai::update_plugins(rt, false)?;
    }
    Ok(())
}

pub fn register(program: &mut Command, rt: &mut Runtime) {
    rt.taken.insert("update".into());
    let mut command = Command::new("update").description(
        "Check for a new version and install it (managed install) or print the upgrade command",
    );
    crate::core::runtime::add_globals(&mut command);
    command
        .options
        .push(OptionDef::boolean("--background", "").hide_help());
    command = command.after_help("\nA managed install installs the new version. Another install prints the upgrade command. See docs/install.md.\n");
    program.commands.push(command.action(update));
}
