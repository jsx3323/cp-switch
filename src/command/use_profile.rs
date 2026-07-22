use std::path::Path;

use crate::cli::validate_name;
use crate::error::CsError;
use crate::input;
use crate::output;
use crate::store::{read_profile, merge_env, write_current, read_settings_local, write_settings_local, has_claude_dir,
                   read_user_settings, write_user_settings, write_user_current};

pub fn run(name: &str, project: &Path) -> Result<(), CsError> {
    validate_name(name)?;

    if !has_claude_dir(project) {
        output::warn("当前目录没有 .claude 目录");
        if !input::prompt_confirm("是否新建 .claude/settings.local.json？")? {
            return Err(CsError::NoClaudeDir);
        }
    }

    let env_values = read_profile(name)?;
    let settings = read_settings_local(project)?;
    let (merged, changed, removed) = merge_env(settings, &env_values)?;
    write_settings_local(project, &merged)?;
    write_current(project, name)?;

    output::success(&format!("Switched to profile '{}'", name));
    for key in &changed {
        output::info(&format!("  {} = {}", key, env_values.get(key).unwrap()));
    }
    for key in &removed {
        output::removed(key);
    }
    Ok(())
}

pub fn run_user(name: &str) -> Result<(), CsError> {
    validate_name(name)?;

    let env_values = read_profile(name)?;
    let settings = read_user_settings()?;
    let (merged, changed, removed) = merge_env(settings, &env_values)?;
    write_user_settings(&merged)?;
    write_user_current(name)?;

    output::success(&format!("Switched to profile '{}' (user)", name));
    for key in &changed {
        output::info(&format!("  {} = {}", key, env_values.get(key).unwrap()));
    }
    for key in &removed {
        output::removed(key);
    }
    Ok(())
}