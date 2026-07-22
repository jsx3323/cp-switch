use crate::cli::{validate_name, ensure_not_reserved};
use crate::command::prompt::prompt_profile_env;
use crate::error::CsError;
use crate::output;
use crate::store::{read_profile, save_profile, KEY_AUTO_COMPACT_WINDOW};

pub fn run(name: &str) -> Result<(), CsError> {
    validate_name(name)?;
    ensure_not_reserved(name)?;

    let existing = read_profile(name)?;
    let env = existing
        .as_object()
        .ok_or(CsError::MalformedJson { detail: "profile must be a JSON object".into() })?;

    let mut new_env = prompt_profile_env(Some(env))?;

    // 保留非标准 key（AUTO_COMPACT_WINDOW 已在 prompt_profile_env 中处理，不恢复）
    for (key, value) in env.iter() {
        if key == KEY_AUTO_COMPACT_WINDOW {
            continue;
        }
        if !new_env.contains_key(key) {
            new_env.insert(key.clone(), value.clone());
        }
    }

    save_profile(name, &serde_json::Value::Object(new_env))?;
    output::success(&format!("Updated profile '{}'", name));
    Ok(())
}
