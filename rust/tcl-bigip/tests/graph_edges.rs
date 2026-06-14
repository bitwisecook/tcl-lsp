//! Differential parity for the **complete** BIG-IP graph edge walk on a
//! realistic `bigip.conf` against `_build_forward_edges` — the pilot value-spec
//! dispatch, the legacy token-scan fallback, AND the iRule body walker.
//! Companion to `graph_pilot.rs`, which exercises the compound pilot specs on a
//! synthetic fixture; this pins the full walk on real config. Self-contained —
//! no Python at test time.
//!
//! The Rust walk reproduces every Python edge, plus a small frozen set of
//! *extra* edges (`graph_edges.drift.txt`) caused by registry-**data** drift:
//! properties Python migrated to the pilot specs (so their *legacy* `references`
//! were cleared, e.g. `persist` on `ltm virtual`) still carry those refs in the
//! generated Rust registry data. That is a registry-data-regen workstream, not
//! a walk-logic bug; this test pins the set so a real logic regression fails.

use std::collections::BTreeSet;

use tcl_bigip::graph::{build_bigip_object_graph, GraphContext};
use tcl_bigip::parser::parse_bigip_conf;

#[test]
fn graph_edges_match_python() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let source = std::fs::read_to_string(format!("{dir}/bigip.conf")).expect("read config");
    let golden = std::fs::read_to_string(format!("{dir}/graph_edges.golden.tsv")).expect("golden");
    let drift = std::fs::read_to_string(format!("{dir}/graph_edges.drift.txt")).expect("drift");

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

    let gset: BTreeSet<&str> = got.iter().map(String::as_str).collect();
    let wset: BTreeSet<&str> = golden.lines().collect();
    let drift_set: BTreeSet<&str> = drift.lines().filter(|l| !l.trim().is_empty()).collect();

    // Every Python legacy edge must be produced.
    let missing: Vec<&&str> = wset.difference(&gset).collect();
    assert!(
        missing.is_empty(),
        "Rust is missing Python legacy edges: {missing:?}"
    );

    // The only extra Rust edges are the frozen registry-data drift set.
    let extra: BTreeSet<&str> = gset.difference(&wset).copied().collect();
    assert_eq!(
        extra, drift_set,
        "extra legacy edges changed — a walk-logic regression or a registry-data shift"
    );

    // Ordering: with the drift edges removed, the Rust order matches Python's.
    let got_no_drift: Vec<&str> = got
        .iter()
        .map(String::as_str)
        .filter(|e| !drift_set.contains(e))
        .collect();
    let want_order: Vec<&str> = golden.lines().collect();
    assert_eq!(
        got_no_drift, want_order,
        "legacy edge order differs from Python"
    );
}
