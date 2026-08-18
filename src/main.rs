use clap::Parser;
use cp_switch::cli::{Cli, Commands};
use cp_switch::command;
use cp_switch::error::CsError;
use cp_switch::output;
use cp_switch::store::{Scope, find_project_dir};

fn main() {
    if let Err(e) = run(Cli::parse()) {
        output::error(&e.to_string());
        if let Some(h) = e.hint() {
            output::hint(&h);
        }
        std::process::exit(e.exit_code());
    }
}

fn run(cli: Cli) -> Result<(), CsError> {
    cli.command.validate()?;
    match cli.command {
        Commands::List { user } => command::list::run(&Scope::from_flag(user)?),
        Commands::Use { name, user } => command::use_profile::run(&name, &Scope::from_flag(user)?),
        Commands::Add { name, force } => command::add::run(&name, force),
        Commands::Current { user } => command::current::run(&Scope::from_flag(user)?),
        // delete 同时清理两个层级的活跃标记，因此总要知道项目目录
        Commands::Delete { name, force } => command::delete::run(&name, force, &find_project_dir()?),
        Commands::Diff { name, user } => command::diff::run(&name, &Scope::from_flag(user)?),
        Commands::Edit { name } => command::edit::run(&name),
        Commands::Copy { src, dst, force } => command::copy::run(&src, &dst, force),
        Commands::Rename { src, dst, force } => command::rename::run(&src, &dst, force),
        Commands::Model { value, clear, user } => {
            command::model::run(value, clear, &Scope::from_flag(user)?)
        }
    }
}
