use crate::cli::validate_name;
use crate::command::prompt::prompt_profile_env;
use crate::error::CsError;
use crate::output;
use crate::store::{save_profile, profile_path};

pub fn run(name: &str, force: bool) -> Result<(), CsError> {
    validate_name(name)?;

    let existed = profile_path(name).exists();
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
