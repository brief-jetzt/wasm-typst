use wasm_typst::WasmWorld;

#[test]
fn render_svg_without_main_source() {
    let mut world = WasmWorld::new();
    world.compile();
    let svg = world.render_svg();
    assert!(svg.starts_with("<svg"));
}

#[test]
fn render_pdf_without_main_source() {
    let mut world = WasmWorld::new();
    world.compile();
    let pdf = world.render_pdf();
    assert!(pdf.len() > 0);
}

#[test]
fn render_svg_with_main_source() {
    let mut world = WasmWorld::new();
    world.compile();
    world.add_source(String::from("main.typ"), String::from("Hello world"));
    let svg = world.render_svg();
    assert!(svg.starts_with("<svg"));
}