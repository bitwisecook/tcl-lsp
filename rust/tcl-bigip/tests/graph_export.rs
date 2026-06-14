//! Byte-parity for `export_graph` (DOT / JSON / Mermaid) against the Python
//! `graph_export.export_graph`, on a drift-free fixture (so the Rust and Python
//! edge sets are identical and the serialised text matches exactly).

use tcl_bigip::graph::{build_bigip_object_graph, export_graph, GraphContext};
use tcl_bigip::parser::parse_bigip_conf;

#[test]
fn graph_export_matches_python() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let source = std::fs::read_to_string(format!("{dir}/graph_export.conf")).expect("read config");
    let cfg = parse_bigip_conf(&source, "Common");
    let uri = "test://config".to_owned();
    let sources = [(uri.clone(), source)];
    let configs = [(uri, &cfg)];

    let ctx = GraphContext::new();
    let graph = build_bigip_object_graph(&sources, &configs, &ctx);

    for fmt in ["dot", "json", "mermaid"] {
        let want = std::fs::read_to_string(format!("{dir}/graph_export.{fmt}.golden"))
            .expect("read golden");
        let export = export_graph(&graph, fmt, &[], false, None).expect("export");
        assert_eq!(export.text, want, "{fmt} export differs from Python");
        assert_eq!(export.node_count, 5, "{fmt} node_count");
        assert_eq!(export.edge_count, 8, "{fmt} edge_count");
    }
}

#[test]
fn graph_export_rejects_unknown_format() {
    let ctx = GraphContext::new();
    let graph = build_bigip_object_graph(&[], &[], &ctx);
    assert!(export_graph(&graph, "svg", &[], false, None).is_err());
}
