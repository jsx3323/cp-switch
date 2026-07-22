pub const KEY_BASE_URL: &str = "ANTHROPIC_BASE_URL";
pub const KEY_API_KEY: &str = "ANTHROPIC_API_KEY";
pub const KEY_AUTH_TOKEN: &str = "ANTHROPIC_AUTH_TOKEN";
pub const KEY_MODEL: &str = "ANTHROPIC_MODEL";
pub const KEY_SMALL_FAST_MODEL: &str = "ANTHROPIC_SMALL_FAST_MODEL";
pub const KEY_DEFAULT_HAIKU: &str = "ANTHROPIC_DEFAULT_HAIKU_MODEL";
pub const KEY_DEFAULT_SONNET: &str = "ANTHROPIC_DEFAULT_SONNET_MODEL";
pub const KEY_DEFAULT_OPUS: &str = "ANTHROPIC_DEFAULT_OPUS_MODEL";
pub const KEY_SUBAGENT_MODEL: &str = "CLAUDE_CODE_SUBAGENT_MODEL";
pub const KEY_EFFORT_LEVEL: &str = "CLAUDE_CODE_EFFORT_LEVEL";
pub const KEY_AUTO_COMPACT_WINDOW: &str = "CLAUDE_CODE_AUTO_COMPACT_WINDOW";

pub const CLAUDE_PROFILE_NAME: &str = "claude";

/// 所有需要被 profile 管理的 key 前缀
const MANAGED_PREFIXES: &[&str] = &["ANTHROPIC_", "CLAUDE_CODE_"];

/// 互斥 key 组：组内任一 key 出现在 profile 时，settings 中组内其他 key 应被清除
pub const CONFLICT_GROUPS: &[&[&str]] = &[
    &[KEY_API_KEY, KEY_AUTH_TOKEN],
];

pub fn is_claude_env_key(key: &str) -> bool {
    MANAGED_PREFIXES.iter().any(|prefix| key.starts_with(prefix))
}

/// 返回 settings 中应被清除的冲突 key（组内不在 profile 中的 key）
pub(crate) fn conflicting_keys(profile_env: &serde_json::Map<String, serde_json::Value>) -> Vec<&str> {
    let mut result = Vec::new();
    for group in CONFLICT_GROUPS.iter() {
        if group.iter().any(|k| profile_env.contains_key(*k)) {
            for key in group.iter() {
                if !profile_env.contains_key(*key) {
                    result.push(*key);
                }
            }
        }
    }
    result
}

pub fn derive_default_models(model: &str) -> [(String, String); 4] {
    [
        (KEY_SMALL_FAST_MODEL.into(), model.into()),
        (KEY_DEFAULT_HAIKU.into(), model.into()),
        (KEY_DEFAULT_SONNET.into(), model.into()),
        (KEY_DEFAULT_OPUS.into(), model.into()),
    ]
}
