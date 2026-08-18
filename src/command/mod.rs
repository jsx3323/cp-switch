pub mod add;
pub mod copy;
pub mod current;
pub mod delete;
pub mod diff;
pub mod edit;
pub mod list;
pub mod model;
pub mod prompt;
pub mod use_profile;

use crate::error::CsError;
use crate::input;
use crate::output;
use crate::store::{Scope, has_claude_dir};

/// 输出里区分项目级与用户级的后缀
pub(crate) fn scope_label(scope: &Scope) -> &'static str {
    match scope {
        Scope::Project(_) => "",
        Scope::User => " (user)",
    }
}

/// 项目无 .claude 目录时先征得同意，避免在非项目目录里凭空建出 settings 文件。
/// 用户级 settings 位于 home 下，无需确认。
pub(crate) fn ensure_claude_dir(scope: &Scope) -> Result<(), CsError> {
    let Some(project) = scope.project() else {
        return Ok(());
    };
    if has_claude_dir(project) {
        return Ok(());
    }
    output::warn("当前目录没有 .claude 目录");
    if !input::prompt_confirm("是否新建 .claude/settings.local.json？")? {
        return Err(CsError::NoClaudeDir);
    }
    Ok(())
}
