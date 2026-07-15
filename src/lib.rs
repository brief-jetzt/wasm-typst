mod utils;

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use typst::{Library, LibraryExt, World};
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Dict, Duration, Smart, Str, Value};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
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

    pub fn compile(&mut self, inputs: JsValue) -> String {
        self.set_inputs(inputs);
        let warned = typst::compile::<PagedDocument>(self);
        let res = match warned.output {
            Ok(document) => {
                self.document = Some(document);
                let mut res = String::new();
                for warning in warned.warnings {
                    res.push_str(&format!("{:?}\n", warning));
                }
                res
            }
            Err(e) => format!("{:?}\n", e),
        };
        // Bound the memoization cache. A long-lived World is reused across many
        // renders; without eviction comemo's cache would grow unbounded.
        comemo::evict(10);
        res
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

fn now() -> Option<Timestamp> {
    let datetime = Datetime::from_ymd_hms(2000, 1, 1, 0, 0, 0).unwrap();
    Some(Timestamp::new_utc(datetime))
}
