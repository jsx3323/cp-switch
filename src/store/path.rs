use std::env;
use std::path::{Path, PathBuf};

use crate::error::{CsError, io_err};

fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
}

pub(crate) fn store_dir() -> PathBuf {
    match env::var("CP_SWITCH_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => home_dir().join(".cp-switch"),
    }
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
    home_dir().join(".claude").join("settings.json")
}
