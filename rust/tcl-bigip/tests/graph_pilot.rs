//! Differential parity for the graph's **pilot value-spec edge path**
//! (`references_via_spec` dispatch) across every reference-producing pilot spec:
//! `ListSpec(ObjectRefSpec)` (`rules`/`policies`/`vlans`/firewall lists),
//! `Profile`/`Persistence` attachments, `MonitorExpressionSpec`, `SnatModeSpec`,
//! `CertKeyChainSpec`, and `FirewallRuleSpec`.
//!
//! The fixture exercises pilot-only edges (`policies`/`vlans`/`persist`, whose
//! legacy references are cleared) plus the compound specs (monitor min-of, SNAT
//! pool, cert-key-chain, firewall source/destination lists). The Rust graph
//! reproduces the expected edge set exactly — the registry-data regen cleared the
//! former drift. Self-contained — no external oracle at test time.

use tcl_bigip::graph::{GraphContext, build_bigip_object_graph};
use tcl_bigip::parser::parse_bigip_conf;

#[test]
fn graph_pilot_edges_match_python() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let source = std::fs::read_to_string(format!("{dir}/graph_pilot.conf")).expect("read config");
    let golden = std::fs::read_to_string(format!("{dir}/graph_pilot.golden.tsv")).expect("golden");

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

    // Exact ordered parity — including the pilot-only `policies`/`vlans`/`persist`
    // edges legacy can't emit, with no drift edges after the registry-data regen.
    assert_eq!(got, want, "pilot graph edges differ from the expected set");
    assert!(
        got.iter()
            .any(|e| e.contains("\tpersist\tltm_persistence_cookie")),
        "pilot persist"
    );
    assert!(
        got.iter().any(|e| e.contains("\tvlans\tnet_vlan")),
        "pilot vlans"
    );
}
