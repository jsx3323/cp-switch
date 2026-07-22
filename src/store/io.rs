use std::path::{Path, PathBuf};
use std::fs;

use serde_json::Value;
use crate::error::{CsError, io_err, json_err, serialization_err};
use super::path::{profiles_dir, profile_path, project_current_path, settings_local_path,
                   user_settings_path, user_current_path};
use super::keys::is_claude_env_key;

fn sibling_path(path: &Path, suffix: &str) -> PathBuf {
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    path.with_file_name(format!("{}{}", file_name, suffix))
}

fn atomic_write(path: &Path, content: &str) -> Result<(), CsError> {
    let dir = path.parent().unwrap();
    fs::create_dir_all(dir).map_err(|e| io_err(dir, e))?;
    let tmp = sibling_path(path, ".tmp");
    fs::write(&tmp, content).map_err(|e| io_err(&tmp, e))?;
    fs::rename(&tmp, path).map_err(|e| io_err(path, e))?;
    Ok(())
}

pub fn list_profiles() -> Result<Vec<String>, CsError> {
    let dir = profiles_dir();
    let mut names = Vec::new();
    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(names),
        Err(e) => return Err(io_err(&dir, e)),
    };
    for entry in entries {
        let entry = entry.map_err(|e| io_err(&dir, e))?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json")
            && let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                names.push(name.to_string());
            }
    }
    names.sort();
    Ok(names)
}

pub fn read_current(project: &Path) -> Result<Option<String>, CsError> {
    let path = project_current_path(project);
    match fs::read_to_string(&path) {
        Ok(content) => Ok(Some(content.trim().to_string())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_err(&path, e)),
    }
}

pub fn write_current(project: &Path, name: &str) -> Result<(), CsError> {
    atomic_write(&project_current_path(project), name)
}

pub fn clear_current(project: &Path) -> Result<(), CsError> {
    let path = project_current_path(project);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_err(&path, e)),
    }
}

pub fn read_profile(name: &str) -> Result<Value, CsError> {
    let path = profile_path(name);
    let content = fs::read_to_string(&path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            let available = list_profiles().unwrap_or_default();
            CsError::ProfileNotFound { name: name.into(), available }
        } else {
            io_err(&path, e)
        }
    })?;
    serde_json::from_str(&content).map_err(|e| json_err(&path, e))
}

pub fn save_profile(name: &str, content: &Value) -> Result<(), CsError> {
    let dir = profiles_dir();
    fs::create_dir_all(&dir).map_err(|e| io_err(&dir, e))?;
    let path = profile_path(name);
    let json = serde_json::to_string_pretty(content).map_err(|e| serialization_err(&path.display().to_string(), e))?;
    atomic_write(&path, &json)
}

pub fn delete_profile(name: &str) -> Result<(), CsError> {
    let path = profile_path(name);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let available = list_profiles().unwrap_or_default();
            Err(CsError::ProfileNotFound { name: name.into(), available })
        }
        Err(e) => Err(io_err(&path, e)),
    }
}

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

    // 先备份，再序列化，避免序列化后备份失败浪费
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

// ── 用户级 current 标记 ──

pub fn read_user_current() -> Result<Option<String>, CsError> {
    let path = user_current_path();
    match fs::read_to_string(&path) {
        Ok(content) => Ok(Some(content.trim().to_string())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_err(&path, e)),
    }
}

pub fn write_user_current(name: &str) -> Result<(), CsError> {
    atomic_write(&user_current_path(), name)
}

pub fn clear_user_current() -> Result<(), CsError> {
    let path = user_current_path();
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_err(&path, e)),
    }
}