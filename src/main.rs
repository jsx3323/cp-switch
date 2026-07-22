use clap::Parser;
use cp_switch::cli::{Cli, Commands};
use cp_switch::command;
use cp_switch::error::CsError;
use cp_switch::input;
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
        Commands::List => {
            let project = store::find_project_dir()?;
            command::list::run(&project)
        }
        Commands::Use { name } => {
            let project = store::find_project_dir()?;
            ensure_claude_dir(&project)?;
            command::use_profile::run(&name, &project)
        }
        Commands::Add { name, force } => command::add::run(&name, force),
        Commands::Current => {
            let project = store::find_project_dir()?;
            command::current::run(&project)
        }
        Commands::Delete { name, force } => {
            let project = store::find_project_dir()?;
            command::delete::run(&name, force, &project)
        }
        Commands::Diff { name } => {
            let project = store::find_project_dir()?;
            ensure_claude_dir(&project)?;
            command::diff::run(&name, &project)
        }
        Commands::Edit { name } => command::edit::run(&name),
    }
}

fn ensure_claude_dir(project: &std::path::Path) -> Result<(), CsError> {
    if store::has_claude_dir(project) {
        return Ok(());
    }
    output::warn("当前目录没有 .claude 目录");
    if !input::prompt_confirm("是否新建 .claude/settings.local.json？")? {
        return Err(CsError::NoClaudeDir);
    }
    Ok(())
}
