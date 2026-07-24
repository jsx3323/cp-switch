use std::path::Path;

use serde_json::Value;
use crate::error::CsError;
use crate::output;
use crate::store::{read_current, read_user_current, is_builtin,
                   read_settings_local, write_settings_local,
                   read_user_settings, write_user_settings,
                   set_model, clear_model, MODEL_FIELD};

pub fn run(value: Option<String>, clear: bool, project: &Path) -> Result<(), CsError> {
    ensure_claude(read_current(project)?)?;
    let settings = read_settings_local(project)?;
    let (new, dirty) = apply(settings, value, clear, "")?;
    if dirty {
        write_settings_local(project, &new)?;
    }
    Ok(())
}

pub fn run_user(value: Option<String>, clear: bool) -> Result<(), CsError> {
    ensure_claude(read_user_current()?)?;
    let settings = read_user_settings()?;
    let (new, dirty) = apply(settings, value, clear, " (user)")?;
    if dirty {
        write_user_settings(&new)?;
    }
    Ok(())
}

/// 门控：仅当前 profile 为内置 claude（官方直连）时放行
fn ensure_claude(current: Option<String>) -> Result<(), CsError> {
    match current {
        Some(name) if is_builtin(&name) => Ok(()),
        _ => Err(CsError::ModelRequiresClaude),
    }
}

/// 执行 model 操作，返回 (可能修改后的 settings, 是否需要写回)。label 用于区分项目/用户级输出。
fn apply(settings: Value, value: Option<String>, clear: bool, label: &str) -> Result<(Value, bool), CsError> {
    if clear {
        let (new, existed) = clear_model(settings)?;
        if existed {
            output::success(&format!("Cleared model{}", label));
        } else {
            output::info("  (no model field to clear)");
        }
        return Ok((new, existed));
    }

    match value {
        Some(v) => {
            let v = v.trim();
            if v.is_empty() {
                return Err(CsError::InvalidModel);
            }
            let new = set_model(settings, v)?;
            output::success(&format!("Model set to '{}'{}", v, label));
            Ok((new, true))
        }
        None => {
            match settings.get(MODEL_FIELD).and_then(|m| m.as_str()) {
                Some(m) => output::success(&format!("Current model{}: {}", label, m)),
                None => output::info(&format!("No model set{} (using Claude Code default)", label)),
            }
            Ok((settings, false))
        }
    }
}
