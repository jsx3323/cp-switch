use crate::error::CsError;
use crate::output;
use crate::store::copy_profile;

pub fn run(src: &str, dst: &str, force: bool) -> Result<(), CsError> {
    if copy_profile(src, dst, force)? {
        output::success(&format!("Copied '{}' onto existing profile '{}'", src, dst));
    } else {
        output::success(&format!("Copied profile '{}' to '{}'", src, dst));
    }
    Ok(())
}
