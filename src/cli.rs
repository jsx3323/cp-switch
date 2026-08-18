use clap::{Parser, Subcommand};

use crate::error::CsError;
use crate::store::is_builtin;

#[derive(Parser)]
#[command(name = "cp-switch")]
#[command(about = "切换 Claude Code 项目配置")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 列出所有配置，标记当前活跃
    #[command(visible_aliases = ["ls"])]
    List {
        /// 显示用户级活跃状态（对比 ~/.claude/settings.json）
        #[arg(long, short)]
        user: bool,
    },

    /// 切换到指定配置
    Use {
        /// 配置名称
        name: String,
        /// 写入用户级 settings.json（~/.claude/settings.json）
        #[arg(long, short)]
        user: bool,
    },

    /// 添加一个新的配置
    Add {
        /// 配置名称
        name: String,
        /// 覆盖已存在的配置
        #[arg(long, short)]
        force: bool,
    },

    /// 显示当前活跃配置名称
    #[command(visible_aliases = ["show"])]
    Current {
        /// 读取用户级 current 标记
        #[arg(long, short)]
        user: bool,
    },

    /// 删除指定配置
    #[command(visible_aliases = ["rm"])]
    Delete {
        /// 配置名称
        name: String,
        /// 跳过删除活跃配置的确认
        #[arg(long, short)]
        force: bool,
    },

    /// 查看当前配置与指定配置的差异
    Diff {
        /// 配置名称
        name: String,
        /// 对比用户级 settings.json
        #[arg(long, short)]
        user: bool,
    },

    /// 编辑已有配置（以当前值为默认）
    Edit {
        /// 配置名称
        name: String,
    },

    /// 设置/查看本地使用的 Claude 模型（仅在当前 profile 为 claude 时可用）
    Model {
        /// 模型 id（如 claude-fable-5[1m]）；省略则显示当前值
        value: Option<String>,
        /// 清除 model 字段，回退到 Claude Code 默认
        #[arg(long, conflicts_with = "value")]
        clear: bool,
        /// 操作用户级 settings.json（~/.claude/settings.json）
        #[arg(long, short)]
        user: bool,
    },
}

impl Commands {
    /// 各子命令对 profile 名的要求集中在这张表里：谁校验、谁拒绝保留名。
    /// 在 main 分派前统一调用，命令模块因此不再自带校验。
    pub fn validate(&self) -> Result<(), CsError> {
        match self {
            // 会写 profile：保留名必须拒绝
            Commands::Add { name, .. } | Commands::Edit { name } | Commands::Delete { name, .. } => {
                validate_profile_arg(name)
            }
            // 只是引用 profile：内置 claude 是合法目标
            Commands::Use { name, .. } | Commands::Diff { name, .. } => validate_name(name),
            Commands::List { .. } | Commands::Current { .. } | Commands::Model { .. } => Ok(()),
        }
    }
}

pub fn validate_name(name: &str) -> Result<(), CsError> {
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err(CsError::InvalidProfileName { name: name.into() });
    }
    Ok(())
}

pub fn ensure_not_reserved(name: &str) -> Result<(), CsError> {
    if is_builtin(name) {
        return Err(CsError::ReservedProfileName { name: name.into() });
    }
    Ok(())
}

/// add / edit / delete 接受的 profile 名：既要合法，也不能占用内置名
pub fn validate_profile_arg(name: &str) -> Result<(), CsError> {
    validate_name(name)?;
    ensure_not_reserved(name)
}