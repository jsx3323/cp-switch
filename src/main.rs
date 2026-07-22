use clap::Parser;
use cp_switch::cli::{Cli, Commands};
use cp_switch::command;
use cp_switch::error::CsError;
use cp_switch::output;
use cp_switch::store;

fn main() {
    let cli = Cli::parse();

    let result = run(cli);

    if let Err(e) = result {
        output::error(&e.to_string());
        if let Some(h) = e.hint() {
            output::hint(&h);
        }
        std::process::exit(e.exit_code());
    }
}

fn run(cli: Cli) -> Result<(), CsError> {
    match cli.command {
        Commands::List { user } => {
            if user {
                command::list::run_user()
            } else {
                let project = store::find_project_dir()?;
                command::list::run(&project)
            }
        }
        Commands::Use { name, user } => {
            if user {
                command::use_profile::run_user(&name)
            } else {
                let project = store::find_project_dir()?;
                command::use_profile::run(&name, &project)
            }
        }
        Commands::Add { name, force } => command::add::run(&name, force),
        Commands::Current { user } => {
            if user {
                command::current::run_user()
            } else {
                let project = store::find_project_dir()?;
                command::current::run(&project)
            }
        }
        Commands::Delete { name, force } => {
            let project = store::find_project_dir()?;
            command::delete::run(&name, force, &project)
        }
        Commands::Diff { name, user } => {
            if user {
                command::diff::run_user(&name)
            } else {
                let project = store::find_project_dir()?;
                command::diff::run(&name, &project)
            }
        }
        Commands::Edit { name } => command::edit::run(&name),
    }
}
