use std::path::Path;

use crate::error::CsError;
use crate::output;
use crate::store::{read_current, read_user_current, CLAUDE_PROFILE_NAME};

pub fn run(project: &Path) -> Result<(), CsError> {
    let current = read_current(project)?;

    match current {
        Some(name) if name == CLAUDE_PROFILE_NAME => {
            output::success("Current profile: claude (default Claude provider)");
        }
        Some(name) => output::success(&format!("Current profile: {}", name)),
        None => output::info("No active profile (settings.local.json is not managed by cp-switch)"),
    }
    Ok(())
}

pub fn run_user() -> Result<(), CsError> {
    let current = read_user_current()?;

    match current {
        Some(name) if name == CLAUDE_PROFILE_NAME => {
            output::success("Current profile (user): claude (default Claude provider)");
        }
        Some(name) => output::success(&format!("Current profile (user): {}", name)),
        None => output::info("No active user-level profile"),
    }
    Ok(())
}
