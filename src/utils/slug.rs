use std::path::{Path, PathBuf};
use crate::config::{SiteConfig, SlugMode};
use rayon::prelude::*;

const FORBIDDEN: &[char] = &['<', '>', ':', '|', '?', '*', '#', '\\', '(', ')', '[', ']', '\t', '\r', '\n'];

pub fn slugify_fragment(text: &str, config: &'static SiteConfig) -> String {
    let slug_mode = &config.build.slug.fragment;
    
    match slug_mode {
        SlugMode::Safe => sanitize_text(text),
        SlugMode::On => slug::slugify(text),
        SlugMode::No => text.to_owned(),
    }
}

pub fn slugify_path(path: impl AsRef<Path>, config: &'static SiteConfig) -> PathBuf {
    let slug_mode = &config.build.slug.path;
    let path = path.as_ref();
    
    match slug_mode {
        SlugMode::Safe => slugify_safe(path),
        SlugMode::On => slugify_on(path),
        SlugMode::No => path.to_path_buf(),
    }
}

fn sanitize_text(text: &str) -> String {
    text.trim().par_chars()
        .filter(|c| !FORBIDDEN.contains(c))
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect::<String>()
}

fn sanitize_path(path: &Path) -> PathBuf {
    let components: Vec<_> = path.components()
        .map(|segmant| segmant.as_os_str().to_string_lossy())
        .map(|s| sanitize_text(s.as_ref()))
        .collect();

    PathBuf::from_iter(components)
}

fn slugify_safe(path: impl AsRef<Path>) -> PathBuf {
    sanitize_path(path.as_ref())
}

fn slugify_on(path: impl AsRef<Path>) -> PathBuf {
    let path = sanitize_path(path.as_ref());
    slug::slugify(path.to_string_lossy()).into()
}

