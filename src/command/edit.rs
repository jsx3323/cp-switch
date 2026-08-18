use crate::cli::validate_profile_arg;
use crate::command::prompt::prompt_profile_env;
use crate::error::CsError;
use crate::output;
use crate::store::{is_claude_env_key, read_profile, save_profile};

pub fn run(name: &str) -> Result<(), CsError> {
    validate_profile_arg(name)?;

    let existing = read_profile(name)?;
    let env = existing
        .as_object()
        .ok_or(CsError::MalformedJson { detail: "profile must be a JSON object".into() })?;

    let mut new_env = prompt_profile_env(Some(env))?;

    // 保留非标准 key（用户手动加的 non-managed key）。
    // managed key 一律不恢复：prompt 已重新收集当前应有的 managed key，
    // 恢复反而会残留 prompt 不再输出的旧 key（如从 API_KEY 迁移到 AUTH_TOKEN 时的旧 API_KEY）。
    for (key, value) in env.iter() {
        if is_claude_env_key(key) {
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
