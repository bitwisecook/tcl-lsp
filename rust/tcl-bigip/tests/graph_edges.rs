//! Differential parity for the **complete** BIG-IP graph edge walk on a
//! realistic `bigip.conf` against `_build_forward_edges` — the pilot value-spec
//! dispatch, the legacy token-scan fallback, AND the iRule body walker.
//! Companion to `graph_pilot.rs`, which exercises the compound pilot specs on a
//! synthetic fixture; this pins the full walk on real config. The Rust graph now
//! reproduces the expected edge set exactly — the registry-data regen
//! cleared the former drift. Self-contained — no external oracle at
//! test time.

use tcl_bigip::graph::{GraphContext, build_bigip_object_graph};
use tcl_bigip::parser::parse_bigip_conf;

#[test]
fn graph_edges_match_python() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let source = std::fs::read_to_string(format!("{dir}/bigip.conf")).expect("read config");
    let golden = std::fs::read_to_string(format!("{dir}/graph_edges.golden.tsv")).expect("golden");

    let cfg = parse_bigip_conf(&source, "Common");
    let uri = "test://config".to_owned();
    let sources = [(uri.clone(), source)];
    let configs = [(uri, &cfg)];

    let ctx = GraphContext::new();
    let graph = build_bigip_object_graph(&sources, &configs, &ctx);

    let got: Vec<String> = graph
        .edges
        .iter()
        .map(|e| {
            format!(
                "{}\t{}\t{}\t{}",
                e.source_id, e.target_id, e.via_property, e.via_kind
            )
        })
        .collect();
    let want: Vec<&str> = golden.lines().collect();

    // Exact ordered parity — the registry-data regen cleared the former drift.
    assert_eq!(got, want, "graph edges differ from the expected set");
}
