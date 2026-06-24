//! OBJ family — object dispatch (W307/W308) + snit / TclOO modelling.
//! Pairs to `tests/test_fp_obj.py` and the §OBJ entries in FP.md.

use super::{codes, fires, D};

// FP-OBJ-01 — snit self-references ($self/$type/$selfns/$win) are method
// dispatch on the current object, not stray non-literal command words.

#[test]
fn fp_obj_01_snit_self_references_no_w307() {
    for r in ["self", "type", "selfns", "win"] {
        let src = format!("snit::type T {{\n method m {{}} {{ ${r} foo }}\n}}");
        assert!(!fires(&src, D, "W307"), "${r} foo in snit body fired W307: {:?}", codes(&src, D));
    }
}

#[test]
fn fp_obj_01_self_ref_outside_snit_still_w307() {
    // TP control: the same names in a vanilla proc ARE stray dispatch.
    for r in ["self", "type", "selfns", "win", "hull"] {
        let src = format!("proc f {{}} {{ set {r} [getThing]\n ${r} foo }}");
        assert!(fires(&src, D, "W307"), "${r} foo outside snit did not fire W307");
    }
}
