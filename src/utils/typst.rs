//! Typst library integration for compiling Typst files to HTML.
//!
//! This module provides a World implementation and compilation utilities
//! that replace the external typst CLI with direct library usage.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{Datelike, Local, Utc};
use parking_lot::Mutex;
use typst::diag::{FileError, FileResult, PackageError};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Feature, Library, LibraryExt, World};
use typst_html::HtmlDocument;
use typst_kit::fonts::{FontSearcher, FontSlot, Fonts};

/// A World implementation for compiling Typst files.
pub struct TolaWorld {
    /// The root directory for resolving paths.
    root: PathBuf,
    /// The main source file ID.
    main: FileId,
    /// Typst's standard library.
    library: LazyHash<Library>,
    /// Metadata about discovered fonts.
    book: LazyHash<FontBook>,
    /// Font slots for lazy loading.
    fonts: Vec<FontSlot>,
    /// Cache of loaded source files.
    sources: Mutex<HashMap<FileId, Source>>,
    /// Cache of loaded binary files.
    files: Mutex<HashMap<FileId, Bytes>>,
}

impl TolaWorld {
    /// Create a new world for compiling a Typst file.
    pub fn new(root: &Path, main_path: &Path, font_paths: &[PathBuf]) -> Result<Self> {
        let root = root
            .canonicalize()
            .with_context(|| format!("Failed to canonicalize root: {}", root.display()))?;

        let main_path = main_path
            .canonicalize()
            .with_context(|| format!("Failed to canonicalize main path: {}", main_path.display()))?;

        // Resolve the virtual path of the main file within the project root
        let main_vpath = VirtualPath::within_root(&main_path, &root)
            .with_context(|| "Main file must be within the project root")?;
        let main = FileId::new(None, main_vpath);

        // Build the library with HTML feature enabled
        let library = Library::builder()
            .with_features([Feature::Html].into_iter().collect())
            .build();

        // Search for fonts
        let fonts = Self::search_fonts(font_paths, &root);

        Ok(Self {
            root,
            main,
            library: LazyHash::new(library),
            book: LazyHash::new(fonts.book),
            fonts: fonts.fonts,
            sources: Mutex::new(HashMap::new()),
            files: Mutex::new(HashMap::new()),
        })
    }

    /// Search for fonts in the specified paths and system directories.
    fn search_fonts(font_paths: &[PathBuf], root: &Path) -> Fonts {
        let mut searcher = FontSearcher::new();
        searcher.include_system_fonts(true);

        // Add root directory as font path
        let mut paths: Vec<&Path> = vec![root];
        for path in font_paths {
            paths.push(path.as_path());
        }

        searcher.search_with(paths)
    }

    /// Read a file from the file system.
    fn read_file(&self, id: FileId) -> FileResult<Vec<u8>> {
        let path = self.resolve_path(id)?;
        fs::read(&path).map_err(|e| FileError::from_io(e, &path))
    }

    /// Resolve a FileId to a file system path.
    fn resolve_path(&self, id: FileId) -> FileResult<PathBuf> {
        // For now, we don't support packages
        if id.package().is_some() {
            return Err(FileError::Package(PackageError::Other(Some(
                ecow::eco_format!("packages are not supported"),
            ))));
        }

        id.vpath()
            .resolve(&self.root)
            .ok_or(FileError::AccessDenied)
    }

    /// Reset caches for a new compilation.
    #[allow(dead_code)]
    pub fn reset(&self) {
        self.sources.lock().clear();
        self.files.lock().clear();
    }
}

impl World for TolaWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        // Check cache first
        if let Some(source) = self.sources.lock().get(&id) {
            return Ok(source.clone());
        }

        // Read and parse the file
        let data = self.read_file(id)?;
        let text = String::from_utf8(data).map_err(|_| FileError::InvalidUtf8)?;
        let source = Source::new(id, text);

        // Cache the source
        self.sources.lock().insert(id, source.clone());

        Ok(source)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        // Check cache first
        if let Some(bytes) = self.files.lock().get(&id) {
            return Ok(bytes.clone());
        }

        // Read the file
        let data = self.read_file(id)?;
        let bytes = Bytes::new(data);

        // Cache the bytes
        self.files.lock().insert(id, bytes.clone());

        Ok(bytes)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index)?.get()
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let now = Utc::now();

        let with_offset = match offset {
            None => Local::now().fixed_offset(),
            Some(hours) => {
                let seconds = i32::try_from(hours).ok()?.checked_mul(3600)?;
                now.with_timezone(&chrono::FixedOffset::east_opt(seconds)?)
            }
        };

        Datetime::from_ymd(
            with_offset.year(),
            with_offset.month().try_into().ok()?,
            with_offset.day().try_into().ok()?,
        )
    }
}

/// Compile a Typst file to HTML using the typst library.
pub fn compile_to_html(root: &Path, content_path: &Path, font_paths: &[PathBuf]) -> Result<Vec<u8>> {
    let world = TolaWorld::new(root, content_path, font_paths)?;

    // Compile to HTML document
    let result = typst::compile::<HtmlDocument>(&world);

    // Handle warnings (just log them for now)
    for warning in &result.warnings {
        // Skip the standard HTML export warning
        let msg = warning.message.to_string();
        if !msg.contains("html export is under active development") {
            eprintln!("typst warning: {}", msg);
        }
    }

    // Handle the compilation result
    match result.output {
        Ok(document) => {
            // Convert the HTML document to a string
            let html = typst_html::html(&document)
                .map_err(|errors| {
                    let messages: Vec<_> = errors.iter().map(|e| e.message.to_string()).collect();
                    anyhow::anyhow!("HTML encoding failed: {}", messages.join(", "))
                })?;
            Ok(html.into_bytes())
        }
        Err(errors) => {
            let messages: Vec<_> = errors.iter().map(|e| e.message.to_string()).collect();
            Err(anyhow::anyhow!("Typst compilation failed:\n{}", messages.join("\n")))
        }
    }
}
