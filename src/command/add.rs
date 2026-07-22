use crate::cli::{validate_name, ensure_not_reserved};
use crate::command::prompt::prompt_profile_env;
use crate::error::CsError;
use crate::output;
use crate::store::{save_profile, list_profiles};

pub fn run(name: &str, force: bool) -> Result<(), CsError> {
    validate_name(name)?;
    ensure_not_reserved(name)?;

    let existed = list_profiles()?.contains(&name.to_string());
    if !force && existed {
        return Err(CsError::ProfileExists { name: name.into() });
    }

    let env = prompt_profile_env(None)?;
    save_profile(name, &serde_json::Value::Object(env))?;

    if existed {
        output::success(&format!("Overwritten profile '{}'", name));
    } else {
        output::success(&format!("Created profile '{}'", name));
    }
    Ok(())
}
