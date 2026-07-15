use wasm_bindgen::JsValue;
use wasm_typst::{FileInput, SourceInput, WasmWorld};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn render_svg_with_main_source() {
    let mut world = WasmWorld::new();
    let sources = vec![SourceInput::new(String::from("main.typ"), String::from("Hello world"))];
    world.set_sources_and_files(sources, vec![]);
    world.compile(JsValue::NULL);
    let svg = world.render_svg();
    assert!(svg.starts_with("<svg"));
}

#[wasm_bindgen_test]
fn update_source_is_reflected() {
    let mut world = WasmWorld::new();
    world.set_sources_and_files(
        vec![SourceInput::new(String::from("main.typ"), String::from("First"))],
        vec![],
    );
    // Incrementally edit the main source, then recompile.
    world.update_source(String::from("main.typ"), String::from("= Second heading"));
    let errors = world.compile(JsValue::NULL);
    assert_eq!(errors, "", "compile should be clean, got: {errors}");
    assert!(world.render_svg().starts_with("<svg"));
}

#[wasm_bindgen_test]
fn render_svg_pages_returns_one_per_page() {
    let mut world = WasmWorld::new();
    world.set_sources_and_files(
        vec![SourceInput::new(
            String::from("main.typ"),
            String::from("First page #pagebreak() Second page"),
        )],
        vec![],
    );
    world.compile(JsValue::NULL);
    let pages = world.render_svg_pages();
    assert_eq!(pages.len(), 2);
    assert!(pages.iter().all(|p| p.starts_with("<svg")));
}

#[wasm_bindgen_test]
fn reading_a_binary_file_works() {
    // Regression test: World::file() used to panic (todo!()). A `read` of a
    // file registered via update_file must now succeed.
    let mut world = WasmWorld::new();
    world.set_sources_and_files(
        vec![SourceInput::new(
            String::from("main.typ"),
            String::from("#read(\"data.txt\")"),
        )],
        vec![FileInput::new(String::from("data.txt"), b"hello".to_vec())],
    );
    let errors = world.compile(JsValue::NULL);
    assert_eq!(errors, "", "reading data.txt should be clean, got: {errors}");
}
