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

pub(crate) fn profiles_dir() -> PathBuf {
    store_dir().join("profiles")
}

pub fn profile_path(name: &str) -> PathBuf {
    profiles_dir().join(format!("{}.json", name))
}

pub(crate) fn project_current_path(project: &Path) -> PathBuf {
    let hash = simple_hash(project.to_string_lossy().as_ref());
    store_dir().join("projects").join(hash).join("current")
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

pub(crate) fn user_current_path() -> PathBuf {
    store_dir().join("current")
}

pub(crate) fn simple_hash(s: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis
    for b in s.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3); // FNV prime
    }
    format!("{:016x}", hash)
}