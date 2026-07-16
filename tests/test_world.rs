use wasm_bindgen::JsValue;
use wasm_typst::{
    Diagnostic, DiagnosticSeverity, FileInput, FontInput, SourceInput, TooltipKind, WasmWorld,
};
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
    world.compile(JsValue::NULL, None);
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
    let errors = world.compile(JsValue::NULL, None);
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
    world.compile(JsValue::NULL, None);
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
    world.compile(JsValue::NULL, None);
    assert_eq!(world.get_page_count(), 2);
}

#[wasm_bindgen_test]
fn page_size_of_a4() {
    let mut world = WasmWorld::new();
    world.set_sources_and_files(
        vec![SourceInput::new(String::from("main.typ"), String::from("Hello world"))],
        vec![],
    );
    world.compile(JsValue::NULL, None);
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
    let errors = world.compile(JsValue::NULL, None);
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
    let diags: Vec<Diagnostic> = world.compile(JsValue::NULL, None);
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
    let diags = world.compile(JsValue::NULL, None);
    assert!(diags.is_empty(), "expected clean compile, got: {diags:?}");
}

#[wasm_bindgen_test]
fn compile_uses_passed_now_for_today() {
    let mut world = WasmWorld::new();
    world.set_sources_and_files(
        vec![SourceInput::new(
            String::from("main.typ"),
            String::from("#assert(datetime.today() == datetime(year: 2024, month: 6, day: 15))"),
        )],
        vec![],
    );
    // 2024-06-15T12:00:00Z
    let errors = world.compile(JsValue::NULL, Some(1_718_452_800_000.0));
    assert!(errors.is_empty(), "today() should be 2024-06-15, got: {errors:?}");
}

#[wasm_bindgen_test]
fn today_respects_utc_offset() {
    let mut world = WasmWorld::new();
    world.set_sources_and_files(
        vec![SourceInput::new(
            String::from("main.typ"),
            String::from("#assert(datetime.today(offset: 2) == datetime(year: 2024, month: 6, day: 16))"),
        )],
        vec![],
    );
    // 2024-06-15T23:00:00Z; UTC+2 is already past midnight.
    let errors = world.compile(JsValue::NULL, Some(1_718_492_400_000.0));
    assert!(errors.is_empty(), "today(offset: 2) should be 2024-06-16, got: {errors:?}");
}

#[wasm_bindgen_test]
fn today_without_now_is_unavailable() {
    let mut world = WasmWorld::new();
    world.set_sources_and_files(
        vec![SourceInput::new(
            String::from("main.typ"),
            String::from("#datetime.today()"),
        )],
        vec![],
    );
    let errors = world.compile(JsValue::NULL, None);
    assert_eq!(errors.len(), 1, "today() must error without a clock, got: {errors:?}");
}

#[wasm_bindgen_test]
fn render_png_page() {
    let mut world = WasmWorld::new();
    assert!(world.render_png(0, 1.0).is_none(), "no document yet");
    world.set_fonts(load_fonts());
    world.set_sources_and_files(
        vec![SourceInput::new(String::from("main.typ"), String::from("= Hello world"))],
        vec![],
    );
    let errors = world.compile(JsValue::NULL, None);
    assert!(errors.is_empty(), "compile should be clean, got: {errors:?}");
    let png = world.render_png(0, 1.0).expect("page 0 should render");
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "PNG magic bytes");
    assert!(world.render_png(5, 1.0).is_none(), "page out of bounds");
}

#[wasm_bindgen_test]
fn autocomplete_suggests_functions() {
    let mut world = WasmWorld::new();
    world.set_sources_and_files(
        vec![SourceInput::new(String::from("main.typ"), String::from("#te"))],
        vec![],
    );
    // Errors are fine; completion works on the sources regardless.
    world.compile(JsValue::NULL, None);
    let result = world
        .autocomplete(String::from("main.typ"), 3, true)
        .expect("completions after `#te`");
    assert!(result.from <= 3, "replacement starts at or before the cursor");
    assert!(
        result.completions.iter().any(|c| c.label == "text"),
        "expected `text` in completions, got: {:?}",
        result.completions.iter().map(|c| &c.label).collect::<Vec<_>>(),
    );
    assert!(world.autocomplete(String::from("nope.typ"), 0, true).is_none(), "unknown file");
}

#[wasm_bindgen_test]
fn tooltip_shows_binding_value() {
    let mut world = WasmWorld::new();
    world.set_sources_and_files(
        vec![SourceInput::new(String::from("main.typ"), String::from("#let x = 5\n#x"))],
        vec![],
    );
    let errors = world.compile(JsValue::NULL, None);
    assert!(errors.is_empty(), "compile should be clean, got: {errors:?}");
    // Hover over the `x` in `#x` (byte 13 is just after it).
    let tooltip = world
        .tooltip(String::from("main.typ"), 13)
        .expect("tooltip for a bound variable");
    assert_eq!(tooltip.kind, TooltipKind::Code);
    assert_eq!(tooltip.text, "5");
    assert!(world.tooltip(String::from("nope.typ"), 0).is_none(), "unknown file");
}

#[wasm_bindgen_test]
fn jump_from_cursor_finds_preview_position() {
    let mut world = WasmWorld::new();
    world.set_fonts(load_fonts());
    world.set_sources_and_files(
        vec![SourceInput::new(String::from("main.typ"), String::from("= Hello world"))],
        vec![],
    );
    let errors = world.compile(JsValue::NULL, None);
    assert!(errors.is_empty(), "compile should be clean, got: {errors:?}");
    // Cursor inside "Hello".
    let positions = world.jump_from_cursor(String::from("main.typ"), 3);
    assert_eq!(positions.len(), 1, "one position for the heading");
    let pos = &positions[0];
    assert_eq!(pos.page, 0);
    // Inside an A4 page, past the 2.5cm default margins.
    assert!(pos.x >= 25.0 && pos.x <= 210.0, "x was {}", pos.x);
    assert!(pos.y >= 25.0 && pos.y <= 297.0, "y was {}", pos.y);
    assert!(world.jump_from_cursor(String::from("nope.typ"), 0).is_empty(), "unknown file");
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
    let errors = world.compile(JsValue::NULL, None);
    assert!(errors.is_empty(), "reading data.txt should be clean, got: {errors:?}");
}
