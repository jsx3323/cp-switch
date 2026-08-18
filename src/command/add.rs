use crate::cli::validate_profile_arg;
use crate::command::prompt::prompt_profile_env;
use crate::error::CsError;
use crate::output;
use crate::store::{profile_exists, save_profile};

pub fn run(name: &str, force: bool) -> Result<(), CsError> {
    validate_profile_arg(name)?;

    let existed = profile_exists(name)?;
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
