use serde_json::{Map, Value};

use crate::error::CsError;
use crate::input;
use crate::output;
use crate::store::{
    derive_default_models,
    KEY_AUTH_TOKEN, KEY_AUTO_COMPACT_WINDOW, KEY_BASE_URL, KEY_EFFORT_LEVEL,
    KEY_MODEL, KEY_SMALL_FAST_MODEL, KEY_SUBAGENT_MODEL,
};

const EFFORT_DEFAULT: &str = "high";

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

    // 必填字段：新增模式必须给值，编辑模式空输入沿用旧值
    let required = |key: &str| -> Result<String, CsError> {
        match existing {
            Some(_) => input::prompt_with_default(key, get_str(key).unwrap_or("")),
            None => input::prompt_required(key),
        }
    };

    let mut env = Map::new();
    env.insert(KEY_BASE_URL.into(), Value::String(required(KEY_BASE_URL)?));
    env.insert(KEY_AUTH_TOKEN.into(), Value::String(required(KEY_AUTH_TOKEN)?));
    let model = required(KEY_MODEL)?;
    env.insert(KEY_MODEL.into(), Value::String(model.clone()));

    // --- 4 个推导模型 key（SMALL_FAST / HAIKU / SONNET / OPUS）---
    for (key, derived) in derive_default_models(&model) {
        let existing_val = get_str(key);
        let fallback = existing_val.unwrap_or(derived);
        let val = match input::prompt_optional(key, fallback)? {
            Some(val) => val,
            None => {
                if existing_val.is_none() {
                    output::info(&format!("  → auto-derived: {}", derived));
                }
                fallback.to_string()
            }
        };
        env.insert(key.into(), Value::String(val));
    }

    // --- SUBAGENT_MODEL：继承新 env 中的 SMALL_FAST_MODEL ---
    let small_fast = env
        .get(KEY_SMALL_FAST_MODEL)
        .and_then(|v| v.as_str())
        .unwrap_or(&model)
        .to_string();
    let subagent_existing = get_str(KEY_SUBAGENT_MODEL);
    let subagent_default = subagent_existing.unwrap_or(&small_fast);
    let subagent_val = match input::prompt_optional(KEY_SUBAGENT_MODEL, subagent_default)? {
        Some(val) => val,
        None => {
            if subagent_existing.is_none() {
                output::info(&format!(
                    "  → inherited from {}: {}",
                    KEY_SMALL_FAST_MODEL, small_fast
                ));
            }
            subagent_default.to_string()
        }
    };
    env.insert(KEY_SUBAGENT_MODEL.into(), Value::String(subagent_val));

    // --- EFFORT_LEVEL：默认 "high" ---
    let effort_default = get_str(KEY_EFFORT_LEVEL).unwrap_or(EFFORT_DEFAULT);
    let effort_val = input::prompt_optional(KEY_EFFORT_LEVEL, effort_default)?
        .unwrap_or_else(|| effort_default.to_string());
    env.insert(KEY_EFFORT_LEVEL.into(), Value::String(effort_val));

    // --- AUTO_COMPACT_WINDOW：真正可选（跳过则不注入 / 清空）---
    if let Some(val) =
        input::prompt_optional(KEY_AUTO_COMPACT_WINDOW, get_str(KEY_AUTO_COMPACT_WINDOW).unwrap_or(""))?
    {
        env.insert(KEY_AUTO_COMPACT_WINDOW.into(), Value::String(val));
    }

    Ok(env)
}
