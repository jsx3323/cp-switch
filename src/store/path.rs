use std::path::{Path, PathBuf};
use std::env;

use crate::error::{CsError, io_err};

pub(crate) fn store_dir() -> PathBuf {
    std::env::var("CP_SWITCH_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/"))
                .join(".cp-switch")
        })
}

pub(crate) fn state_path() -> PathBuf {
    store_dir().join("state.json")
}

pub(crate) fn settings_local_path(project: &Path) -> PathBuf {
    project.join(".claude").join("settings.local.json")
}

pub fn find_project_dir() -> Result<PathBuf, CsError> {
    env::current_dir().map_err(|e| io_err("current_dir", e))
}

pub fn has_claude_dir(project: &Path) -> bool {
    project.join(".claude").exists()
}

pub fn user_settings_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/"))
        .join(".claude")
        .join("settings.json")
}
