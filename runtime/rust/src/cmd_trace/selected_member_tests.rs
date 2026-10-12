// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual detached-member correspondence is separate from public trace logs.

use super::{VarTrace, VarTraceScope};
use crate::{
    frame::VarTable,
    obj::{self, Owned},
    vars::TraceHome,
};

#[test]
fn trace_registration_distinguishes_recreated_members_of_a_retained_root() {
    // naming.variable.original-array-member-trace-retirement-horizon
    // docs/design/analysis/name-resolution-proofs/variable-original-array-member-trace-retirement-horizon.md
    // This Rust owner test checks actual member correspondence; the native
    // programs independently establish public callback-selection outcomes.
    let mut table = VarTable::default();
    let value = Owned::fresh(obj::new_string_bytes(b"VALUE"));
    table.store_elem(b"a", b"k", value.as_ptr()).unwrap();
    let receiver = table.capture_receiver(b"a", None).unwrap();
    let home = TraceHome {
        binding_id: receiver.binding_id(),
        selected_member: None,
        ns: None,
        level: Some(0),
        base: b"a".to_vec(),
        link_elem: None,
    };
    let original = table.begin_array_destruction(b"a").unwrap();
    let old_home = home.for_selected_array_member(&original, b"k").unwrap();
    let old = VarTraceScope::cell(&old_home, Some(b"k"), original.trace_member_identity(b"k"));
    table.store_elem(b"a", b"k", value.as_ptr()).unwrap();
    assert_eq!(table.trace_binding_identity(b"a"), home.binding_id);
    let replacement = table.capture_array_cell(b"a").unwrap();
    let current = VarTraceScope::cell(&home, Some(b"k"), replacement.trace_member_identity(b"k"));
    assert_ne!(old.member_identity(), current.member_identity());
    let trace = VarTrace {
        id: 0,
        binding_id: home.binding_id,
        element_binding_id: old.member_identity(),
        name: b"a(k)".to_vec(),
        base: b"a".to_vec(),
        elem: Some(b"k".to_vec()),
        ops: vec![b"unset".to_vec()],
        command: b"callback".to_vec(),
        native: None,
        frame_level: Some(0),
        ns: None,
        old_style: false,
    };
    assert!(old.owns_registration(&trace));
    assert!(old.matches(&trace, b"unset"));
    assert!(!current.owns_registration(&trace));
    assert!(!current.matches(&trace, b"unset"));
    assert!(!current.matches(&trace, b"read"));
    original.finish_destruction();
}
