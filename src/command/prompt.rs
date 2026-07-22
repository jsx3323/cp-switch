use serde_json::{Map, Value};

use crate::error::CsError;
use crate::input;
use crate::output;
use crate::store::{
    derive_default_models,
    KEY_API_KEY, KEY_AUTO_COMPACT_WINDOW, KEY_BASE_URL, KEY_EFFORT_LEVEL,
    KEY_MODEL, KEY_SMALL_FAST_MODEL, KEY_SUBAGENT_MODEL,
};

/// 交互式提示所有 profile 字段。
///
/// `existing` — `Some(map)` 时为编辑模式（用已有值作为默认值），
/// `None` 时为新增模式（纯空默认）。
pub fn prompt_profile_env(
    existing: Option<&Map<String, Value>>,
) -> Result<Map<String, Value>, CsError> {
    let get_str = |key: &str| -> Option<&str> {
        existing
            .and_then(|m| m.get(key))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
    };

    // --- 3 个必填字段：新增模式用 prompt_required，编辑模式用 prompt_with_default ---
    let base_url = if existing.is_some() {
        input::prompt_with_default(KEY_BASE_URL, get_str(KEY_BASE_URL).unwrap_or(""))?
    } else {
        input::prompt_required(KEY_BASE_URL)?
    };
    let api_key = if existing.is_some() {
        input::prompt_with_default(KEY_API_KEY, get_str(KEY_API_KEY).unwrap_or(""))?
    } else {
        input::prompt_required(KEY_API_KEY)?
    };
    let model = if existing.is_some() {
        input::prompt_with_default(KEY_MODEL, get_str(KEY_MODEL).unwrap_or(""))?
    } else {
        input::prompt_required(KEY_MODEL)?
    };

    let mut env = Map::new();
    env.insert(KEY_BASE_URL.into(), Value::String(base_url));
    env.insert(KEY_API_KEY.into(), Value::String(api_key));
    env.insert(KEY_MODEL.into(), Value::String(model.clone()));

    // --- 4 个推导模型 key（SMALL_FAST / HAIKU / SONNET / OPUS）---
    let defaults = derive_default_models(&model);
    for (key, default_val) in &defaults {
        let existing_val = get_str(key);
        let display_default = existing_val.unwrap_or(default_val);
        match input::prompt_optional(key, display_default)? {
            None => {
                let val = existing_val.unwrap_or(default_val).to_string();
                env.insert(key.clone(), Value::String(val));
                if existing_val.is_none() {
                    output::info(&format!("  → auto-derived: {}", default_val));
                }
            }
            Some(val) => {
                env.insert(key.clone(), Value::String(val));
            }
        }
    }

    // --- SUBAGENT_MODEL：继承新 env 中的 SMALL_FAST_MODEL ---
    let small_fast = env
        .get(KEY_SMALL_FAST_MODEL)
        .and_then(|v| v.as_str())
        .unwrap_or(&model)
        .to_string();
    let subagent_default = get_str(KEY_SUBAGENT_MODEL).unwrap_or(&small_fast);
    let subagent_val = match input::prompt_optional(KEY_SUBAGENT_MODEL, subagent_default)? {
        None => {
            let val = get_str(KEY_SUBAGENT_MODEL)
                .unwrap_or(&small_fast)
                .to_string();
            if get_str(KEY_SUBAGENT_MODEL).is_none() {
                output::info(&format!(
                    "  → inherited from {}: {}",
                    KEY_SMALL_FAST_MODEL, small_fast
                ));
            }
            val
        }
        Some(val) => val,
    };
    env.insert(KEY_SUBAGENT_MODEL.into(), Value::String(subagent_val));

    // --- EFFORT_LEVEL：默认 "high" ---
    let effort_default = get_str(KEY_EFFORT_LEVEL).unwrap_or("high");
    let effort_val = match input::prompt_optional(KEY_EFFORT_LEVEL, effort_default)? {
        None => get_str(KEY_EFFORT_LEVEL).unwrap_or("high").to_string(),
        Some(val) => val,
    };
    env.insert(KEY_EFFORT_LEVEL.into(), Value::String(effort_val));

    // --- AUTO_COMPACT_WINDOW：真正可选（跳过则不注入 / 清空）---
    if let Some(val) =
        input::prompt_optional(KEY_AUTO_COMPACT_WINDOW, get_str(KEY_AUTO_COMPACT_WINDOW).unwrap_or(""))?
    {
        env.insert(KEY_AUTO_COMPACT_WINDOW.into(), Value::String(val));
    }

    Ok(env)
}
