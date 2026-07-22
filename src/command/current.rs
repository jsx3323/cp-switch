use std::path::Path;

use crate::error::CsError;
use crate::output;
use crate::store::read_current;

pub fn run(project: &Path) -> Result<(), CsError> {
    let current = read_current(project)?;

    match current {
        Some(name) => output::success(&format!("Current profile: {}", name)),
        None => output::info("No active profile (settings.local.json is not managed by cp-switch)"),
    }
    Ok(())
}
