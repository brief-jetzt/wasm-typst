mod utils;

use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use comemo::Prehashed;
use typst::{Library, World};
use typst::diag::FileResult;
use typst::eval::Tracer;
use typst::foundations::{Bytes, Datetime, Smart};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use wasm_bindgen::prelude::*;

// #[wasm_bindgen]
// extern "C" {
//     fn alert(s: &str);
// }

#[wasm_bindgen(js_name = World)]
pub struct WasmWorld {
    /// Typst's standard library.
    library: Prehashed<Library>,
    book: Prehashed<FontBook>,
    fonts: Vec<FontSlot>,
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
        }
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

    pub fn render_pdf(&self) -> Vec<u8> {
        let mut tracer = Tracer::new();
        let document = typst::compile(self, &mut tracer).unwrap();
        // let warnings = tracer.warnings();
        typst_pdf::pdf(&document, Smart::Auto, now())
        // let document = compile();
        //
        // typst::compile(world, &mut tracer).unwrap();
        // typst_pdf::pdf(&document, Smart::Auto, world.today(Some(0)))
    }

    pub fn render_svg(&self) -> String {
        let mut tracer = Tracer::new();
        let document = typst::compile(self, &mut tracer).unwrap();
        // let warnings = tracer.warnings();
        // TODO: Replace svg_merged by something where we can tell the pages apart
        typst_svg::svg_merged(&document, typst::layout::Abs::pt(5.0))
        // let document = compile();
        //
        // typst::compile(world, &mut tracer).unwrap();
        // typst_pdf::pdf(&document, Smart::Auto, world.today(Some(0)))
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
        self.source(file_id).unwrap()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        let text = String::from("= Hello world
This is an *awesome* example _document_.
        ");
        let source = Source::new(id, String::from(text));
        Ok(source)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
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