use std::io;

use crate::error::{CsError, io_err};
use crate::output;

/// 读取一行并去掉首尾空白。返回 `None` 表示 stdin 已到 EOF——
/// 可选字段据此回退到默认值，必填字段据此报错而不是无限重试。
fn read_trimmed() -> Result<Option<String>, CsError> {
    let mut input = String::new();
    let read = io::stdin().read_line(&mut input).map_err(|e| io_err("stdin", e))?;
    if read == 0 {
        return Ok(None);
    }
    Ok(Some(input.trim().to_string()))
}

pub fn prompt_required(field: &str) -> Result<String, CsError> {
    loop {
        eprintln!("{}: ", field);
        match read_trimmed()? {
            // EOF 后再问也只会拿到 EOF，重试会变成死循环
            None => return Err(CsError::MissingInput { field: field.into() }),
            Some(value) if value.is_empty() => output::error(&format!("{} is required", field)),
            Some(value) => return Ok(value),
        }
    }
}

/// 显示默认值，空输入保留默认值，否则使用用户输入
pub fn prompt_with_default(field: &str, default: &str) -> Result<String, CsError> {
    eprintln!("{} (default: {}): ", field, default);
    Ok(read_trimmed()?
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_string()))
}

pub fn prompt_optional(field: &str, default: &str) -> Result<Option<String>, CsError> {
    eprintln!("{} (optional, default: {}): ", field, default);
    Ok(read_trimmed()?.filter(|value| !value.is_empty()))
}

pub fn prompt_confirm(msg: &str) -> Result<bool, CsError> {
    eprintln!("{} [y/N] ", msg);
    Ok(read_trimmed()?.is_some_and(|answer| answer.to_lowercase() == "y"))
}
