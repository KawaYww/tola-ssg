use colored::{ColoredString, Colorize};
use crossterm::{
    execute,
    terminal::{Clear, ClearType, size},
};
use std::{
    io::{Write, stdout},
    sync::OnceLock,
};

/// Cached terminal width (only fetched once)
static TERMINAL_WIDTH: OnceLock<u16> = OnceLock::new();

fn get_terminal_width() -> u16 {
    *TERMINAL_WIDTH.get_or_init(|| size().map(|(w, _)| w).unwrap_or(120))
}

/// Modules that use carriage return instead of newline (for progress display)
const INLINE_MODULES: &[&str] = &["content", "assets", "svg"];

#[macro_export]
macro_rules! log {
    ($newline:expr; $module:expr; $($arg:tt)*) => {{
        $crate::utils::log::log($module, &format!($($arg)*), $newline)
    }};
    ($module:expr; $($arg:tt)*) => {{
        $crate::utils::log::log($module, &format!($($arg)*), false)
    }};
}

#[inline]
pub fn log(module: &str, message: &str, force_newline: bool) {
    let module_lower = module.to_ascii_lowercase();
    let use_newline = force_newline || !INLINE_MODULES.contains(&module_lower.as_str());

    let prefix = colorize_prefix(module, &module_lower);
    let width = get_terminal_width() as usize;

    let mut stdout = stdout().lock();
    execute!(stdout, Clear(ClearType::UntilNewLine)).ok();

    // Write prefix and message, truncating if needed
    let prefix_len = module.len() + 3; // "[module] "
    let max_msg_len = width.saturating_sub(prefix_len + 1);

    if message.len() <= max_msg_len {
        if use_newline {
            writeln!(stdout, "{prefix} {message}").ok();
        } else {
            write!(stdout, "{prefix} {message}\r").ok();
        }
    } else {
        // Truncate message (byte-safe for ASCII, char-safe for Unicode)
        let truncated = truncate_str(message, max_msg_len);
        if use_newline {
            writeln!(stdout, "{prefix} {truncated}").ok();
        } else {
            write!(stdout, "{prefix} {truncated}\r").ok();
        }
    }

    stdout.flush().ok();
}

#[inline]
fn colorize_prefix(module: &str, module_lower: &str) -> ColoredString {
    let prefix = format!("[{module}]");
    match module_lower {
        "serve" => prefix.bright_blue().bold(),
        "watch" => prefix.bright_green().bold(),
        "error" => prefix.bright_red().bold(),
        _ => prefix.bright_yellow().bold(),
    }
}

/// Truncate string to max bytes, ensuring valid UTF-8 boundary
#[inline]
fn truncate_str(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        return s;
    }
    // Find the last valid UTF-8 boundary within max_len
    let mut end = max_len;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}
