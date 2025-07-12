#[macro_export]
macro_rules! log {
    ($module:literal, $($arg:tt)*) => {{
        use colored::Colorize;
        use std::io::{stdout, Write};
        use crossterm::{execute, terminal::{Clear, ClearType}, cursor::MoveUp};

        let module_lower = $module.to_lowercase();
        let is_important = matches!(module_lower.as_str(), "server" | "watcher" | "initer" | "deployer" | "error");
        
        let colored_prefix = match module_lower.as_str() {
            "server" => format!("[{}]", $module).bright_blue().bold(),
            "watcher" => format!("[{}]", $module).bright_green().bold(),
            "error" => format!("[{}]", $module).bright_red().bold(),
            _ => format!("[{}]", $module).bright_yellow().bold(),
        };

        let log_message = format!($($arg)*);
        let mut stdout = stdout().lock();
        if is_important {
            execute!(stdout, Clear(ClearType::CurrentLine)).ok();
            writeln!(stdout, "{} {}", colored_prefix, log_message).ok();
        } else {
            execute!(stdout, Clear(ClearType::CurrentLine)).ok();
            writeln!(stdout, "{} {}", colored_prefix, log_message).ok();
            execute!(stdout, MoveUp(1)).ok();
        }
        stdout.flush().ok();
    }};
}
#[macro_export]
macro_rules! log {
    ($module:literal, $($arg:tt)*) => {{
        use colored::Colorize;
        use std::io::{stdout, Write};
        use crossterm::{execute, terminal::{Clear, ClearType}, cursor::MoveUp};

        let module_lower = $module.to_lowercase();
        let is_important = matches!(module_lower.as_str(), "builder" | "server" | "watcher" | "initer" | "deployer" | "error");
        let is_important = matches!(module_lower.as_str(), "builder" | "server" | "watcher" | "initer" | "deployer" | "commit" | "error");
        let is_important = matches!(module_lower.as_str(), "builder" | "server" | "watcher" | "initer" | "deployer" | "commit" | "git" | "error");
        
        let colored_prefix = match module_lower.as_str() {
            "server" => format!("[{}]", $module).bright_blue().bold(),
            "watcher" => format!("[{}]", $module).bright_green().bold(),
            "error" => format!("[{}]", $module).bright_red().bold(),
            _ => format!("[{}]", $module).bright_yellow().bold(),
        };

        let log_message = format!($($arg)*);
        let mut stdout = stdout().lock();
        if is_important {
            execute!(stdout, Clear(ClearType::CurrentLine)).ok();
            writeln!(stdout, "{} {}", colored_prefix, log_message).ok();
        } else {
            execute!(stdout, Clear(ClearType::CurrentLine)).ok();
            writeln!(stdout, "{} {}", colored_prefix, log_message).ok();
            execute!(stdout, MoveUp(1)).ok();
        }
        stdout.flush().ok();
    }};
}
