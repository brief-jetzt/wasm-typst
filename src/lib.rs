mod utils;

use std::collections::HashMap;
use std::fs;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use typst::{Library, LibraryExt, World, WorldExt};
use typst::diag::{FileError, FileResult, Severity, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Dict, Duration, Smart, Str, Value};
use typst::introspection::PagedPosition;
use typst::layout::{Abs, Point};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst_ide::{IdeWorld, Jump, jump_from_click};
use typst_layout::PagedDocument;
use typst_pdf::{PdfOptions, Timestamp};
use typst_svg::SvgOptions;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = World)]
pub struct WasmWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<FontSlot>,
    slots: Mutex<HashMap<FileId, FileSlot>>,
    // used to store the compiled document, so that we are able to
    // return compiler warnings in the .compile() method
    document: Option<PagedDocument>,
}

#[wasm_bindgen]
#[derive(Debug)]
pub struct FontInput {
    path: String,
    data: Vec<u8>,
}

#[wasm_bindgen]
#[derive(Debug)]
pub struct FileInput {
    path: String,
    data: Vec<u8>,
}

#[wasm_bindgen]
#[derive(Debug)]
pub struct SourceInput {
    path: String,
    source: String,
}

/// Where a click in the rendered document points to in the sources.
#[wasm_bindgen]
#[derive(Debug)]
pub struct DefinitionPosition {
    /// Source path without leading slash, e.g. `main.typ` — matches the path
    /// keys passed to `SourceInput`/`updateSource`.
    #[wasm_bindgen(getter_with_clone)]
    pub path: String,
    /// Byte offset into that source.
    pub cursor: usize,
}

/// Severity of a [`Diagnostic`]. Passed as a plain number over the wasm
/// boundary (strings would be copied/decoded on every access).
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error = 0,
    Warning = 1,
}

/// A compiler diagnostic with its source location resolved.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    #[wasm_bindgen(getter_with_clone)]
    pub message: String,
    /// Source path without leading slash, e.g. `main.typ` — matches the path
    /// keys passed to `SourceInput`/`updateSource`. `None` when the diagnostic
    /// is not tied to a file.
    #[wasm_bindgen(getter_with_clone)]
    pub path: Option<String>,
    /// Byte range into that source.
    pub start: Option<usize>,
    pub end: Option<usize>,
    /// 1-based line/column of `start`.
    pub line: Option<usize>,
    pub column: Option<usize>,
    /// Suggestions on how to fix the problem.
    #[wasm_bindgen(getter_with_clone)]
    pub hints: Vec<String>,
}

/// Page dimensions in millimeters.
#[wasm_bindgen]
pub struct PageSize {
    pub width: f64,
    pub height: f64,
}

struct FileSlot {
    source: Source,
    file: Bytes,
}

struct FontSlot {
    path: PathBuf,
    index: u32,
    font: OnceLock<Option<Font>>,
}

fn file_id(path: &str) -> FileId {
    FileId::new(RootedPath::new(
        VirtualRoot::Project,
        VirtualPath::new(path).expect("invalid path"),
    ))
}

impl FontSlot {
    fn get(&self) -> Option<Font> {
        self.font
            .get_or_init(|| {
                let data = fs::read(&self.path).ok()?;
                Font::new(Bytes::new(data), self.index)
            })
            .clone()
    }
}

#[wasm_bindgen(js_class = FontInput)]
impl FontInput {
    pub fn new(path: String, data: Vec<u8>) -> Self {
        Self { path, data }
    }
}

#[wasm_bindgen(js_class = FileInput)]
impl FileInput {
    pub fn new(path: String, data: Vec<u8>) -> Self {
        Self { path, data }
    }
}

#[wasm_bindgen(js_class = SourceInput)]
impl SourceInput {
    pub fn new(path: String, source: String) -> Self {
        Self { path, source }
    }
}

#[wasm_bindgen(js_class = World)]
impl WasmWorld {
    pub fn new() -> Self {
        Self {
            library: LazyHash::new(Library::builder().build()),
            book: LazyHash::new(FontBook::new()),
            fonts: Vec::new(),
            slots: Mutex::new(HashMap::new()),
            document: None,
        }
    }

    #[wasm_bindgen(js_name = setFonts)]
    pub fn set_fonts(&mut self, fonts: Vec<FontInput>) {
        let mut book = FontBook::new();
        self.fonts = Vec::new();
        for font_input in fonts {
            let buffer = Bytes::new(font_input.data);
            for (i, font) in Font::iter(buffer).enumerate() {
                book.push(font.info().clone());
                self.fonts.push(FontSlot {
                    path: PathBuf::from(&font_input.path),
                    index: i as u32,
                    font: OnceLock::from(Some(font)),
                });
            }
        }
        self.book = LazyHash::new(book);
    }

    #[wasm_bindgen(js_name = setSourcesAndFiles)]
    pub fn set_sources_and_files(&self, sources: Vec<SourceInput>, files: Vec<FileInput>) {
        let mut slots = self.slots.lock().unwrap();
        slots.clear();
        for file in files {
            let id = file_id(&file.path);
            slots.insert(
                id,
                FileSlot {
                    source: Source::new(id, String::new()),
                    file: Bytes::new(file.data),
                },
            );
        }
        for source in sources {
            let id = file_id(&source.path);
            slots.insert(
                id,
                FileSlot {
                    source: Source::new(id, source.source),
                    file: Bytes::new(Vec::new()),
                },
            );
        }
    }

    /// Incrementally update a single source without touching the others.
    ///
    /// Reuses the existing [`Source`] via [`Source::replace`], so typst only
    /// reparses the changed span instead of parsing the whole file from scratch
    /// (see `typst-kit`'s `SlotCell`). Inserts a fresh source if the path is new.
    #[wasm_bindgen(js_name = updateSource)]
    pub fn update_source(&self, path: String, source: String) {
        let id = file_id(&path);
        let mut slots = self.slots.lock().unwrap();
        match slots.get_mut(&id) {
            Some(slot) => {
                slot.source.replace(&source);
            }
            None => {
                slots.insert(
                    id,
                    FileSlot {
                        source: Source::new(id, source),
                        file: Bytes::new(Vec::new()),
                    },
                );
            }
        }
    }

    /// Update a single binary file's bytes, inserting it if the path is new.
    #[wasm_bindgen(js_name = updateFile)]
    pub fn update_file(&self, path: String, data: Vec<u8>) {
        let id = file_id(&path);
        let mut slots = self.slots.lock().unwrap();
        match slots.get_mut(&id) {
            Some(slot) => {
                slot.file = Bytes::new(data);
            }
            None => {
                slots.insert(
                    id,
                    FileSlot {
                        source: Source::new(id, String::new()),
                        file: Bytes::new(data),
                    },
                );
            }
        }
    }

    /// Compile the sources. Returns the diagnostics, errors first (empty on a
    /// clean compile). On a hard error the previously compiled document is kept.
    pub fn compile(&mut self, inputs: JsValue) -> Vec<Diagnostic> {
        self.set_inputs(inputs);
        let warned = typst::compile::<PagedDocument>(self);
        let mut diagnostics = Vec::new();
        match warned.output {
            Ok(document) => self.document = Some(document),
            Err(errors) => diagnostics.extend(errors.iter().map(|d| self.to_diagnostic(d))),
        }
        diagnostics.extend(warned.warnings.iter().map(|d| self.to_diagnostic(d)));
        // Bound the memoization cache. A long-lived World is reused across many
        // renders; without eviction comemo's cache would grow unbounded.
        comemo::evict(10);
        diagnostics
    }

    fn to_diagnostic(&self, diag: &SourceDiagnostic) -> Diagnostic {
        let range = self.range(diag.span);
        let (line, column) = range
            .as_ref()
            .zip(diag.span.id())
            .and_then(|(range, id)| {
                let source = self.source(id).ok()?;
                let (line, column) = source.lines().byte_to_line_column(range.start)?;
                Some((line + 1, column + 1))
            })
            .unzip();
        Diagnostic {
            severity: match diag.severity {
                Severity::Error => DiagnosticSeverity::Error,
                Severity::Warning => DiagnosticSeverity::Warning,
            },
            message: diag.message.to_string(),
            path: diag
                .span
                .id()
                .map(|id| id.vpath().get_without_slash().to_string()),
            start: range.as_ref().map(|r| r.start),
            end: range.as_ref().map(|r| r.end),
            line,
            column,
            hints: diag.hints.iter().map(|hint| hint.v.to_string()).collect(),
        }
    }

    pub fn render_pdf(&self) -> Vec<u8> {
        match self.document {
            Some(ref document) => {
                let options = PdfOptions {
                    ident: Smart::Auto,
                    creator: Smart::Auto,
                    timestamp: now(),
                    page_ranges: None,
                    standards: Default::default(),
                    tagged: true,
                    pretty: false,
                };
                typst_pdf::pdf(document, &options).unwrap_or_default()
            },
            None => Vec::new()
        }
    }

    /// Render all pages merged into a single SVG (pages stacked with a gap).
    /// Use [`render_svg_pages`](Self::render_svg_pages) to keep pages separate.
    pub fn render_svg(&self) -> String {
        match self.document {
            Some(ref document) => {
                typst_svg::svg_merged(document, &SvgOptions::default(), typst::layout::Abs::pt(5.0))
            }
            None => {
                "<pre class=\"typst-render-error\">No document</pre>".to_string()
            }
        }
    }

    /// Render each page to its own SVG string, in page order.
    #[wasm_bindgen(js_name = renderSvgPages)]
    pub fn render_svg_pages(&self) -> Vec<String> {
        match self.document {
            Some(ref document) => document
                .pages()
                .iter()
                .map(|page| typst_svg::svg(page, &SvgOptions::default()))
                .collect(),
            None => Vec::new(),
        }
    }

    /// Number of pages in the compiled document (0 before a successful compile).
    #[wasm_bindgen(js_name = getPageCount)]
    pub fn get_page_count(&self) -> usize {
        self.document.as_ref().map_or(0, |d| d.pages().len())
    }

    /// Size of the given 0-based page in millimeters.
    #[wasm_bindgen(js_name = getPageSize)]
    pub fn get_page_size(&self, page: usize) -> Option<PageSize> {
        let size = self.document.as_ref()?.pages().get(page)?.frame.size();
        Some(PageSize {
            width: size.x.to_mm(),
            height: size.y.to_mm(),
        })
    }

    /// Map a click at (x, y) millimeters from the top-left of the given
    /// 0-based page to a position in the sources ("go to definition").
    /// Returns `None` when the click hits nothing jumpable.
    #[wasm_bindgen(js_name = goToDefinition)]
    pub fn go_to_definition(&self, page: usize, x: f64, y: f64) -> Option<DefinitionPosition> {
        let document = self.document.as_ref()?;
        let position = PagedPosition {
            page: NonZeroUsize::new(page + 1)?,
            point: Point::new(Abs::mm(x), Abs::mm(y)),
        };
        match jump_from_click(self, document, &position)? {
            Jump::File(id, cursor) => Some(DefinitionPosition {
                path: id.vpath().get_without_slash().to_string(),
                cursor,
            }),
            // Clicks on links (Url) or outline/reference targets (Position)
            // are not source jumps.
            Jump::Url(_) | Jump::Position(_) => None,
        }
    }

    fn set_inputs(&mut self, inputs: JsValue) {
        // `inputs` is a plain JS object (Record<string, string>); the typed
        // `Inputs` contract lives in the JS wrapper. Deserialize it into a dict.
        let inputs: HashMap<String, String> = serde_wasm_bindgen::from_value(inputs).unwrap_or(HashMap::new());
        let mut dict = Dict::new();
        for (key, value) in inputs {
            dict.insert(Str::from(key), Value::Str(Str::from(value)));
        }
        self.library = LazyHash::new(Library::builder().with_inputs(dict).build());
    }
}

impl World for WasmWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        file_id("main.typ")
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        let slot = self.slots.lock().unwrap();
        // let file_slot = slot.get(&id).unwrap();
        match slot.get(&id) {
            Some(file_slot) => Ok(file_slot.source.clone()),
            None => {
                let file_path = id.vpath().get_with_slash();
                Err(FileError::NotFound(PathBuf::from(file_path)))
            }
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let slots = self.slots.lock().unwrap();
        match slots.get(&id) {
            Some(slot) => Ok(slot.file.clone()),
            None => Err(FileError::NotFound(PathBuf::from(id.vpath().get_with_slash()))),
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts[index].get()
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        Datetime::from_ymd(1970, 1, 1)
    }
}

impl IdeWorld for WasmWorld {
    fn upcast(&self) -> &dyn World {
        self
    }
}

fn now() -> Option<Timestamp> {
    let datetime = Datetime::from_ymd_hms(2000, 1, 1, 0, 0, 0).unwrap();
    Some(Timestamp::new_utc(datetime))
}
