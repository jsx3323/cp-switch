use std::io;

use crate::error::{CsError, io_err};
use crate::output;

fn read_trimmed() -> Result<String, CsError> {
    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(|e| io_err("stdin", e))?;
    Ok(input.trim().to_string())
}

pub fn prompt_required(field: &str) -> Result<String, CsError> {
    loop {
        eprintln!("{}: ", field);
        let value = read_trimmed()?;
        if value.is_empty() {
            output::error(&format!("{} is required", field));
            continue;
        }
        return Ok(value);
    }
}

/// 显示默认值，空输入保留默认值，否则使用用户输入
pub fn prompt_with_default(field: &str, default: &str) -> Result<String, CsError> {
    eprintln!("{} (default: {}): ", field, default);
    let value = read_trimmed()?;
    Ok(if value.is_empty() { default.to_string() } else { value })
}

pub fn prompt_optional(field: &str, default: &str) -> Result<Option<String>, CsError> {
    eprintln!("{} (optional, default: {}): ", field, default);
    let value = read_trimmed()?;
    Ok((!value.is_empty()).then_some(value))
}

pub fn prompt_confirm(msg: &str) -> Result<bool, CsError> {
    eprintln!("{} [y/N] ", msg);
    Ok(read_trimmed()?.to_lowercase() == "y")
}
