use serde_json::{Map, Value};

use super::{ensure_claude_dir, scope_label};
use crate::error::CsError;
use crate::output;
use crate::store::{Scope, is_builtin, merge_env, read_profile};

pub fn run(name: &str, scope: &Scope) -> Result<(), CsError> {
    // 内置 claude 是「受管 env 为空」的退化 profile：走同一条 read→merge→write，只清不写
    let builtin = is_builtin(name);
    let env_values = if builtin {
        Value::Object(Map::new())
    } else {
        // 名字合法性已在 CLI 边界校验；先确认 profile 存在，再去动文件系统或打断用户
        read_profile(name)?
    };

    ensure_claude_dir(scope)?;
    let (merged, removed) = merge_env(scope.read_settings()?, &env_values)?;
    scope.write_settings(&merged)?;
    scope.write_current(name)?;

    if builtin {
        output::success("Switched to default Claude provider");
    } else {
        output::success(&format!("Switched to profile '{}'{}", name, scope_label(scope)));
    }
    if let Some(written) = env_values.as_object() {
        for (key, value) in written {
            output::written(key, value);
        }
    }
    for key in &removed {
        output::removed(key);
    }
    if builtin && removed.is_empty() {
        output::info("  (no managed env vars to clear)");
    }
    Ok(())
}
