use std::path::{Path, PathBuf};
use std::env;
use std::fs;

use crate::error::{CsError, io_err};

pub(crate) fn store_dir() -> PathBuf {
    let new_dir = std::env::var("CP_SWITCH_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/"))
                .join(".cp-switch")
        });

    // 环境变量覆盖时，不自动迁移
    if std::env::var("CP_SWITCH_DIR").is_err() {
        migrate_old_store(&new_dir);
    }

    new_dir
}

/// 将 ~/.claude-provider-switch 内容迁移到 ~/.cp-switch
fn migrate_old_store(new_dir: &Path) {
    if new_dir.exists() {
        return;
    }
    let old_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/"))
        .join(".claude-provider-switch");
    if !old_dir.exists() {
        return;
    }
    if let Err(e) = copy_dir_recursively(&old_dir, new_dir) {
        eprintln!("cp-switch: migration from {} to {} failed: {}",
            old_dir.display(), new_dir.display(), e);
    } else {
        eprintln!("cp-switch: migrated profiles from {} to {}",
            old_dir.display(), new_dir.display());
    }
}

fn copy_dir_recursively(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursively(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
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

pub(crate) fn simple_hash(s: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis
    for b in s.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3); // FNV prime
    }
    format!("{:016x}", hash)
}