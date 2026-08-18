use serde_json::Value;

use super::{ensure_claude_dir, scope_label};
use crate::error::CsError;
use crate::output;
use crate::store::{Scope, clear_model, get_model, is_builtin, set_model};

pub fn run(value: Option<String>, clear: bool, scope: &Scope) -> Result<(), CsError> {
    // 门控：仅当前 profile 为内置 claude（官方直连）时放行
    match scope.read_current()? {
        Some(name) if is_builtin(&name) => {}
        _ => return Err(CsError::ModelRequiresClaude),
    }

    let (settings, outcome) = apply(scope.read_settings()?, value, clear)?;
    if outcome.needs_write() {
        // 活跃标记存在 state.json，.claude 可能已被删掉；写盘前与 use 一样先确认
        ensure_claude_dir(scope)?;
        scope.write_settings(&settings)?;
    }
    report(&outcome, scope_label(scope));
    Ok(())
}

enum Outcome {
    Set(String),
    Cleared,
    NothingToClear,
    Shown(Option<String>),
}

impl Outcome {
    fn needs_write(&self) -> bool {
        matches!(self, Outcome::Set(_) | Outcome::Cleared)
    }
}

/// 执行 model 操作，返回 (可能修改后的 settings, 结果)。不做输出，写盘成功后才报告。
fn apply(settings: Value, value: Option<String>, clear: bool) -> Result<(Value, Outcome), CsError> {
    if clear {
        let (new, existed) = clear_model(settings)?;
        return Ok((new, if existed { Outcome::Cleared } else { Outcome::NothingToClear }));
    }

    match value {
        Some(v) => {
            let v = v.trim();
            if v.is_empty() {
                return Err(CsError::InvalidModel);
            }
            Ok((set_model(settings, v)?, Outcome::Set(v.to_string())))
        }
        None => {
            let current = get_model(&settings).map(str::to_string);
            Ok((settings, Outcome::Shown(current)))
        }
    }
}

fn report(outcome: &Outcome, label: &str) {
    match outcome {
        Outcome::Set(model) => output::success(&format!("Model set to '{}'{}", model, label)),
        Outcome::Cleared => output::success(&format!("Cleared model{}", label)),
        Outcome::NothingToClear => output::info("  (no model field to clear)"),
        Outcome::Shown(Some(model)) => {
            output::success(&format!("Current model{}: {}", label, model))
        }
        Outcome::Shown(None) => {
            output::info(&format!("No model set{} (using Claude Code default)", label))
        }
    }
}
