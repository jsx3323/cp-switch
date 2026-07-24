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

/// settings.local.json 顶层字段：Claude Code 原生模型选择器（区别于 env 里的 ANTHROPIC_MODEL）
pub const MODEL_FIELD: &str = "model";

/// 精确白名单：仅这 11 个 key 被 cp-switch 管理
const MANAGED_KEYS: &[&str] = &[
    KEY_BASE_URL,
    KEY_API_KEY,
    KEY_AUTH_TOKEN,
    KEY_MODEL,
    KEY_SMALL_FAST_MODEL,
    KEY_DEFAULT_HAIKU,
    KEY_DEFAULT_SONNET,
    KEY_DEFAULT_OPUS,
    KEY_SUBAGENT_MODEL,
    KEY_EFFORT_LEVEL,
    KEY_AUTO_COMPACT_WINDOW,
];

pub fn is_claude_env_key(key: &str) -> bool {
    MANAGED_KEYS.iter().any(|managed| *managed == key)
}

/// 判断是否为内置 profile 名称
pub fn is_builtin(name: &str) -> bool {
    name == CLAUDE_PROFILE_NAME
}

pub fn derive_default_models(model: &str) -> [(String, String); 4] {
    [
        (KEY_SMALL_FAST_MODEL.into(), model.into()),
        (KEY_DEFAULT_HAIKU.into(), model.into()),
        (KEY_DEFAULT_SONNET.into(), model.into()),
        (KEY_DEFAULT_OPUS.into(), model.into()),
    ]
}
