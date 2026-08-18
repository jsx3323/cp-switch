use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::merge::managed_env;
use super::path::{settings_local_path, state_path, user_settings_path};
use super::state::State;
use crate::error::{CsError, io_err, json_err, serialization_err};

fn sibling_path(path: &Path, suffix: &str) -> PathBuf {
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    path.with_file_name(format!("{}{}", file_name, suffix))
}

pub(crate) fn atomic_write(path: &Path, content: &str) -> Result<(), CsError> {
    let dir = path.parent().unwrap();
    fs::create_dir_all(dir).map_err(|e| io_err(dir, e))?;
    let tmp = sibling_path(path, ".tmp");
    fs::write(&tmp, content).map_err(|e| io_err(&tmp, e))?;
    fs::rename(&tmp, path).map_err(|e| io_err(path, e))?;
    Ok(())
}

// ── state 读写 ──

pub(crate) fn read_state() -> Result<State, CsError> {
    let path = state_path();
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).map_err(|e| json_err(&path, e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let state = try_migrate()?;
            write_state(&state)?;
            Ok(state)
        }
        Err(e) => Err(io_err(&path, e)),
    }
}

fn write_state(state: &State) -> Result<(), CsError> {
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| serialization_err("state", e))?;
    atomic_write(&state_path(), &json)
}

/// read→mutate→write 的公共封装，单次读单次写
pub(crate) fn update_state(mutate: impl FnOnce(&mut State)) -> Result<(), CsError> {
    let mut state = read_state()?;
    mutate(&mut state);
    write_state(&state)
}

/// 从旧版文件夹格式迁移到 state.json
fn try_migrate() -> Result<State, CsError> {
    let dir = state_path().parent().unwrap().to_path_buf();
    let mut state = State::default();

    // 迁移 profiles
    let profiles_dir = dir.join("profiles");
    if let Ok(entries) = fs::read_dir(&profiles_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json")
                && let Some(name) = path.file_stem().and_then(|s| s.to_str())
                && let Ok(content) = fs::read_to_string(&path)
                && let Ok(value) = serde_json::from_str(&content)
            {
                state.profiles.insert(name.to_string(), value);
            }
        }
    }

    // 迁移用户级 current
    let uc_path = dir.join("current");
    if let Ok(content) = fs::read_to_string(&uc_path) {
        let name = content.trim().to_string();
        if !name.is_empty() {
            state.user_current = Some(name);
        }
    }

    // 清理旧文件（忽略错误，state.json 已写好即可）
    let _ = fs::remove_dir_all(dir.join("projects"));
    let _ = fs::remove_dir_all(&profiles_dir);
    let _ = fs::remove_file(&uc_path);

    Ok(state)
}

// ── profiles ──

/// 排序后的 profile 名单，用于列表展示与「可用值」提示（BTreeMap 的迭代序即字典序）
pub(crate) fn sorted_names(state: State) -> Vec<String> {
    state.profiles.into_keys().collect()
}

pub fn list_profiles() -> Result<Vec<String>, CsError> {
    Ok(sorted_names(read_state()?))
}

pub fn profile_exists(name: &str) -> Result<bool, CsError> {
    Ok(read_state()?.profiles.contains_key(name))
}

fn not_found(name: &str, state: State) -> CsError {
    CsError::ProfileNotFound { name: name.into(), available: sorted_names(state) }
}

pub fn read_profile(name: &str) -> Result<Value, CsError> {
    let mut state = read_state()?;
    match state.profiles.remove(name) {
        Some(value) => Ok(value),
        None => Err(not_found(name, state)),
    }
}

pub fn save_profile(name: &str, content: &Value) -> Result<(), CsError> {
    update_state(|state| {
        state.profiles.insert(name.to_string(), content.clone());
    })
}

/// 删除 profile，并按需一并清理项目级 / 用户级活跃标记（单次读单次写）
pub fn delete_profile_and_clear(
    name: &str,
    clear_project: Option<&Path>,
    clear_user: bool,
) -> Result<(), CsError> {
    let mut state = read_state()?;
    if state.profiles.remove(name).is_none() {
        return Err(not_found(name, state));
    }
    if let Some(project) = clear_project {
        state.project_currents.remove(&current_key(project));
    }
    if clear_user {
        state.user_current = None;
    }
    write_state(&state)
}

pub fn delete_profile(name: &str) -> Result<(), CsError> {
    delete_profile_and_clear(name, None, false)
}

// ── current 标记 ──
//
// 「项目级还是用户级」这个区分由 store::scope 统一持有，这里只提供旧的按级别命名的入口。

pub(crate) fn current_key(project: &Path) -> String {
    project.to_string_lossy().to_string()
}

pub fn read_current(project: &Path) -> Result<Option<String>, CsError> {
    Ok(read_state()?.project_currents.remove(&current_key(project)))
}

pub fn write_current(project: &Path, name: &str) -> Result<(), CsError> {
    update_state(|state| {
        state.project_currents.insert(current_key(project), name.to_string());
    })
}

pub fn clear_current(project: &Path) -> Result<(), CsError> {
    update_state(|state| {
        state.project_currents.remove(&current_key(project));
    })
}

/// 一次 state 读取同时取出项目级与用户级活跃 profile（delete 需要同时看两边）
pub fn read_currents(project: &Path) -> Result<(Option<String>, Option<String>), CsError> {
    let mut state = read_state()?;
    Ok((state.project_currents.remove(&current_key(project)), state.user_current))
}

pub fn read_user_current() -> Result<Option<String>, CsError> {
    Ok(read_state()?.user_current)
}

pub fn write_user_current(name: &str) -> Result<(), CsError> {
    update_state(|state| state.user_current = Some(name.to_string()))
}

pub fn clear_user_current() -> Result<(), CsError> {
    update_state(|state| state.user_current = None)
}

// ── settings 读写（项目级与用户级共用同一套逻辑）──

fn default_settings() -> Value {
    serde_json::json!({"permissions": {"allow": [], "deny": []}, "env": {}})
}

pub(crate) fn read_settings_file(path: &Path) -> Result<Value, CsError> {
    match fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).map_err(|e| json_err(path, e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(default_settings()),
        Err(e) => Err(io_err(path, e)),
    }
}

pub(crate) fn write_settings_file(path: &Path, content: &Value) -> Result<(), CsError> {
    let bak = sibling_path(path, ".bak");
    match fs::copy(path, &bak) {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(io_err(&bak, e)),
    }

    let json = serde_json::to_string_pretty(content)
        .map_err(|e| serialization_err(&path.display().to_string(), e))?;
    atomic_write(path, &json)
}

pub fn read_settings_local(project: &Path) -> Result<Value, CsError> {
    read_settings_file(&settings_local_path(project))
}

pub fn write_settings_local(project: &Path, content: &Value) -> Result<(), CsError> {
    write_settings_file(&settings_local_path(project), content)
}

pub fn read_user_settings() -> Result<Value, CsError> {
    read_settings_file(&user_settings_path())
}

pub fn write_user_settings(content: &Value) -> Result<(), CsError> {
    write_settings_file(&user_settings_path(), content)
}

pub fn read_current_env(project: &Path) -> Result<Value, CsError> {
    managed_env(&read_settings_local(project)?)
}

pub fn read_user_current_env() -> Result<Value, CsError> {
    managed_env(&read_user_settings()?)
}
