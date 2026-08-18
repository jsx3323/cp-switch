use std::path::Path;

use crate::error::CsError;
use crate::input;
use crate::output;
use crate::store::{delete_profile_and_clear, read_currents};

pub fn run(name: &str, force: bool, project: &Path) -> Result<(), CsError> {
    // delete 同时影响两个层级，一次读取取回两边的活跃标记
    let (project_current, user_current) = read_currents(project)?;
    let is_project_active = project_current.as_deref() == Some(name);
    let is_user_active = user_current.as_deref() == Some(name);

    if (is_project_active || is_user_active) && !force {
        output::info(&format!("Profile '{}' is currently active.", name));
        output::info("Deleting will remove the profile but leave current settings unchanged.");
        if !input::prompt_confirm("Continue?")? {
            output::info("Cancelled.");
            return Ok(());
        }
    }

    delete_profile_and_clear(
        name,
        if is_project_active { Some(project) } else { None },
        is_user_active,
    )?;

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
