use std::path::{Path, PathBuf};
use std::fs;

use serde_json::Value;
use crate::error::{CsError, io_err, json_err, serialization_err};
use super::state::State;
use super::path::{settings_local_path, user_settings_path, state_path};
use super::keys::is_claude_env_key;

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

fn read_state() -> Result<State, CsError> {
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

/// 从旧版文件夹格式迁移到 state.json
fn try_migrate() -> Result<State, CsError> {
    let dir = state_path().parent().unwrap().to_path_buf();
    let mut state = State::default();

    // 迁移 profiles
    let profiles_dir = dir.join("profiles");
    if let Ok(entries) = fs::read_dir(&profiles_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(value) = serde_json::from_str(&content) {
                            state.profiles.insert(name.to_string(), value);
                        }
                    }
                }
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
    let _ = fs::remove_dir_all(&dir.join("projects"));
    let _ = fs::remove_dir_all(&profiles_dir);
    let _ = fs::remove_file(&uc_path);

    Ok(state)
}

// ── profiles ──

pub fn list_profiles() -> Result<Vec<String>, CsError> {
    let state = read_state()?;
    let mut names: Vec<String> = state.profiles.into_keys().collect();
    names.sort();
    Ok(names)
}

pub fn read_profile(name: &str) -> Result<Value, CsError> {
    let state = read_state()?;
    match state.profiles.get(name) {
        Some(v) => Ok(v.clone()),
        None => {
            let available = state.profiles.into_keys().collect();
            Err(CsError::ProfileNotFound { name: name.into(), available })
        }
    }
}

pub fn save_profile(name: &str, content: &Value) -> Result<(), CsError> {
    let mut state = read_state()?;
    state.profiles.insert(name.to_string(), content.clone());
    write_state(&state)
}

pub fn delete_profile(name: &str) -> Result<(), CsError> {
    let mut state = read_state()?;
    if state.profiles.remove(name).is_none() {
        let available: Vec<String> = state.profiles.into_keys().collect();
        return Err(CsError::ProfileNotFound { name: name.into(), available });
    }
    write_state(&state)
}

// ── 项目级 current ──

fn current_key(project: &Path) -> String {
    project.to_string_lossy().to_string()
}

pub fn read_current(project: &Path) -> Result<Option<String>, CsError> {
    let state = read_state()?;
    Ok(state.project_currents.get(&current_key(project)).cloned())
}

pub fn write_current(project: &Path, name: &str) -> Result<(), CsError> {
    let mut state = read_state()?;
    state.project_currents.insert(current_key(project), name.to_string());
    write_state(&state)
}

pub fn clear_current(project: &Path) -> Result<(), CsError> {
    let mut state = read_state()?;
    state.project_currents.remove(&current_key(project));
    write_state(&state)
}

// ── 用户级 current ──

pub fn read_user_current() -> Result<Option<String>, CsError> {
    let state = read_state()?;
    Ok(state.user_current)
}

pub fn write_user_current(name: &str) -> Result<(), CsError> {
    let mut state = read_state()?;
    state.user_current = Some(name.to_string());
    write_state(&state)
}

pub fn clear_user_current() -> Result<(), CsError> {
    let mut state = read_state()?;
    state.user_current = None;
    write_state(&state)
}

// ── settings 本地读写（不变 ──

pub fn read_settings_local(project: &Path) -> Result<Value, CsError> {
    let path = settings_local_path(project);
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).map_err(|e| json_err(&path, e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(serde_json::json!({"permissions": {"allow": [], "deny": []}, "env": {}}))
        }
        Err(e) => Err(io_err(&path, e)),
    }
}

pub fn write_settings_local(project: &Path, content: &Value) -> Result<(), CsError> {
    let path = settings_local_path(project);

    let bak = sibling_path(&path, ".bak");
    match fs::copy(&path, &bak) {
        Ok(_) => {},
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
        Err(e) => return Err(io_err(&bak, e)),
    }

    let json = serde_json::to_string_pretty(content).map_err(|e| serialization_err(&path.display().to_string(), e))?;
    atomic_write(&path, &json)
}

pub fn read_current_env(project: &Path) -> Result<Value, CsError> {
    let settings = read_settings_local(project)?;
    filter_claude_env(&settings)
}

fn filter_claude_env(settings: &Value) -> Result<Value, CsError> {
    let env = settings.get("env").cloned().unwrap_or(Value::Object(serde_json::Map::new()));
    let env_obj = env.as_object()
        .ok_or(CsError::MalformedJson { detail: "\"env\" field must be a JSON object".into() })?;
    let filtered: serde_json::Map<String, Value> = env_obj
        .iter()
        .filter(|(k, _)| is_claude_env_key(k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    Ok(Value::Object(filtered))
}

// ── 用户级 settings ──

pub fn read_user_settings() -> Result<Value, CsError> {
    let path = user_settings_path();
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).map_err(|e| json_err(&path, e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(serde_json::json!({"permissions": {"allow": [], "deny": []}, "env": {}}))
        }
        Err(e) => Err(io_err(&path, e)),
    }
}

pub fn write_user_settings(content: &Value) -> Result<(), CsError> {
    let path = user_settings_path();

    let bak = sibling_path(&path, ".bak");
    match fs::copy(&path, &bak) {
        Ok(_) => {},
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
        Err(e) => return Err(io_err(&bak, e)),
    }

    let json = serde_json::to_string_pretty(content).map_err(|e| serialization_err(&path.display().to_string(), e))?;
    atomic_write(&path, &json)
}

pub fn read_user_current_env() -> Result<Value, CsError> {
    let settings = read_user_settings()?;
    filter_claude_env(&settings)
}
