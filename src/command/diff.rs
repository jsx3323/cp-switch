use crate::error::{CsError, serialization_err};
use crate::output;
use crate::store::{Scope, read_profile};

pub fn run(name: &str, scope: &Scope) -> Result<(), CsError> {
    let profile_env = read_profile(name)?;
    // 只读命令：settings 缺失时按空 env 比较，不去问用户要不要新建它不会创建的文件
    let current_env = scope.read_current_env()?;

    let label = match scope {
        Scope::Project(_) => "current env",
        Scope::User => "user settings",
    };

    // Value 相等即无差异，无差异时省掉两次 pretty-print
    if current_env == profile_env {
        output::info(&format!("No differences between {} and profile '{}'", label, name));
        return Ok(());
    }

    let current_json = serde_json::to_string_pretty(&current_env)
        .map_err(|e| serialization_err(label, e))?;
    let profile_json = serde_json::to_string_pretty(&profile_env)
        .map_err(|e| serialization_err(name, e))?;

    output::render_diff(label, &format!("profile: {}", name), &current_json, &profile_json);
    Ok(())
}
