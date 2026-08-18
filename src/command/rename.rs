use crate::error::CsError;
use crate::output;
use crate::store::rename_profile;

pub fn run(src: &str, dst: &str, force: bool) -> Result<(), CsError> {
    if rename_profile(src, dst, force)? {
        output::success(&format!("Renamed '{}' onto existing profile '{}'", src, dst));
    } else {
        output::success(&format!("Renamed profile '{}' to '{}'", src, dst));
    }
    Ok(())
}
