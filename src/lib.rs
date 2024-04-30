mod utils;

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use comemo::Prehashed;
use typst::{Library, LibraryBuilder, World};
use typst::diag::{FileError, FileResult};
use typst::eval::Tracer;
use typst::foundations::{Bytes, Datetime, Dict, Smart, Str, Value};
use typst::model::Document;
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use wasm_bindgen::prelude::*;

// #[wasm_bindgen]
// extern "C" {
//     fn alert(s: &str);
// }

#[wasm_bindgen(js_name = World)]
pub struct WasmWorld {
    library: Prehashed<Library>,
    book: Prehashed<FontBook>,
    fonts: Vec<FontSlot>,
    slots: Mutex<HashMap<FileId, FileSlot>>,
    // used to store the compiled document, so that we are able to
    // return compiler warnings in the .compile() method
    document: Option<Document>,
}

struct FileSlot {
    _id: FileId,
    source: Source,
    _file: Bytes,
}

struct FontSlot {
    path: PathBuf,
    index: u32,
    font: OnceLock<Option<Font>>,
}

impl FontSlot {
    fn get(&self) -> Option<Font> {
        self.font
            .get_or_init(|| {
                let data = fs::read(&self.path).ok()?.into();
                Font::new(data, self.index)
            })
            .clone()
    }
}

#[wasm_bindgen(js_class = World)]
impl WasmWorld {
    pub fn new() -> Self {
        Self {
            library: Prehashed::new(Library::builder().build()),
            book: Prehashed::new(FontBook::new()),
            fonts: Vec::new(),
            slots: Mutex::new(HashMap::new()),
            document: None,
        }
    }

    pub fn set_inputs(&mut self, inputs: JsValue) {
        // TODO: proper typing for JsValue
        let inputs: HashMap<String, String> = serde_wasm_bindgen::from_value(inputs).unwrap_or(HashMap::new());
        let mut dict = Dict::new();
        for (key, value) in inputs {
            dict.insert(Str::from(key), Value::Str(Str::from(value)));
        }
        self.library = Prehashed::new(LibraryBuilder::default().with_inputs(dict).build());
    }

    pub fn add_font(&mut self, path: String, data: Vec<u8>) {
        let buffer = typst::foundations::Bytes::from(data);
        let mut book = self.book.clone().into_inner();
        for (i, font) in Font::iter(buffer).enumerate() {
            book.push(font.info().clone());
            self.fonts.push(FontSlot {
                path: PathBuf::from(&path),
                index: i as u32,
                font: OnceLock::from(Some(font)),
            });
        }
        self.book = Prehashed::new(book);
    }

    fn add_file_slot(&self, path: String, source: String, data: Vec<u8>) {
        let file_id = FileId::new(None, VirtualPath::new(path));
        let slot = FileSlot {
            _id: file_id,
            source: Source::new(file_id, source),
            _file: Bytes::from(data),
        };
        let mut slots = self.slots.lock().unwrap();
        slots.insert(file_id, slot);
    }

    pub fn add_source(&self, path: String, source: String) {
        self.add_file_slot(path, source, Vec::new());
    }

    pub fn add_file(&self, path: String, data: Vec<u8>) {
        self.add_file_slot(path, String::new(), data);
    }

    pub fn compile(&mut self) -> String {
        let mut tracer = Tracer::new();
        match typst::compile(self, &mut tracer) {
            Ok(document) => {
                self.document = Some(document);
                let warnings = tracer.warnings();
                let mut res = String::new();
                for warning in warnings {
                    res.push_str(&format!("{:?}\n", warning));
                }
                res
            },
            Err(e) => {
                let mut res = String::new();
                res.push_str(&format!("{:?}\n", e));
                res
            }
        }
    }

    pub fn render_pdf(&self) -> Vec<u8> {
        match self.document {
            Some(ref document) => typst_pdf::pdf(document, Smart::Auto, now()),
            None => Vec::new()
        }
    }

    pub fn render_svg(&self) -> String {
        match self.document {
            Some(ref document) => {
                // TODO: Replace svg_merged by something where we can tell the pages apart
                typst_svg::svg_merged(document, typst::layout::Abs::pt(5.0))
            },
            None => {
                String::from("<pre class=\"typst-render-error\">No document</pre>".to_string())
            }
        }
    }
}

impl World for WasmWorld {
    fn library(&self) -> &Prehashed<Library> {
        &self.library
    }

    fn book(&self) -> &Prehashed<FontBook> {
        &self.book
    }

    fn main(&self) -> Source {
        let file_id = FileId::new(None, VirtualPath::new("main.typ"));
        match self.source(file_id) {
            Ok(source) => source,
            Err(_) => Source::new(file_id, String::from("= Error!\nCould not find main.typ file."))
        }
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        let slot = self.slots.lock().unwrap();
        // let file_slot = slot.get(&id).unwrap();
        match slot.get(&id) {
            Some(file_slot) => Ok(file_slot.source.clone()),
            None => {
                let file_path = id.vpath().as_rooted_path();
                Err(FileError::NotFound(PathBuf::from(file_path)))
            }
        }
        // Ok(file_slot.source.clone())
//         let text = String::from("= Hello world
// This is an *awesome* example _document_.
//         ");
//         let source = Source::new(id, String::from(text));
//         Ok(source)
    }

    fn file(&self, _id: FileId) -> FileResult<Bytes> {
        todo!()
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts[index].get()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        Datetime::from_ymd(1970, 1, 1)
    }
}

fn now() -> Option<Datetime> {
    Datetime::from_ymd_hms(2000, 1, 1, 0, 0, 0)
}

pub fn render_pdf() -> Vec<u8> {
    let world = &WasmWorld::new();
    let mut tracer = Tracer::new();
    let document = typst::compile(world, &mut tracer).unwrap();
    // let warnings = tracer.warnings();
    typst_pdf::pdf(&document, Smart::Auto, now())
    // let document = compile();
    //
    // typst::compile(world, &mut tracer).unwrap();
    // typst_pdf::pdf(&document, Smart::Auto, world.today(Some(0)))
}