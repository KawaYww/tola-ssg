#[macro_export]
macro_rules! log {
    ($newline:expr; $module:expr; $($arg:tt)*) => {{
        use $crate::utils::log::log;

        let log_message = format!($($arg)*);
        log($module, log_message, $newline) 
    }};
    ($module:expr; $($arg:tt)*) => {{
        use $crate::utils::log::log;

        let log_message = format!($($arg)*);
        log($module, log_message, false) 
    }};
}

pub fn log(module: &str, message: String, force_newline: bool) {
    use colored::Colorize;
    use std::io::{stdout, Write};
    #[allow(unused_imports)]
    use crossterm::{execute, terminal::{size, Clear, ClearType}, cursor::{MoveTo, MoveUp}};

    let module_lower = module.to_lowercase();
    let should_newline = force_newline || !matches!(module.to_lowercase().as_str(), "content" | "assets" | "svg");
    
    let colored_prefix = match module_lower.as_str() {
        "serve" => format!("[{module}]").bright_blue().bold(),
        "watch" => format!("[{module}]").bright_green().bold(),
        "error" => format!("[{module}]").bright_red().bold(),
        _ => format!("[{module}]").bright_yellow().bold(),
    };

    let mut stdout = stdout().lock();
    let (width, _) = size().unwrap_or((80, 25));

    // execute!(stdout, Clear(ClearType::CurrentLine)).ok();
    execute!(stdout, 
        Clear(ClearType::UntilNewLine)
    ).ok();

    let log_msg = format!("{colored_prefix} {message}"); 
    let log_msg = if log_msg.len() > width as usize { log_msg.chars().take(width as usize - 1).collect::<String>() } else { log_msg };

    if should_newline {
        writeln!(stdout, "{log_msg}").ok();
    } else {
        write!(stdout, "{log_msg}\r").ok();
    }

    stdout.flush().ok();
}
