use serde_json::{Map, Value};

use super::keys::{ENV_FIELD, MODEL_FIELD, is_claude_env_key};
use crate::error::CsError;

fn as_object_mut<'a>(value: &'a mut Value, what: &str) -> Result<&'a mut Map<String, Value>, CsError> {
    value.as_object_mut().ok_or_else(|| CsError::MalformedJson {
        detail: format!("{} must be a JSON object", what),
    })
}

fn as_object<'a>(value: &'a Value, what: &str) -> Result<&'a Map<String, Value>, CsError> {
    value.as_object().ok_or_else(|| CsError::MalformedJson {
        detail: format!("{} must be a JSON object", what),
    })
}

/// 就地删除 env 中所有受管理 key，返回被删除的 key 列表
fn take_managed(env_obj: &mut Map<String, Value>) -> Vec<String> {
    let mut removed = Vec::new();
    env_obj.retain(|k, _| {
        if is_claude_env_key(k) {
            removed.push(k.clone());
            false
        } else {
            true
        }
    });
    removed
}

/// 用 profile 的 env 替换 settings 中的受管理 key，返回 (新的 settings, 写入的 key, 真正移除的 key)。
///
/// `env_values` 为空对象时等价于纯清除（`clear_env`）：受管理 key 全部移除，
/// env 因此变空则连字段一起删掉，不凭空写出 `"env": {}` 污染用户手写的 settings。
pub fn merge_env(
    mut settings: Value,
    env_values: &Value,
) -> Result<(Value, Vec<String>, Vec<String>), CsError> {
    let profile_env = as_object(env_values, "env_values")?;

    let obj = as_object_mut(&mut settings, "settings")?;
    // 纯清除且本就没有 env：保持不存在，只有确实要写入值时才建出该字段
    if profile_env.is_empty() && !obj.contains_key(ENV_FIELD) {
        return Ok((settings, Vec::new(), Vec::new()));
    }

    let env_slot = obj.entry(ENV_FIELD).or_insert_with(|| Value::Object(Map::new()));
    let env_obj = as_object_mut(env_slot, "\"env\" field")?;

    let mut removed = take_managed(env_obj);
    for (key, value) in profile_env {
        env_obj.insert(key.clone(), value.clone());
    }
    if env_obj.is_empty() {
        obj.remove(ENV_FIELD);
    }

    // removed 暂含所有被清除的受管理 key；剔除 profile 重新写入的，只保留真正移除的
    removed.retain(|k| !profile_env.contains_key(k));

    Ok((settings, profile_env.keys().cloned().collect(), removed))
}

/// 清除 settings env 中所有受管理的 ANTHROPIC_* / CLAUDE_CODE_* key。
/// 返回 (新的 settings, 被移除的 key 列表)。
pub fn clear_env(settings: Value) -> Result<(Value, Vec<String>), CsError> {
    let (settings, _, removed) = merge_env(settings, &Value::Object(Map::new()))?;
    Ok((settings, removed))
}

/// 从 settings 中提取受管理的 env key，缺失或空的 env 得到空对象。
pub fn managed_env(settings: &Value) -> Result<Value, CsError> {
    let Some(env) = settings.get(ENV_FIELD) else {
        return Ok(Value::Object(Map::new()));
    };
    let env_obj = as_object(env, "\"env\" field")?;
    Ok(Value::Object(
        env_obj
            .iter()
            .filter(|(k, _)| is_claude_env_key(k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    ))
}

/// profile 的每个 key 在当前 env 中是否都有相同的值（`merge_env` 的逆向检查）
pub fn is_env_applied(current_env: &Value, profile_env: &Value) -> bool {
    let empty = Map::new();
    let current_obj = current_env.as_object().unwrap_or(&empty);
    let profile_obj = profile_env.as_object().unwrap_or(&empty);
    profile_obj.iter().all(|(k, v)| current_obj.get(k) == Some(v))
}

/// 读取 settings 顶层 model 字段
pub fn get_model(settings: &Value) -> Option<&str> {
    settings.get(MODEL_FIELD).and_then(|m| m.as_str())
}

/// 设置 settings 顶层 model 字段（纯函数，不读写文件）。
pub fn set_model(mut settings: Value, model: &str) -> Result<Value, CsError> {
    as_object_mut(&mut settings, "settings")?
        .insert(MODEL_FIELD.to_string(), Value::String(model.to_string()));
    Ok(settings)
}

/// 移除 settings 顶层 model 字段，返回 (新的 settings, 是否原本存在)。
pub fn clear_model(mut settings: Value) -> Result<(Value, bool), CsError> {
    let existed = as_object_mut(&mut settings, "settings")?.remove(MODEL_FIELD).is_some();
    Ok((settings, existed))
}
