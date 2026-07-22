use serde_json::Value;
use crate::error::CsError;
use super::keys::is_claude_env_key;

pub fn merge_env(mut settings: Value, env_values: &Value) -> Result<(Value, Vec<String>, Vec<String>), CsError> {
    let profile_env = env_values.as_object()
        .ok_or(CsError::MalformedJson { detail: "env_values must be a JSON object".into() })?;

    let settings_env = settings
        .as_object_mut()
        .ok_or(CsError::MalformedJson { detail: "settings must be a JSON object".into() })?
        .entry("env")
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    let env_obj = settings_env.as_object_mut()
        .ok_or(CsError::MalformedJson { detail: "\"env\" field must be a JSON object".into() })?;

    // 先清除所有受管理的 key，再写入 profile 的 key
    let mut removed = Vec::new();
    env_obj.retain(|k, _| {
        if is_claude_env_key(k) {
            removed.push(k.clone());
            false
        } else {
            true
        }
    });

    let mut written = Vec::new();
    for (key, value) in profile_env {
        env_obj.insert(key.clone(), value.clone());
        written.push(key.clone());
    }

    Ok((settings, written, removed))
}

/// 清除 settings env 中所有受管理的 ANTHROPIC_* / CLAUDE_CODE_* key。
/// 返回 (新的 settings, 被移除的 key 列表)。
pub fn clear_env(mut settings: Value) -> Result<(Value, Vec<String>), CsError> {
    let settings_env = settings
        .as_object_mut()
        .ok_or(CsError::MalformedJson { detail: "settings must be a JSON object".into() })?
        .entry("env")
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    let env_obj = settings_env.as_object_mut()
        .ok_or(CsError::MalformedJson { detail: "\"env\" field must be a JSON object".into() })?;

    let mut removed = Vec::new();
    env_obj.retain(|k, _| {
        if is_claude_env_key(k) {
            removed.push(k.clone());
            false
        } else {
            true
        }
    });

    Ok((settings, removed))
}