//! Byte-stable fixtures for `compute_stats` (text report + JSON), checked
//! against captured expected output on a realistic `bigip.conf`. Self-contained.

use tcl_bigip::graph::{GraphContext, build_bigip_object_graph};
use tcl_bigip::parser::parse_bigip_conf;
use tcl_bigip::stats::{compute_stats, report_to_json};

#[test]
fn stats() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let source = std::fs::read_to_string(format!("{dir}/bigip.conf")).expect("read config");
    let cfg = parse_bigip_conf(&source, "Common");
    let uri = "test://config".to_owned();
    let sources = [(uri.clone(), source)];
    let configs = [(uri, &cfg)];

    let ctx = GraphContext::new();
    let graph = build_bigip_object_graph(&sources, &configs, &ctx);

    for top in [10usize, 3] {
        let report = compute_stats(&graph, &configs, top);

        let mut text = report.text_report.clone();
        if !text.ends_with('\n') {
            text.push('\n');
        }
        let want_text = std::fs::read_to_string(format!("{dir}/stats_top{top}.txt.golden"))
            .expect("read text golden");
        assert_eq!(
            text, want_text,
            "stats text (top {top}) differs from the expected fixture"
        );

        let json = report_to_json(&report) + "\n";
        let want_json = std::fs::read_to_string(format!("{dir}/stats_top{top}.json.golden"))
            .expect("read json golden");
        assert_eq!(
            json, want_json,
            "stats JSON (top {top}) differs from the expected fixture"
        );
    }
}
