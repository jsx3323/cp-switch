use std::path::Path;

use crate::cli::validate_name;
use crate::error::CsError;
use crate::input;
use crate::output;
use crate::store::{delete_profile, read_current, clear_current,
                   read_user_current, clear_user_current};

pub fn run(name: &str, force: bool, project: &Path) -> Result<(), CsError> {
    validate_name(name)?;
    let project_current = read_current(project)?;
    let user_current = read_user_current()?;

    let is_project_active = project_current.as_deref() == Some(name);
    let is_user_active = user_current.as_deref() == Some(name);
    let is_active = is_project_active || is_user_active;

    if is_active && !force {
        output::info(&format!("Profile '{}' is currently active.", name));
        output::info("Deleting will remove the profile but leave current settings unchanged.");
        if !input::prompt_confirm("Continue?")? {
            output::info("Cancelled.");
            return Ok(());
        }
    }

    delete_profile(name)?;

    if is_project_active {
        clear_current(project)?;
    }
    if is_user_active {
        clear_user_current()?;
    }

    match (is_project_active, is_user_active) {
        (true, true) => output::success(&format!(
            "Deleted profile '{}' (was active in both project and user contexts).", name
        )),
        (true, false) => output::success(&format!(
            "Deleted profile '{}' (was active). settings.local.json unchanged.", name
        )),
        (false, true) => output::success(&format!(
            "Deleted profile '{}' (was user-level active). ~/.claude/settings.json unchanged.", name
        )),
        (false, false) => output::success(&format!("Deleted profile '{}'", name)),
    }
    Ok(())
}
