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
}

pub fn validate_name(name: &str) -> Result<(), CsError> {
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err(CsError::InvalidProfileName { name: name.into() });
    }
    Ok(())
}

pub fn ensure_not_reserved(name: &str) -> Result<(), CsError> {
    if is_builtin(name) {
        return Err(CsError::InvalidProfileName { name: name.into() });
    }
    Ok(())
}