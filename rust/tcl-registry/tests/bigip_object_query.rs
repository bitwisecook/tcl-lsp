//! Differential parity for the BIG-IP object-registry query layer.
//!
//! Asserts the Rust `kind_for_header` / `candidate_kinds_for_key` /
//! `candidate_kinds_for_section_item` reproduce `object_registry.py` exactly,
//! over a golden captured from the Python registry (every header in the
//! registry, plus every property name per container, probed across several
//! sections). Self-contained — no Python at test time.
//!
//! `kind_for_header` (the structural header→kind map), `candidate_kinds_for_key`,
//! and `candidate_kinds_for_section_item` must all match Python **exactly** —
//! every probe, no exceptions. The registry **data** (`bigip/data`) is
//! regenerated from the reconciled Python `OBJECT_SPECS` by
//! `scripts/registry-audit/gen_bigip_rust.py`, so there is no data drift to pin.

use std::collections::BTreeSet;

use tcl_registry::bigip::default_registry;

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path).expect("read fixture")
}

#[test]
fn object_query_matches_python() {
    let reg = default_registry();
    let mut checked = 0usize;
    let mut header_mismatches: Vec<String> = Vec::new();
    let mut candidate_mismatches: BTreeSet<String> = BTreeSet::new();

    for line in fixture("object_query.golden.tsv").lines() {
        let f: Vec<&str> = line.split('\t').collect();
        match f[0] {
            "H" => {
                let (module, object_type, want) = (f[1], f[2], f[3]);
                let got = reg.kind_for_header(module, object_type).unwrap_or("-");
                if got != want {
                    header_mismatches.push(format!(
                        "kind_for_header({module:?},{object_type:?}): got {got:?} want {want:?}"
                    ));
                }
            }
            "K" => {
                let (module, object_type, name) = (f[1], f[2], f[3]);
                let section = if f[4] == "<none>" { None } else { Some(f[4]) };
                let want: Vec<&str> = f[5].split(',').collect();
                let got =
                    reg.candidate_kinds_for_key(name, section, Some(module), Some(object_type));
                if got != want {
                    candidate_mismatches.insert(format!(
                        "  candidate_kinds_for_key({name:?},sec={section:?},{module}/{object_type})"
                    ));
                }
            }
            "S" => {
                let (module, object_type, name) = (f[1], f[2], f[3]);
                let want: Vec<&str> = f[4].split(',').collect();
                let got =
                    reg.candidate_kinds_for_section_item(name, Some(module), Some(object_type));
                if got != want {
                    candidate_mismatches.insert(format!(
                        "  candidate_kinds_for_section_item({name:?},{module}/{object_type})"
                    ));
                }
            }
            other => panic!("unknown golden row tag {other:?}"),
        }
        checked += 1;
    }
    assert!(checked > 3000, "golden unexpectedly small: {checked} rows");

    // The structural header→kind mapping must be exactly in sync.
    assert!(
        header_mismatches.is_empty(),
        "kind_for_header diverged from Python:\n{}",
        header_mismatches.join("\n")
    );

    // candidate_kinds_* must match Python on every probe.
    assert!(
        candidate_mismatches.is_empty(),
        "candidate_kinds diverged from Python ({} rows):\n{}",
        candidate_mismatches.len(),
        candidate_mismatches
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
