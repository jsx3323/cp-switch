use crate::error::CsError;
use crate::output::{self, ListStatus};
use crate::store::{Scope, is_builtin, is_env_applied};

pub fn run(scope: &Scope) -> Result<(), CsError> {
    let listing = scope.read_listing()?;

    if listing.profiles.is_empty() && listing.current.is_none() {
        output::info("No profiles found. Use 'cp-switch add <name>' to create one.");
        return Ok(());
    }

    match scope.project() {
        Some(project) => output::info(&format!("Profiles for {}:", project.display())),
        None => output::info("Profiles (user):"),
    }

    // 至多一个 profile 处于活跃状态，settings 只需读一次，不放进循环
    let current_env = match &listing.active_env {
        Some(_) => Some(scope.read_current_env()?),
        None => None,
    };

    for name in &listing.profiles {
        let status = match (listing.current.as_ref() == Some(name), &current_env, &listing.active_env) {
            (false, _, _) => ListStatus::Inactive,
            (true, Some(current), Some(profile)) if is_env_applied(current, profile) => ListStatus::Active,
            (true, ..) => ListStatus::Outdated,
        };
        output::list_item(name, &status);
    }

    // 活跃的 profile 不在名单里：内置 claude 属正常，其余是被删掉的残留标记
    if let Some(active) = &listing.current
        && !listing.profiles.contains(active)
    {
        let status = if is_builtin(active) { ListStatus::Active } else { ListStatus::Missing };
        output::list_item(active, &status);
    }

    output::info(&format!(
        "{} profiles, {} active",
        listing.profiles.len(),
        usize::from(listing.current.is_some())
    ));
    Ok(())
}
