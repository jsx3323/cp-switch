use colored::Colorize;

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
    let prefix = match status {
        ListStatus::Inactive => "  ",
        _ => "  *",
    };
    let suffix = status.suffix();
    if suffix.is_empty() {
        println!("{}  {}", prefix, name);
    } else {
        println!("{} {} {}", prefix.green().bold(), name.bold(), suffix);
    }
}

pub fn diff_header(current_label: &str, profile_label: &str) {
    println!("--- {}", current_label);
    println!("+++ {}", profile_label);
}

pub fn diff_deleted(line: &str) {
    println!("-{}", line.red());
}

pub fn diff_inserted(line: &str) {
    println!("+{}", line.green());
}

pub fn diff_equal(line: &str) {
    println!(" {}", line);
}
