use colored::Colorize;
use serde_json::Value;
use similar::{ChangeTag, TextDiff};

pub fn success(msg: &str) {
    println!("{}", msg.green().bold());
}

pub fn error(msg: &str) {
    eprintln!("{}", format!("Error: {}", msg).red().bold());
}

pub fn hint(msg: &str) {
    eprintln!("{}", format!("Hint: {}", msg).blue());
}

pub fn info(msg: &str) {
    println!("{}", msg);
}

pub fn warn(msg: &str) {
    eprintln!("{}", format!("Warning: {}", msg).yellow());
}

pub fn written(key: &str, value: &Value) {
    println!("  {} = {}", key, value);
}

pub fn removed(key: &str) {
    println!("  - {} (removed)", key.yellow());
}

pub enum ListStatus {
    Inactive,
    Active,
    Outdated,
    Missing,
}

impl ListStatus {
    fn suffix(&self) -> colored::ColoredString {
        match self {
            ListStatus::Inactive => "".normal(),
            ListStatus::Active => "(active)".green(),
            ListStatus::Outdated => "(active - outdated)".yellow(),
            ListStatus::Missing => "(active - missing!)".red(),
        }
    }
}

pub fn list_item(name: &str, status: &ListStatus) {
    match status {
        ListStatus::Inactive => println!("    {}", name),
        _ => println!("{} {} {}", "  *".green().bold(), name.bold(), status.suffix()),
    }
}

/// 逐行渲染两段文本的差异
pub fn render_diff(current_label: &str, profile_label: &str, current: &str, profile: &str) {
    println!("--- {}", current_label);
    println!("+++ {}", profile_label);

    for change in TextDiff::from_lines(current, profile).iter_all_changes() {
        let line = change.to_string_lossy();
        match change.tag() {
            ChangeTag::Delete => println!("-{}", line.red()),
            ChangeTag::Insert => println!("+{}", line.green()),
            ChangeTag::Equal => println!(" {}", line),
        }
    }
}
