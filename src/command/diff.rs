use std::path::Path;

use crate::error::{CsError, serialization_err};
use crate::input;
use crate::output;
use crate::store::{read_current_env, read_profile, read_user_current_env, has_claude_dir};
use similar::{ChangeTag, TextDiff};

pub fn run(name: &str, project: &Path) -> Result<(), CsError> {
    if !has_claude_dir(project) {
        output::warn("当前目录没有 .claude 目录");
        if !input::prompt_confirm("是否新建 .claude/settings.local.json？")? {
            return Err(CsError::NoClaudeDir);
        }
    }

    let current_env = read_current_env(project)?;
    let profile_env = read_profile(name)?;

    let current_json = serde_json::to_string_pretty(&current_env)
        .map_err(|e| serialization_err("current env", e))?;
    let profile_json = serde_json::to_string_pretty(&profile_env)
        .map_err(|e| serialization_err(name, e))?;

    if current_json == profile_json {
        output::info(&format!("No differences between current env and profile '{}'", name));
        return Ok(());
    }

    output::diff_header("current env", &format!("profile: {}", name));

    let diff = TextDiff::from_lines(&current_json, &profile_json);
    for change in diff.iter_all_changes() {
        let line = change.to_string_lossy();
        match change.tag() {
            ChangeTag::Delete => output::diff_deleted(&line),
            ChangeTag::Insert => output::diff_inserted(&line),
            ChangeTag::Equal => output::diff_equal(&line),
        }
    }
    Ok(())
}

pub fn run_user(name: &str) -> Result<(), CsError> {
    let current_env = read_user_current_env()?;
    let profile_env = read_profile(name)?;

    let current_json = serde_json::to_string_pretty(&current_env)
        .map_err(|e| serialization_err("user settings env", e))?;
    let profile_json = serde_json::to_string_pretty(&profile_env)
        .map_err(|e| serialization_err(name, e))?;

    if current_json == profile_json {
        output::info(&format!("No differences between user env and profile '{}'", name));
        return Ok(());
    }

    output::diff_header("user settings", &format!("profile: {}", name));

    let diff = TextDiff::from_lines(&current_json, &profile_json);
    for change in diff.iter_all_changes() {
        let line = change.to_string_lossy();
        match change.tag() {
            ChangeTag::Delete => output::diff_deleted(&line),
            ChangeTag::Insert => output::diff_inserted(&line),
            ChangeTag::Equal => output::diff_equal(&line),
        }
    }
    Ok(())
}