//! Typst library integration for compiling Typst files to HTML.
//!
//! This module provides a [`World`] implementation and compilation utilities
//! that replace the external typst CLI with direct library usage.
//!
//! # Architecture
//!
//! The module is organized into the following components:
//! - [`TolaWorld`]: Implements typst's [`World`] trait for file resolution, fonts, and packages
//! - [`FontManager`]: Handles font discovery from system and custom directories
//! - [`compile_to_html`]: Main entry point for compiling Typst files to HTML
//!
//! # Font Discovery
//!
//! Fonts are discovered in the following order (higher priority first):
//! 1. Custom font paths passed to [`compile_to_html`]
//! 2. Project root directory (equivalent to `--font-path root` in typst CLI)
//! 3. System fonts (platform-specific directories)
//!
//! # Package Support
//!
//! Supports Typst packages from:
//! - Official registry: `#import "@preview/package:version"`
//! - Local packages: `#import "@local/package:version"` (from user data directory)
//!
//! # Error Handling
//!
//! - Compilation errors are collected and returned with source locations
//! - Warnings are logged using the project's logging framework
//! - File access errors include path context for debugging

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use chrono::{Datelike, Local, Utc};
use parking_lot::Mutex;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Feature, Library, LibraryExt, World};
use typst_html::HtmlDocument;
use typst_kit::download::{DownloadState, Downloader, Progress};
use typst_kit::fonts::{FontSearcher, FontSlot};
use typst_kit::package::PackageStorage;

// ============================================================================
// Constants
// ============================================================================

/// User agent string for package downloads
const USER_AGENT: &str = concat!("tola-ssg/", env!("CARGO_PKG_VERSION"));

/// Warnings to suppress (these are expected and not actionable by users)
const SUPPRESSED_WARNINGS: &[&str] = &["html export is under active development"];

// ============================================================================
// Progress Reporter
// ============================================================================

/// A silent progress reporter for package downloads.
///
/// In the future, this could be extended to show download progress in the terminal.
struct SilentProgress;

impl Progress for SilentProgress {
    fn print_start(&mut self) {}
    fn print_progress(&mut self, _: &DownloadState) {}
    fn print_finish(&mut self, _: &DownloadState) {}
}

// ============================================================================
// Font Manager
// ============================================================================

/// Manages font discovery and loading.
///
/// Fonts are discovered from multiple sources:
/// - System font directories (platform-specific)
/// - Project root directory
/// - Additional custom font paths
#[derive(Debug)]
struct FontManager {
    /// Metadata about all discovered fonts
    book: LazyHash<FontBook>,
    /// Font slots for lazy loading
    slots: Vec<FontSlot>,
}

impl FontManager {
    /// Create a new font manager with fonts from the specified directories.
    ///
    /// # Arguments
    /// * `root` - Project root directory (always included as font path)
    /// * `font_paths` - Additional font directories to search
    /// * `include_system` - Whether to include system fonts
    fn new(root: &Path, font_paths: &[PathBuf], include_system: bool) -> Self {
        let mut searcher = FontSearcher::new();
        searcher.include_system_fonts(include_system);

        // Build list of font paths: root first, then custom paths
        let mut paths: Vec<&Path> = Vec::with_capacity(1 + font_paths.len());
        paths.push(root);
        paths.extend(font_paths.iter().map(PathBuf::as_path));

        let fonts = searcher.search_with(paths);

        Self {
            book: LazyHash::new(fonts.book),
            slots: fonts.fonts,
        }
    }

    /// Get the font book containing metadata for all fonts.
    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    /// Get a font by its index in the font book.
    fn get(&self, index: usize) -> Option<Font> {
        self.slots.get(index)?.get()
    }
}

// ============================================================================
// Package Manager
// ============================================================================

/// Manages package resolution and downloading.
struct PackageManager {
    storage: PackageStorage,
}

impl PackageManager {
    /// Create a new package manager with default storage paths.
    fn new() -> Self {
        let downloader = Downloader::new(USER_AGENT);
        let storage = PackageStorage::new(None, None, downloader);
        Self { storage }
    }

    /// Resolve a package to its directory on disk.
    ///
    /// Downloads the package if not already cached.
    fn resolve(&self, spec: &typst::syntax::package::PackageSpec) -> FileResult<PathBuf> {
        self.storage
            .prepare_package(spec, &mut SilentProgress)
            .map_err(FileError::Package)
    }
}

// ============================================================================
// File Cache
// ============================================================================

/// Thread-safe cache for source files and binary data.
struct FileCache {
    sources: Mutex<HashMap<FileId, Source>>,
    files: Mutex<HashMap<FileId, Bytes>>,
}

impl FileCache {
    fn new() -> Self {
        Self {
            sources: Mutex::new(HashMap::new()),
            files: Mutex::new(HashMap::new()),
        }
    }

    /// Get or insert a source file into the cache.
    fn get_or_insert_source(
        &self,
        id: FileId,
        loader: impl FnOnce() -> FileResult<Source>,
    ) -> FileResult<Source> {
        // Check cache first
        if let Some(source) = self.sources.lock().get(&id) {
            return Ok(source.clone());
        }

        // Load and cache
        let source = loader()?;
        self.sources.lock().insert(id, source.clone());
        Ok(source)
    }

    /// Get or insert a binary file into the cache.
    fn get_or_insert_file(
        &self,
        id: FileId,
        loader: impl FnOnce() -> FileResult<Bytes>,
    ) -> FileResult<Bytes> {
        // Check cache first
        if let Some(bytes) = self.files.lock().get(&id) {
            return Ok(bytes.clone());
        }

        // Load and cache
        let bytes = loader()?;
        self.files.lock().insert(id, bytes.clone());
        Ok(bytes)
    }
}

impl Default for FileCache {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Typst World Implementation
// ============================================================================

/// A [`World`] implementation for compiling Typst files in Tola.
///
/// This struct provides all the context needed for typst compilation:
/// - File resolution (local files and packages)
/// - Font discovery and loading
/// - Standard library access
/// - Date/time information
pub struct TolaWorld {
    /// Project root directory for resolving paths
    root: PathBuf,
    /// Main source file identifier
    main: FileId,
    /// Typst standard library with HTML feature enabled
    library: LazyHash<Library>,
    /// Font manager for font discovery and loading
    fonts: FontManager,
    /// Package manager for resolving external packages
    packages: PackageManager,
    /// Cache for loaded files
    cache: Arc<FileCache>,
}

impl TolaWorld {
    /// Create a new world for compiling a Typst file.
    ///
    /// # Arguments
    /// * `root` - Project root directory (used for file resolution and as font path)
    /// * `main_path` - Path to the main Typst source file
    /// * `font_paths` - Additional directories to search for fonts
    ///
    /// # Errors
    /// Returns an error if:
    /// - The root or main path cannot be canonicalized
    /// - The main file is not within the project root
    pub fn new(root: &Path, main_path: &Path, font_paths: &[PathBuf]) -> Result<Self> {
        // Canonicalize paths for consistent resolution
        let root = root
            .canonicalize()
            .with_context(|| format!("Failed to resolve project root: {}", root.display()))?;

        let main_path = main_path
            .canonicalize()
            .with_context(|| format!("Failed to resolve main file: {}", main_path.display()))?;

        // Resolve the virtual path of the main file within the project root
        let main_vpath = VirtualPath::within_root(&main_path, &root)
            .with_context(|| format!(
                "Main file '{}' must be within project root '{}'",
                main_path.display(),
                root.display()
            ))?;
        let main = FileId::new(None, main_vpath);

        // Build the library with HTML feature enabled
        let library = Library::builder()
            .with_features([Feature::Html].into_iter().collect())
            .build();

        Ok(Self {
            fonts: FontManager::new(&root, font_paths, true),
            packages: PackageManager::new(),
            cache: Arc::new(FileCache::new()),
            root,
            main,
            library: LazyHash::new(library),
        })
    }

    /// Resolve a file ID to a filesystem path.
    fn resolve_path(&self, id: FileId) -> FileResult<PathBuf> {
        // Handle package imports
        if let Some(spec) = id.package() {
            let package_dir = self.packages.resolve(spec)?;
            return id
                .vpath()
                .resolve(&package_dir)
                .ok_or(FileError::AccessDenied);
        }

        // Local file resolution
        id.vpath()
            .resolve(&self.root)
            .ok_or(FileError::AccessDenied)
    }

    /// Read raw bytes from a file.
    fn read_bytes(&self, id: FileId) -> FileResult<Vec<u8>> {
        let path = self.resolve_path(id)?;
        fs::read(&path).map_err(|e| FileError::from_io(e, &path))
    }
}

impl World for TolaWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.book()
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.cache.get_or_insert_source(id, || {
            let data = self.read_bytes(id)?;
            let text = String::from_utf8(data).map_err(|_| FileError::InvalidUtf8)?;
            Ok(Source::new(id, text))
        })
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.cache.get_or_insert_file(id, || {
            let data = self.read_bytes(id)?;
            Ok(Bytes::new(data))
        })
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index)
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let datetime = match offset {
            None => Local::now().fixed_offset(),
            Some(hours) => {
                let seconds = i32::try_from(hours).ok()?.checked_mul(3600)?;
                let tz = chrono::FixedOffset::east_opt(seconds)?;
                Utc::now().with_timezone(&tz)
            }
        };

        Datetime::from_ymd(
            datetime.year(),
            datetime.month().try_into().ok()?,
            datetime.day().try_into().ok()?,
        )
    }
}

// ============================================================================
// Compilation API
// ============================================================================

/// Compile a Typst file to HTML.
///
/// This is the main entry point for Typst compilation. It creates a world,
/// compiles the document, and returns the HTML as bytes.
///
/// # Arguments
/// * `root` - Project root directory (used for file resolution and as font path)
/// * `content_path` - Path to the Typst source file to compile
/// * `font_paths` - Additional directories to search for fonts
///
/// # Returns
/// The compiled HTML document as a byte vector.
///
/// # Errors
/// Returns an error if:
/// - World creation fails (invalid paths)
/// - Compilation fails (syntax errors, missing files, etc.)
/// - HTML encoding fails
///
/// # Example
/// ```ignore
/// let html = compile_to_html(
///     Path::new("/project"),
///     Path::new("/project/content/index.typ"),
///     &[],
/// )?;
/// ```
pub fn compile_to_html(root: &Path, content_path: &Path, font_paths: &[PathBuf]) -> Result<Vec<u8>> {
    // Create the world
    let world = TolaWorld::new(root, content_path, font_paths)?;

    // Compile to HTML document
    let result = typst::compile::<HtmlDocument>(&world);

    // Log warnings (excluding suppressed ones)
    log_warnings(&result.warnings);

    // Handle compilation result
    match result.output {
        Ok(document) => encode_html(&document),
        Err(errors) => {
            let messages: Vec<_> = errors
                .iter()
                .map(|e| format!("  • {}", e.message))
                .collect();
            bail!("Typst compilation failed:\n{}", messages.join("\n"))
        }
    }
}

/// Log compilation warnings using the project's logging framework.
fn log_warnings(warnings: &[typst::diag::SourceDiagnostic]) {
    for warning in warnings {
        let msg = warning.message.to_string();

        // Skip suppressed warnings
        if SUPPRESSED_WARNINGS.iter().any(|s| msg.contains(s)) {
            continue;
        }

        crate::log!(true; "typst"; "warning: {}", msg);
    }
}

/// Encode an HTML document to bytes.
fn encode_html(document: &HtmlDocument) -> Result<Vec<u8>> {
    typst_html::html(document)
        .map(|html| html.into_bytes())
        .map_err(|errors| {
            let messages: Vec<_> = errors
                .iter()
                .map(|e| format!("  • {}", e.message))
                .collect();
            anyhow::anyhow!("HTML encoding failed:\n{}", messages.join("\n"))
        })
}
