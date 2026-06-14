//! Differential parity for the BIG-IP object-registry query layer.
//!
//! Asserts the Rust `kind_for_header` / `candidate_kinds_for_key` /
//! `candidate_kinds_for_section_item` reproduce `object_registry.py` exactly,
//! over a golden captured from the Python registry (every header in the
//! registry, plus every property name per container, probed across several
//! sections). Self-contained — no Python at test time.
//!
//! `kind_for_header` (the structural header→kind map) must match **exactly**.
//! For `candidate_kinds_*` the *logic* is verified the same way, but a small,
//! frozen set of rows differ because the generated Rust registry **data**
//! (`bigip/data`, a `gen_bigip_rust.py` baseline) has drifted from current
//! Python for ~14 properties' `references` lists. That drift is pinned in
//! `object_query.drift.txt` and is a separate registry-data workstream — not a
//! query-logic bug. Any *new* divergence (a real logic regression) changes the
//! mismatch set and fails this test.

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

    // candidate_kinds_* must match Python everywhere except the frozen
    // registry-data drift set — proving the query logic is faithful.
    let expected_drift: BTreeSet<String> = fixture("object_query.drift.txt")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_owned)
        .collect();
    assert_eq!(
        candidate_mismatches, expected_drift,
        "candidate_kinds drift set changed — a query-logic regression or registry-data shift \
         (regenerate object_query.drift.txt only if the data baseline legitimately moved)"
    );
}
