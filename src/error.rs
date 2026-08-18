use thiserror::Error;

#[derive(Error, Debug)]
pub enum CsError {
    #[error("Profile '{name}' not found. Available: {available:?}")]
    ProfileNotFound {
        name: String,
        available: Vec<String>,
    },

    #[error("Profile '{name}' already exists. Use --force to overwrite.")]
    ProfileExists { name: String },

    #[error("No active profile for this project")]
    NoActiveProfile,

    #[error("Invalid profile name '{name}'. Use only letters, digits, hyphens, and underscores.")]
    InvalidProfileName { name: String },

    #[error("Profile name '{name}' is reserved by cp-switch.")]
    ReservedProfileName { name: String },

    #[error("I/O error at {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },

    #[error("Invalid JSON in {path}: {source}")]
    Json {
        path: String,
        source: serde_json::Error,
    },

    #[error("Malformed JSON: {detail}")]
    MalformedJson { detail: String },

    #[error("Serialization error in {context}: {source}")]
    Serialization {
        context: String,
        source: serde_json::Error,
    },

    #[error("当前目录没有 .claude 目录")]
    NoClaudeDir,

    #[error("model 命令仅在当前 profile 为 claude（官方直连）时可用")]
    ModelRequiresClaude,

    #[error("Invalid model value (must not be empty)")]
    InvalidModel,

    #[error("Input ended before '{field}' was provided")]
    MissingInput { field: String },
}

impl CsError {
    pub fn exit_code(&self) -> i32 {
        match self {
            CsError::ProfileNotFound { .. } => 1,
            CsError::ProfileExists { .. } => 2,
            CsError::NoActiveProfile => 4,
            // 与 InvalidProfileName 同码：对调用方都是「这个名字不能用」，不为文案细分新增契约
            CsError::InvalidProfileName { .. } | CsError::ReservedProfileName { .. } => 5,
            CsError::Io { .. } => 6,
            CsError::Json { .. } => 7,
            CsError::MalformedJson { .. } => 8,
            CsError::Serialization { .. } => 9,
            CsError::NoClaudeDir => 10,
            CsError::ModelRequiresClaude => 11,
            CsError::InvalidModel => 12,
            CsError::MissingInput { .. } => 13,
        }
    }

    pub fn hint(&self) -> Option<String> {
        match self {
            CsError::NoActiveProfile => {
                Some("Use 'cp-switch add <name>' to create and activate a profile".into())
            }
            CsError::MalformedJson { .. } => {
                Some("Check your .claude/settings.local.json for manual edits that broke the structure".into())
            }
            CsError::NoClaudeDir => {
                Some("需要新建 .claude/settings.local.json 才能使用此命令".into())
            }
            CsError::ReservedProfileName { .. } => {
                Some("'claude' 是内置的官方直连配置，无需创建；直接运行 'cp-switch use claude'".into())
            }
            CsError::ModelRequiresClaude => {
                Some("先运行 'cp-switch use claude' 切换到官方直连，再设置 model".into())
            }
            CsError::MissingInput { .. } => {
                Some("非交互运行时需要把全部必填字段一并喂给 stdin".into())
            }
            _ => None,
        }
    }
}

pub fn io_err(path: impl AsRef<std::path::Path>, source: std::io::Error) -> CsError {
    CsError::Io { path: path.as_ref().display().to_string(), source }
}

pub fn json_err(path: impl AsRef<std::path::Path>, source: serde_json::Error) -> CsError {
    CsError::Json { path: path.as_ref().display().to_string(), source }
}

pub fn serialization_err(context: &str, source: serde_json::Error) -> CsError {
    CsError::Serialization { context: context.into(), source }
}