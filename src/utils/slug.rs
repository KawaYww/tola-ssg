//! URL slugification utilities.
//!
//! Converts paths and fragments to URL-safe formats.

use crate::config::{SiteConfig, SlugMode};
use std::path::{Path, PathBuf};

/// Characters forbidden in file paths and fragments
const FORBIDDEN_CHARS: &[char] = &[
    '<', '>', ':', '|', '?', '*', '#', '\\', '(', ')', '[', ']', '\t', '\r', '\n',
];

/// Convert fragment text to URL-safe format based on config
pub fn slugify_fragment(text: &str, config: &'static SiteConfig) -> String {
    match config.build.slug.fragment {
        SlugMode::Safe => sanitize_text(text),
        SlugMode::On => slug::slugify(text),
        SlugMode::No => text.to_owned(),
    }
}

/// Convert path to URL-safe format based on config
pub fn slugify_path(path: impl AsRef<Path>, config: &'static SiteConfig) -> PathBuf {
    match config.build.slug.path {
        SlugMode::Safe => sanitize_path(path.as_ref()),
        SlugMode::On => slug::slugify(path.as_ref().to_string_lossy()).into(),
        SlugMode::No => path.as_ref().to_path_buf(),
    }
}

/// Remove forbidden characters and replace whitespace with underscores
fn sanitize_text(text: &str) -> String {
    text.trim()
        .chars()
        .filter(|c| !FORBIDDEN_CHARS.contains(c))
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect()
}

/// Sanitize each component of a path
fn sanitize_path(path: &Path) -> PathBuf {
    path.components()
        .map(|c| sanitize_text(&c.as_os_str().to_string_lossy()))
        .collect()
}
