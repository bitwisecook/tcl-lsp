//! Differential parity for `resolve_kind_in_configs` (the graph's name-
//! resolution layer) against `object_registry.resolve_kind_in_configs`.
//!
//! Resolves a broad set of `(kind, reference, preferred_module)` probes over a
//! representative `bigip.conf` and asserts each resolves to the same source span
//! (or miss) as Python. Self-contained — no Python at test time.

use tcl_bigip::graph::resolve_kind_in_configs;
use tcl_bigip::parser::parse_bigip_conf;
use tcl_registry::BigipRegistry;

#[test]
fn resolve_kind_matches_python() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let source = std::fs::read_to_string(format!("{dir}/bigip.conf")).expect("read config");
    let golden = std::fs::read_to_string(format!("{dir}/resolve_kind.golden.tsv")).expect("golden");

    let cfg = parse_bigip_conf(&source, "Common");
    let configs = [("test://config".to_owned(), &cfg)];
    let reg = BigipRegistry::build();

    let mut checked = 0usize;
    for line in golden.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        let (kind, reference) = (f[0], f[1]);
        let pm = if f[2] == "<none>" { None } else { Some(f[2]) };
        let want = f[3];

        let got = match resolve_kind_in_configs(kind, reference, &configs, pm, &reg) {
            None => "-".to_owned(),
            Some((uri, (sl, sc, el, ec))) => format!("{uri}|{sl},{sc},{el},{ec}"),
        };
        assert_eq!(
            got, want,
            "resolve_kind_in_configs({kind:?}, {reference:?}, pm={pm:?})"
        );
        checked += 1;
    }
    assert!(checked > 50, "golden unexpectedly small: {checked}");
}
