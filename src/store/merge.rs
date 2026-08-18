use serde_json::Value;
use crate::error::CsError;
use super::keys::{is_claude_env_key, MODEL_FIELD};

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

    // removed 暂含所有被清除的受管理 key；剔除 profile 重新写入的，只保留真正移除的
    removed.retain(|k| !profile_env.contains_key(k));

    Ok((settings, written, removed))
}

/// 清除 settings env 中所有受管理的 ANTHROPIC_* / CLAUDE_CODE_* key。
/// 返回 (新的 settings, 被移除的 key 列表)。
pub fn clear_env(mut settings: Value) -> Result<(Value, Vec<String>), CsError> {
    let obj = settings.as_object_mut()
        .ok_or(CsError::MalformedJson { detail: "settings must be a JSON object".into() })?;
    // 纯移除操作：env 本就不存在时保持不存在，不凭空写出空字段污染用户手写的 settings
    let Some(settings_env) = obj.get_mut("env") else {
        return Ok((settings, Vec::new()));
    };
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
    let now_empty = env_obj.is_empty();

    // 清空后连字段一起删掉，与「env 缺失时不创建」保持一致
    if now_empty {
        obj.remove("env");
    }

    Ok((settings, removed))
}

/// 设置 settings 顶层 model 字段（纯函数，不读写文件）。
pub fn set_model(mut settings: Value, model: &str) -> Result<Value, CsError> {
    let obj = settings.as_object_mut()
        .ok_or(CsError::MalformedJson { detail: "settings must be a JSON object".into() })?;
    obj.insert(MODEL_FIELD.to_string(), Value::String(model.to_string()));
    Ok(settings)
}

/// 移除 settings 顶层 model 字段，返回 (新的 settings, 是否原本存在)。
pub fn clear_model(mut settings: Value) -> Result<(Value, bool), CsError> {
    let obj = settings.as_object_mut()
        .ok_or(CsError::MalformedJson { detail: "settings must be a JSON object".into() })?;
    let existed = obj.remove(MODEL_FIELD).is_some();
    Ok((settings, existed))
}