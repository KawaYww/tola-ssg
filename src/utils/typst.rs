use std::fs;
use std::io::{self, Read};
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, OnceLock};

use chrono::{DateTime, Datelike, FixedOffset, Local, Utc};
use parking_lot::Mutex;
use rustc_hash::FxHashMap;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Feature, Features, Library, LibraryExt};
use typst::World;
use typst_kit::download::{DownloadState, Downloader, Progress};
use typst_kit::fonts::Fonts;
use typst_kit::package::PackageStorage;

/// Static `FileId` allocated for stdin.
static STDIN_ID: LazyLock<FileId> =
    LazyLock::new(|| FileId::new_fake(VirtualPath::new("<stdin>")));

/// Static `FileId` allocated for empty/no input at all.
static EMPTY_ID: LazyLock<FileId> =
    LazyLock::new(|| FileId::new_fake(VirtualPath::new("<empty>")));

/// A world that provides access to the operating system.
pub struct SystemWorld {
    /// The root relative to which absolute paths are resolved.
    root: PathBuf,
    /// The input path.
    main: FileId,
    /// Typst's standard library.
    library: LazyHash<Library>,
    /// Metadata about discovered fonts and lazily loaded fonts.
    fonts: LazyLock<(Fonts, LazyHash<FontBook>), Box<dyn Fn() -> (Fonts, LazyHash<FontBook>) + Send + Sync>>,
    /// Maps file ids to source files and buffers.
    slots: Mutex<FxHashMap<FileId, FileSlot>>,
    /// Holds information about where packages are stored.
    package_storage: PackageStorage,
    /// The current datetime if requested.
    now: Now,
}

impl SystemWorld {
    pub fn new(entry_file: &Path, root_dir: &Path) -> Result<Self, anyhow::Error> {
        let root = root_dir.to_path_buf();
        
        // Resolve the virtual path of the main file within the project root.
        let virtual_path = VirtualPath::within_root(entry_file, &root)
            .unwrap_or_else(|| VirtualPath::new(entry_file.file_name().unwrap()));
        let main = FileId::new(None, virtual_path);

        let library = Library::builder()
            .with_features(Features::from_iter([Feature::Html]))
            .build();

        let font_path = root.clone();
        let fonts = LazyLock::new(Box::new(move || {
            let mut searcher = Fonts::searcher();
            searcher.include_system_fonts(true);
            let fonts = searcher.search_with([&font_path]);
            let book = LazyHash::new(fonts.book.clone());
            (fonts, book)
        }) as Box<dyn Fn() -> (Fonts, LazyHash<FontBook>) + Send + Sync>);

        let package_storage = PackageStorage::new(
            None, // Use default cache path
            None, // Use default package path
            Downloader::new(format!("tola/{}", env!("CARGO_PKG_VERSION"))),
        );

        Ok(Self {
            root,
            main,
            library: LazyHash::new(library),
            fonts,
            slots: Mutex::new(FxHashMap::default()),
            package_storage,
            now: Now::System(OnceLock::new()),
        })
    }

    /// Access the canonical slot for the given file id.
    fn slot<F, T>(&self, id: FileId, f: F) -> T
    where
        F: FnOnce(&mut FileSlot) -> T,
    {
        let mut map = self.slots.lock();
        f(map.entry(id).or_insert_with(|| FileSlot::new(id)))
    }
}

impl World for SystemWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.fonts.1
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.slot(id, |slot| slot.source(&self.root, &self.package_storage))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.slot(id, |slot| slot.file(&self.root, &self.package_storage))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.0.fonts.get(index)?.get()
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let now = match &self.now {
            Now::System(time) => time.get_or_init(Utc::now),
        };

        let with_offset = match offset {
            None => now.with_timezone(&Local).fixed_offset(),
            Some(hours) => {
                let seconds = i32::try_from(hours).ok()?.checked_mul(3600)?;
                now.with_timezone(&FixedOffset::east_opt(seconds)?)
            }
        };

        Datetime::from_ymd(
            with_offset.year(),
            with_offset.month().try_into().ok()?,
            with_offset.day().try_into().ok()?,
        )
    }
}

/// Holds the processed data for a file ID.
struct FileSlot {
    /// The slot's file id.
    id: FileId,
    /// The lazily loaded and incrementally updated source file.
    source: SlotCell<Source>,
    /// The lazily loaded raw byte buffer.
    file: SlotCell<Bytes>,
}

impl FileSlot {
    /// Create a new file slot.
    fn new(id: FileId) -> Self {
        Self { id, file: SlotCell::new(), source: SlotCell::new() }
    }

    /// Retrieve the source for this file.
    fn source(
        &mut self,
        project_root: &Path,
        package_storage: &PackageStorage,
    ) -> FileResult<Source> {
        self.source.get_or_init(
            || read(self.id, project_root, package_storage),
            |data, prev| {
                let text = decode_utf8(&data)?;
                if let Some(mut prev) = prev {
                    prev.replace(text);
                    Ok(prev)
                } else {
                    Ok(Source::new(self.id, text.into()))
                }
            },
        )
    }

    /// Retrieve the file's bytes.
    fn file(
        &mut self,
        project_root: &Path,
        package_storage: &PackageStorage,
    ) -> FileResult<Bytes> {
        self.file.get_or_init(
            || read(self.id, project_root, package_storage),
            |data, _| Ok(Bytes::new(data)),
        )
    }
}

/// Lazily processes data for a file.
struct SlotCell<T> {
    /// The processed data.
    data: Option<FileResult<T>>,
    /// A hash of the raw file contents / access error.
    fingerprint: u128,
    /// Whether the slot has been accessed in the current compilation.
    accessed: bool,
}

impl<T: Clone> SlotCell<T> {
    /// Creates a new, empty cell.
    fn new() -> Self {
        Self { data: None, fingerprint: 0, accessed: false }
    }

    /// Gets the contents of the cell or initialize them.
    fn get_or_init(
        &mut self,
        load: impl FnOnce() -> FileResult<Vec<u8>>,
        f: impl FnOnce(Vec<u8>, Option<T>) -> FileResult<T>,
    ) -> FileResult<T> {
        if mem::replace(&mut self.accessed, true)
            && let Some(data) = &self.data
        {
            return data.clone();
        }

        let result = load();
        let fingerprint = typst::utils::hash128(&result);

        if mem::replace(&mut self.fingerprint, fingerprint) == fingerprint
            && let Some(data) = &self.data
        {
            return data.clone();
        }

        let prev = self.data.take().and_then(Result::ok);
        let value = result.and_then(|data| f(data, prev));
        self.data = Some(value.clone());

        value
    }
}

/// Resolves the path of a file id on the system, downloading a package if necessary.
fn system_path(
    project_root: &Path,
    id: FileId,
    package_storage: &PackageStorage,
) -> FileResult<PathBuf> {
    let buf;
    let mut root = project_root;
    if let Some(spec) = id.package() {
        buf = package_storage.prepare_package(spec, &mut NoProgress)?;
        root = &buf;
    }

    id.vpath().resolve(root).ok_or(FileError::AccessDenied)
}

/// Reads a file from a `FileId`.
fn read(
    id: FileId,
    project_root: &Path,
    package_storage: &PackageStorage,
) -> FileResult<Vec<u8>> {
    match id {
        id if id == *EMPTY_ID => Ok(Vec::new()),
        id if id == *STDIN_ID => read_from_stdin(),
        _ => {
            let path = system_path(project_root, id, package_storage).or_else(|e| {
                 // Fallback for missing typst.toml
                 if let Some(path) = id.vpath().resolve(project_root) {
                     if path.ends_with("typst.toml") && !path.exists() {
                         return Ok(path);
                     }
                 }
                 Err(e)
            })?;
            
            if path.ends_with("typst.toml") && !path.exists() {
                 return Ok(b"[package]\nname = \"tola-project\"\nversion = \"0.0.0\"\nentrypoint = \"content/index.typ\"".to_vec());
            }
            
            read_from_disk(&path)
        }
    }
}

fn read_from_disk(path: &Path) -> FileResult<Vec<u8>> {
    let f = |e| FileError::from_io(e, path);
    if fs::metadata(path).map_err(f)?.is_dir() {
        Err(FileError::IsDirectory)
    } else {
        fs::read(path).map_err(f)
    }
}

fn read_from_stdin() -> FileResult<Vec<u8>> {
    let mut buf = Vec::new();
    let result = io::stdin().read_to_end(&mut buf);
    match result {
        Ok(_) => (),
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => (),
        Err(err) => return Err(FileError::from_io(err, Path::new("<stdin>"))),
    }
    Ok(buf)
}

fn decode_utf8(buf: &[u8]) -> FileResult<&str> {
    Ok(std::str::from_utf8(buf.strip_prefix(b"\xef\xbb\xbf").unwrap_or(buf))?)
}

enum Now {
    System(OnceLock<DateTime<Utc>>),
}

struct NoProgress;

impl Progress for NoProgress {
    fn print_start(&mut self) {}
    fn print_progress(&mut self, _state: &DownloadState) {}
    fn print_finish(&mut self, _state: &DownloadState) {}
}

/// Compile a Typst file to HTML string
pub fn compile_to_html(path: &Path, root: &Path) -> anyhow::Result<String> {
    let world = SystemWorld::new(path, root)?;

    // 1. Compile to Document
    let document = typst::compile(&world)
        .output
        .map_err(|diags| anyhow::anyhow!("Typst compilation failed: {:?}", diags))?;

    // 2. Export to HTML
    let html = typst_html::html(&document)
        .map_err(|e| anyhow::anyhow!("HTML export failed: {:?}", e))?;

    Ok(html)
}
