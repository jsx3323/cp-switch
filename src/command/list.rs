use std::path::Path;

use crate::error::CsError;
use crate::output::{self, ListStatus};
use crate::store::{list_profiles, read_current, read_current_env, read_profile,
                   read_user_current, read_user_current_env, CLAUDE_PROFILE_NAME};

/// 检查 profile 的每个 key 在当前 env 中是否都有相同的值
fn is_profile_synced(current_env: &serde_json::Value, profile_env: &serde_json::Value) -> bool {
    let empty = serde_json::Map::new();
    let current_obj = current_env.as_object().unwrap_or(&empty);
    let profile_obj = profile_env.as_object().unwrap_or(&empty);
    profile_obj.iter().all(|(k, v)| current_obj.get(k) == Some(v))
}

pub fn run(project: &Path) -> Result<(), CsError> {
    let profiles = list_profiles()?;
    let current = read_current(project)?;

    if profiles.is_empty() && current.is_none() {
        output::info("No profiles found. Use 'cp-switch add <name>' to create one.");
        return Ok(());
    }

    output::info(&format!("Profiles for {}:", project.display()));

    for name in &profiles {
        let is_active = current.as_ref() == Some(name);
        if is_active {
            let current_env = read_current_env(project)?;
            let profile_env = read_profile(name)?;
            if is_profile_synced(&current_env, &profile_env) {
                output::list_item(name, &ListStatus::Active);
            } else {
                output::list_item(name, &ListStatus::Outdated);
            }
        } else {
            output::list_item(name, &ListStatus::Inactive);
        }
    }

    // claude 内置 profile
    if current.as_deref() == Some(CLAUDE_PROFILE_NAME) && !profiles.contains(&CLAUDE_PROFILE_NAME.to_string()) {
        output::list_item(CLAUDE_PROFILE_NAME, &ListStatus::Active);
    }
    // 活跃 profile 的文件被手动删除（claude 为内置，不显示 missing）
    if let Some(active) = &current
        && !profiles.contains(active)
        && active != CLAUDE_PROFILE_NAME {
            output::list_item(active, &ListStatus::Missing);
    }

    let active_count = if current.is_some() { 1 } else { 0 };
    output::info(&format!("{} profiles, {} active", profiles.len(), active_count));
    Ok(())
}

pub fn run_user() -> Result<(), CsError> {
    let profiles = list_profiles()?;
    let current = read_user_current()?;

    if profiles.is_empty() && current.is_none() {
        output::info("No profiles found. Use 'cp-switch add <name>' to create one.");
        return Ok(());
    }

    output::info("Profiles (user):");

    for name in &profiles {
        let is_active = current.as_ref() == Some(name);
        if is_active {
            let current_env = read_user_current_env()?;
            let profile_env = read_profile(name)?;
            if is_profile_synced(&current_env, &profile_env) {
                output::list_item(name, &ListStatus::Active);
            } else {
                output::list_item(name, &ListStatus::Outdated);
            }
        } else {
            output::list_item(name, &ListStatus::Inactive);
        }
    }

    // claude 内置 profile
    if current.as_deref() == Some(CLAUDE_PROFILE_NAME) && !profiles.contains(&CLAUDE_PROFILE_NAME.to_string()) {
        output::list_item(CLAUDE_PROFILE_NAME, &ListStatus::Active);
    }
    // 活跃 profile 的文件被手动删除（claude 为内置，不显示 missing）
    if let Some(active) = &current
        && !profiles.contains(active)
        && active != CLAUDE_PROFILE_NAME {
            output::list_item(active, &ListStatus::Missing);
    }

    let active_count = if current.is_some() { 1 } else { 0 };
    output::info(&format!("{} profiles, {} active", profiles.len(), active_count));
    Ok(())
}
