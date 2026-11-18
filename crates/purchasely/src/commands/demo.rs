use crate::{
    cli::Command,
    core::{
        demo::DEMO_APP,
        json::Value,
        output::{self, EmitOptions},
        runtime::{Runtime, add_globals},
    },
};

const HELP: &str = "\nDemo mode answers every read from the built-in sample data of one fictional app (app_demo).\nIt needs no key and sends no request. The data is not real. A write exits 10; --dry-run and --plan still work.\nEach run prints demo=on in the stderr header line.\n\nExample:\n  export PURCHASELY_DEMO=1\n  purchasely dashboard summary --app app_demo --output table\n  unset PURCHASELY_DEMO";

pub fn register(program: &mut Command, rt: &mut Runtime) {
    let name = rt.d.name.clone();
    rt.taken.insert("demo".into());
    let mut command = Command::new("demo")
        .description("Try the CLI with no key: sample data of a fictional app, for a first test");
    add_globals(&mut command);
    command = command.after_help(HELP.replace("purchasely", &name));
    program.commands.push(command.action(move |_, rt| {
        let value = Value::Object(vec![
            ("demo".into(), Value::from(format!("Demo mode answers every read from the sample data of one fictional app ({DEMO_APP}). No key, no request. The data is not real. A write exits 10."))),
            ("start".into(), Value::from("export PURCHASELY_DEMO=1".into())),
            ("app".into(), Value::from(DEMO_APP.into())),
            ("try".into(), Value::Array(vec![
                Value::from(format!("{} applications list", name)),
                Value::from(format!("{} placements list --app {DEMO_APP}", name)),
                Value::from(format!("{} dashboard summary --app {DEMO_APP} --output table", name)),
            ])),
            ("stop".into(), Value::from("unset PURCHASELY_DEMO".into())),
        ]);
        output::emit(&rt.d, &value, &EmitOptions {
            env: "prod".into(),
            key: None,
            name: "demo".into(),
            ..EmitOptions::default()
        })?;
        Ok(())
    }));
}
