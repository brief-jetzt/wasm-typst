use wasm_bindgen::JsValue;
use wasm_typst::{Diagnostic, DiagnosticSeverity, FileInput, FontInput, SourceInput, WasmWorld};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// Bundled Libertinus fonts, for tests that need laid-out glyphs.
fn load_fonts() -> Vec<FontInput> {
    typst_assets::fonts()
        .enumerate()
        .map(|(i, data)| FontInput::new(format!("font-{i}.ttf"), data.to_vec()))
        .collect()
}

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
    assert!(errors.is_empty(), "compile should be clean, got: {errors:?}");
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
fn count_pages() {
    let mut world = WasmWorld::new();
    assert_eq!(world.get_page_count(), 0, "no document yet");
    world.set_sources_and_files(
        vec![SourceInput::new(
            String::from("main.typ"),
            String::from("First page #pagebreak() Second page"),
        )],
        vec![],
    );
    world.compile(JsValue::NULL);
    assert_eq!(world.get_page_count(), 2);
}

#[wasm_bindgen_test]
fn page_size_of_a4() {
    let mut world = WasmWorld::new();
    world.set_sources_and_files(
        vec![SourceInput::new(String::from("main.typ"), String::from("Hello world"))],
        vec![],
    );
    world.compile(JsValue::NULL);
    let size = world.get_page_size(0).unwrap();
    // pt <-> mm round-trip is not exact, so no float equality.
    assert!((size.width - 210.0).abs() < 0.01, "width was {}", size.width);
    assert!((size.height - 297.0).abs() < 0.01, "height was {}", size.height);
    assert!(world.get_page_size(1).is_none(), "page out of bounds");
}

#[wasm_bindgen_test]
fn go_to_definition() {
    let mut world = WasmWorld::new();
    let source = String::from("= Hello world");
    world.set_fonts(load_fonts());
    world.set_sources_and_files(
        vec![SourceInput::new(String::from("main.typ"), source.clone())],
        vec![],
    );
    let errors = world.compile(JsValue::NULL);
    assert!(errors.is_empty(), "compile should be clean, got: {errors:?}");

    // Click inside the heading text. A4 default margins are 2.5cm, so the
    // first line starts just below/right of (25mm, 25mm).
    let position = world
        .go_to_definition(0, 35.0, 27.0)
        .expect("click inside heading should resolve to a source position");
    assert_eq!(position.path, "main.typ");
    assert!(
        position.cursor < source.len(),
        "cursor {} out of range",
        position.cursor
    );

    // Out-of-bounds page never resolves.
    assert!(world.go_to_definition(5, 35.0, 27.0).is_none());
}

#[wasm_bindgen_test]
fn compile_returns_structured_diagnostics() {
    let mut world = WasmWorld::new();
    // Line 2, unknown variable -> one error diagnostic.
    world.set_sources_and_files(
        vec![SourceInput::new(
            String::from("main.typ"),
            String::from("Hello\n#foobar()"),
        )],
        vec![],
    );
    let diags: Vec<Diagnostic> = world.compile(JsValue::NULL);
    assert_eq!(diags.len(), 1, "expected one error, got: {diags:?}");
    let d = &diags[0];
    assert_eq!(d.severity, DiagnosticSeverity::Error);
    assert!(d.message.contains("unknown variable"), "message was: {}", d.message);
    assert_eq!(d.path.as_deref(), Some("main.typ"));
    assert_eq!(d.line, Some(2));
    assert_eq!(d.column, Some(2), "column points at `foobar` after the `#`");
    let (start, end) = (d.start.unwrap(), d.end.unwrap());
    assert_eq!(&"Hello\n#foobar()"[start..end], "foobar");
}

#[wasm_bindgen_test]
fn clean_compile_returns_no_diagnostics() {
    let mut world = WasmWorld::new();
    world.set_sources_and_files(
        vec![SourceInput::new(String::from("main.typ"), String::from("Hello"))],
        vec![],
    );
    let diags = world.compile(JsValue::NULL);
    assert!(diags.is_empty(), "expected clean compile, got: {diags:?}");
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
    assert!(errors.is_empty(), "reading data.txt should be clean, got: {errors:?}");
}
