use wasm_bindgen::JsValue;
use wasm_typst::{SourceInput, WasmWorld};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn render_svg_without_main_source() {
    let mut world = WasmWorld::new();
    world.compile(JsValue::NULL);
    let svg = world.render_svg();
    assert!(svg.starts_with("<svg"));
}

#[wasm_bindgen_test]
fn render_pdf_without_main_source() {
    let mut world = WasmWorld::new();
    world.compile(JsValue::NULL);
    let pdf = world.render_pdf();
    assert!(pdf.len() > 0);
}

#[wasm_bindgen_test]
fn render_svg_with_main_source() {
    let mut world = WasmWorld::new();
    world.compile(JsValue::NULL);
    let sources = vec![SourceInput::new(String::from("main.typ"), String::from("Hello world"))];
    world.set_sources_and_files(sources, vec![]);
    let svg = world.render_svg();
    assert!(svg.starts_with("<svg"));
}