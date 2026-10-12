#[test]
fn namespace_delete_validates_every_original_before_retirement_and_relooks_after_callbacks() {
    // Native proof: naming.namespace-native.delete-validates-later
    // docs/design/analysis/name-resolution-proofs/namespace-native-delete-validates-later.md
    // Native proof: naming.namespace-native.delete-dependent-child-duplicate
    // docs/design/analysis/name-resolution-proofs/namespace-native-delete-dependent-child-duplicate.md
    // Native proof: naming.namespace-native.delete-unset-trace-later-effects
    // docs/design/analysis/name-resolution-proofs/namespace-native-delete-unset-trace-later-effects.md
    for version in tcl_dialect::TclVersion::ALL {
        let mut interp = Interp::new();
        interp.set_runtime_version(version);
        interp.ensure_namespace(b"::A");
        interp.ensure_namespace(b"::A::child");
        let a = obj::Owned::fresh(obj::new_string_bytes(b"::A"));
        let missing = obj::Owned::fresh(obj::new_string_bytes(b"::missing"));
        assert_eq!(
            tcl_cmd_core::namespace::delete_original(&mut interp, &[a.as_ptr(), missing.as_ptr()])
                .unwrap(),
            Some(1)
        );
        assert!(interp.find_namespace_id(b"::A").is_some());
        assert!(interp.find_namespace_id(b"::A::child").is_some());
        let child = obj::Owned::fresh(obj::new_string_bytes(b"::A::child"));
        assert_eq!(
            tcl_cmd_core::namespace::delete_original(
                &mut interp,
                &[a.as_ptr(), child.as_ptr(), a.as_ptr()]
            )
            .unwrap(),
            None
        );
        assert!(interp.find_namespace_id(b"::A").is_none());
        for recreate in [false, true] {
            let setup = if recreate {
                b"namespace eval ::A {}; namespace eval ::B {}; set ::calls 0; set ::A::x 1; proc dependent {a b c} {incr ::calls; namespace delete ::B; namespace eval ::B {set fresh 1}}".as_slice()
            } else {
                b"namespace eval ::A {}; namespace eval ::B {}; set ::calls 0; set ::A::x 1; proc dependent {a b c} {incr ::calls; namespace delete ::B}".as_slice()
            };
            assert_eq!(
                interp.eval_str(setup),
                Code::Ok,
                "{version:?}: {:?}",
                interp.result_bytes()
            );
            let trace = if version < tcl_dialect::TclVersion::V9_0 {
                b"trace variable ::A::x u dependent".as_slice()
            } else {
                b"trace add variable ::A::x unset dependent".as_slice()
            };
            assert_eq!(interp.eval_str(trace), Code::Ok);
            let b = obj::Owned::fresh(obj::new_string_bytes(b"::B"));
            let command = obj::Owned::fresh(obj::new_string_bytes(b"namespace"));
            let member = obj::Owned::fresh(obj::new_string_bytes(b"delete"));
            assert_eq!(
                interp.dispatch(&[command.as_ptr(), member.as_ptr(), a.as_ptr(), b.as_ptr()]),
                Code::Ok,
                "{version:?} recreate={recreate}: {:?}; refusal={:?}",
                interp.result_bytes(),
                interp.native_access_refusal()
            );
            assert!(interp.result_bytes().is_empty());
            assert!(interp.find_namespace_id(b"::A").is_none());
            assert!(interp.find_namespace_id(b"::B").is_none());
            assert_eq!(interp.read_var(b"::calls", None).unwrap().as_slice(), b"1");
        }
    }
}
