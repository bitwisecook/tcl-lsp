//! Differential parity for the graph's **pilot value-spec edge path**
//! (`references_via_spec` dispatch), increment 1: the `ListSpec(ObjectRefSpec)`
//! family (`rules` / `policies` / `vlans` / firewall `rule-lists` /
//! `address-lists`).
//!
//! The fixture exercises `policies` and `vlans` on an `ltm virtual` — whose
//! *legacy* references are empty (migrated to the pilot specs), so those edges
//! come ONLY from the pilot path. The golden is captured from Python with
//! exactly these pilot specs enabled and the iRule path disabled. Self-contained.

use std::collections::BTreeSet;

use tcl_bigip::graph::{build_bigip_object_graph, GraphContext};
use tcl_bigip::parser::parse_bigip_conf;

#[test]
fn graph_pilot_objectref_edges_match_python() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let source = std::fs::read_to_string(format!("{dir}/graph_pilot.conf")).expect("read config");
    let golden =
        std::fs::read_to_string(format!("{dir}/graph_pilot_objectref.golden.tsv")).expect("golden");
    let drift =
        std::fs::read_to_string(format!("{dir}/graph_pilot_objectref.drift.txt")).expect("drift");

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

    // Every Python edge (pilot + legacy) is produced — including the pilot-only
    // `policies`→`ltm_policy` / `vlans`→`net_vlan` edges legacy can't emit.
    let missing: Vec<&&str> = wset.difference(&gset).collect();
    assert!(missing.is_empty(), "missing Python edges: {missing:?}");

    // The only extras are the frozen registry-data drift: Rust's legacy *section*
    // refs for the migrated `policies`/`vlans` weren't cleared, so it emits the
    // `[]` variants Python doesn't.
    let extra: BTreeSet<&str> = gset.difference(&wset).copied().collect();
    assert_eq!(extra, drift_set, "extra pilot-fixture edges changed");

    // Order matches Python once the drift edges are removed.
    let got_no_drift: Vec<&str> = got
        .iter()
        .map(String::as_str)
        .filter(|e| !drift_set.contains(e))
        .collect();
    let want_order: Vec<&str> = golden.lines().collect();
    assert_eq!(
        got_no_drift, want_order,
        "pilot edge order differs from Python"
    );
}
