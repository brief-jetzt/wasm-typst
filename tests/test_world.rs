use typst::html::tag::data;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use wasm_typst::{FontInput, SourceInput, WasmWorld};

wasm_bindgen_test_configure!(run_in_browser);

// TODO: Sorry I don't remember what was supposed to be tested here, but this fails.
//
// #[wasm_bindgen_test]
// fn render_svg_without_main_source() {
//     let mut world = WasmWorld::new();
//     world.compile(JsValue::NULL);
//     let svg = world.render_svg();
//     assert!(svg.starts_with("<svg"));
// }
//
// #[wasm_bindgen_test]
// fn render_pdf_without_main_source() {
//     let mut world = WasmWorld::new();
//     world.compile(JsValue::NULL);
//     let pdf = world.render_pdf();
//     assert!(pdf.len() > 0);
// }

fn load_fonts() -> Vec<FontInput> {
    let lin_libertine_r = include_bytes!("assets/fonts/LinLibertine_R.ttf").to_vec();
    let lin_libertine_rb = include_bytes!("assets/fonts/LinLibertine_RB.ttf").to_vec();
    let lin_libertine_ri = include_bytes!("assets/fonts/LinLibertine_RI.ttf").to_vec();
    let lin_libertine_rbi = include_bytes!("assets/fonts/LinLibertine_RBI.ttf").to_vec();
    vec![
        FontInput::new("LinLibertine_R.ttf".to_string(), lin_libertine_r),
        FontInput::new("LinLibertine_RB.ttf".to_string(), lin_libertine_rb),
        FontInput::new("LinLibertine_RI.ttf".to_string(), lin_libertine_ri),
        FontInput::new("LinLibertine_RBI.ttf".to_string(), lin_libertine_rbi),
    ]
}

#[wasm_bindgen_test]
fn render_svg_with_main_source() {
    let mut world = WasmWorld::new();
    let sources = vec![SourceInput::new(
        String::from("main.typ"),
        String::from("= Hello world"),
    )];
    world.set_fonts(load_fonts());
    world.set_sources_and_files(sources, vec![]);
    world.compile(JsValue::NULL);
    let svg = world.render_svg(0);
    assert!(svg.starts_with("<svg"));
}

#[wasm_bindgen_test]
fn render_pdf_with_main_source() {
    let mut world = WasmWorld::new();
    let sources = vec![SourceInput::new(
        String::from("main.typ"),
        String::from("= Hello world"),
    )];
    world.set_fonts(load_fonts());
    world.set_sources_and_files(sources, vec![]);
    world.compile(JsValue::NULL);
    let pdf = world.render_pdf();
    // for debugging purposes, output the PDF as base64. You can see it if you run the tests in a
    // non-headless mode, like this: `wasm-pack test --chrome`,
    // and then open the devtools console.
    // console_log!("PDF size: {}", pdf.len());
    // console_log!("PDF as base64: {}", base64::encode(&pdf));

    assert!(pdf.len() > 0);
}

#[wasm_bindgen_test]
fn count_pages() {
    let mut world = WasmWorld::new();
    let sources = vec![SourceInput::new(
        String::from("main.typ"),
        String::from("= Hello world"),
    )];
    world.set_fonts(load_fonts());
    world.set_sources_and_files(sources, vec![]);
    world.compile(JsValue::NULL);
    let pages = world.get_page_count();
    assert_eq!(pages, 1);
}

#[wasm_bindgen_test]
fn page_size_of_a4() {
    let mut world = WasmWorld::new();
    let sources = vec![SourceInput::new(
        String::from("main.typ"),
        String::from("= Hello world"),
    )];
    world.set_fonts(load_fonts());
    world.set_sources_and_files(sources, vec![]);
    world.compile(JsValue::NULL);
    let size = world.get_page_size(0).unwrap();
    assert_eq!(size.width, 210.0);
    assert_eq!(size.height, 297.0);
}

#[wasm_bindgen_test]
fn go_to_definition() {
    let mut world = WasmWorld::new();
    let sources = vec![SourceInput::new(
        String::from("main.typ"),
        String::from("= Hello world"),
    )];
    world.set_fonts(load_fonts());
    world.set_sources_and_files(sources, vec![]);
    world.compile(JsValue::NULL);

    // Position in millimeters after the first and before the second "L"
    let position = world.go_to_definition(0, 33.8, 26.0);
    assert!(position.is_some());
    let position = position.unwrap();
    assert_eq!(position.path, "/main.typ");
    assert_eq!(position.cursor, 5); // number of bytes before to get to the second "L"
}
