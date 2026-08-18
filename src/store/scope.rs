use std::path::{Path, PathBuf};

use serde_json::Value;

use super::io;
use super::merge::managed_env;
use super::path::{find_project_dir, settings_local_path, user_settings_path};
use super::state::State;
use crate::error::CsError;

/// 命令作用的层级：项目 `.claude/settings.local.json` 或用户 `~/.claude/settings.json`。
///
/// 「用哪个 settings 文件、活跃标记存在 state 的哪个字段」这一份知识只在这里表达一次，
/// 命令层因此不再需要成对的 `run` / `run_user`。
pub enum Scope {
    Project(PathBuf),
    User,
}

/// 一次 state 读取即得的列表信息：全部 profile、活跃 profile 名、该 profile 的 env
pub struct Listing {
    pub profiles: Vec<String>,
    pub current: Option<String>,
    pub active_env: Option<Value>,
}

impl Scope {
    /// `--user` 标记到作用域的唯一转换点，也是 `find_project_dir` 的唯一调用点
    pub fn from_flag(user: bool) -> Result<Scope, CsError> {
        if user {
            Ok(Scope::User)
        } else {
            Ok(Scope::Project(find_project_dir()?))
        }
    }

    /// 项目级作用域对应的项目根目录
    pub fn project(&self) -> Option<&Path> {
        match self {
            Scope::Project(path) => Some(path),
            Scope::User => None,
        }
    }

    fn settings_path(&self) -> PathBuf {
        match self {
            Scope::Project(path) => settings_local_path(path),
            Scope::User => user_settings_path(),
        }
    }

    pub fn read_settings(&self) -> Result<Value, CsError> {
        io::read_settings_file(&self.settings_path())
    }

    pub fn write_settings(&self, content: &Value) -> Result<(), CsError> {
        io::write_settings_file(&self.settings_path(), content)
    }

    /// 当前 settings 中受 cp-switch 管理的 env
    pub fn read_current_env(&self) -> Result<Value, CsError> {
        managed_env(&self.read_settings()?)
    }

    fn take_current(&self, state: &mut State) -> Option<String> {
        match self {
            Scope::Project(path) => state.project_currents.remove(&io::current_key(path)),
            Scope::User => state.user_current.take(),
        }
    }

    pub fn read_current(&self) -> Result<Option<String>, CsError> {
        Ok(self.take_current(&mut io::read_state()?))
    }

    pub fn write_current(&self, name: &str) -> Result<(), CsError> {
        io::update_state(|state| match self {
            Scope::Project(path) => {
                state.project_currents.insert(io::current_key(path), name.to_string());
            }
            Scope::User => state.user_current = Some(name.to_string()),
        })
    }

    pub fn read_listing(&self) -> Result<Listing, CsError> {
        let mut state = io::read_state()?;
        let current = self.take_current(&mut state);
        let active_env = current.as_deref().and_then(|name| state.profiles.get(name).cloned());
        Ok(Listing { profiles: io::sorted_names(state), current, active_env })
    }
}
