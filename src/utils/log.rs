#[macro_export]
macro_rules! log {
    ($module:expr, $($arg:tt)*) => {{
        use $crate::utils::log::log;

        let log_message = format!($($arg)*);
        log($module, log_message) 
    }};
}

pub fn log(module: &str, message: String) {
    use colored::Colorize;
    use std::io::{stdout, Write};
    use crossterm::{execute, terminal::{Clear, ClearType}, cursor::MoveUp};

    let module_lower = module.to_lowercase();
    let is_important = matches!(module.to_lowercase().as_str(), "note" | "build" | "serve" | "watch" | "init" | "deploy" | "commit" | "git" | "error");
    
    let colored_prefix = match module_lower.as_str() {
        "serve" => format!("[{module}]").bright_blue().bold(),
        "watch" => format!("[{module}]").bright_green().bold(),
        "error" => format!("[{module}]").bright_red().bold(),
        _ => format!("[{module}]").bright_yellow().bold(),
    };

    let mut stdout = stdout().lock();
    execute!(stdout, Clear(ClearType::CurrentLine)).ok();
    writeln!(stdout, "{colored_prefix} {message}").ok();
    if !is_important {
        execute!(stdout, MoveUp(1)).ok();
    }
    stdout.flush().ok();
}
