//! Differential parity for `candidate_registry_kinds_for_display` (display
//! `Reference.target_kind` → registry kinds) against the Python registry.
//!
//! Exact `(module, object_type)` matches — the form the `ObjectRefSpec` pilot
//! properties use (`"ltm rule"`, `"net vlan"`, …) — must match Python exactly.
//! Family fan-outs (`"ltm monitor"`, `"ltm profile"`, …) are verified as an
//! ordered superset: the Python kinds appear in order, but the generated Rust
//! registry **data** carries a few extra kinds (e.g. `ltm_monitor_none`) — the
//! same registry-data drift documented elsewhere, a data-regen workstream.

use tcl_registry::bigip::default_registry;

/// True when `want` is an in-order subsequence of `got`.
fn is_ordered_subsequence(want: &[&str], got: &[&str]) -> bool {
    let mut wi = 0;
    for &g in got {
        if wi < want.len() && want[wi] == g {
            wi += 1;
        }
    }
    wi == want.len()
}

#[test]
fn display_kinds_match_python() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/display_kinds.golden.tsv"
    );
    let golden = std::fs::read_to_string(path).expect("read golden");
    let reg = default_registry();

    for line in golden.lines() {
        let mut it = line.splitn(2, '\t');
        let display = it.next().unwrap();
        let want_str = it.next().unwrap_or("");
        let want: Vec<&str> = if want_str.is_empty() {
            Vec::new()
        } else {
            want_str.split(',').collect()
        };
        let got = reg.candidate_registry_kinds_for_display(display);

        if want.len() <= 1 {
            // Exact (module, object_type) match — must be identical.
            assert_eq!(got, want, "exact display kind {display:?}");
        } else {
            // Family fan-out — Python's kinds appear in order; Rust may carry
            // extra drift kinds interleaved.
            assert!(
                is_ordered_subsequence(&want, &got),
                "fan-out {display:?}: Python kinds not an ordered subsequence of Rust\n want={want:?}\n got={got:?}"
            );
        }
    }
}
