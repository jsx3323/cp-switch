use super::scope_label;
use crate::error::CsError;
use crate::output;
use crate::store::{Scope, is_builtin};

pub fn run(scope: &Scope) -> Result<(), CsError> {
    match scope.read_current()? {
        Some(name) => {
            let note = if is_builtin(&name) { " (default Claude provider)" } else { "" };
            output::success(&format!("Current profile{}: {}{}", scope_label(scope), name, note));
        }
        None => output::info(match scope {
            Scope::Project(_) => "No active profile (settings.local.json is not managed by cp-switch)",
            Scope::User => "No active user-level profile",
        }),
    }
    Ok(())
}
