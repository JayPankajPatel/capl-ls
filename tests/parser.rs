use capl_ls::model::CaplSymbolKind;
use capl_ls::parser::parse_document;
use capl_ls::preprocessor::{collect_macros, expand_object_like_once};

#[test]
fn parses_core_capl_shapes() {
    let text = include_str!("fixtures/basic.can");
    let parsed = parse_document(text);

    assert!(parsed
        .symbols
        .iter()
        .any(|symbol| symbol.kind == CaplSymbolKind::Include && symbol.name == "common.cin"));
    assert!(parsed
        .symbols
        .iter()
        .any(|symbol| symbol.kind == CaplSymbolKind::Macro && symbol.name == "HEARTBEAT_MS"));
    assert!(parsed
        .symbols
        .iter()
        .any(|symbol| symbol.kind == CaplSymbolKind::Variable && symbol.name == "gCounter"));
    assert!(parsed
        .symbols
        .iter()
        .any(|symbol| symbol.kind == CaplSymbolKind::Function && symbol.name == "helper"));
    assert!(parsed.symbols.iter().any(|symbol| {
        symbol.kind == CaplSymbolKind::EventHandler && symbol.name == "on timer tHeartbeat"
    }));
}

#[test]
fn expands_object_like_macros_once() {
    let text = "#define HEARTBEAT_MS 100\nsetTimer(t, HEARTBEAT_MS);\n";
    let macros = collect_macros(text);
    let expanded = expand_object_like_once("setTimer(t, HEARTBEAT_MS);", &macros);

    assert_eq!(expanded.text, "setTimer(t, 100);");
    assert_eq!(expanded.source_map.len(), 1);
}
