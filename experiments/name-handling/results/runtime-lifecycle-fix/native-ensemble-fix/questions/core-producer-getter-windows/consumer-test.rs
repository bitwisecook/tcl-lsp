#[test]
fn original_namespace_objects_match_100_native_c_producer_and_getter_windows() {
    // Native proof: naming.namespace-native.current-root-primary
    // docs/design/analysis/name-resolution-proofs/namespace-native-current-root-primary.md
    // Native proof: naming.namespace-native.terminal-colon-original-versus-text
    // docs/design/analysis/name-resolution-proofs/namespace-native-terminal-colon-original-versus-text.md
    // Native proof: naming.namespace-native.deleted-current-object-recreation
    // docs/design/analysis/name-resolution-proofs/namespace-native-deleted-current-object-recreation.md
    // Native proof: naming.namespace-native.raw-zero-bytearray-parent
    // docs/design/analysis/name-resolution-proofs/namespace-native-raw-zero-bytearray-parent.md
    // Native proof: naming.namespace-native.opaque-original-parent
    // docs/design/analysis/name-resolution-proofs/namespace-native-opaque-original-parent.md
    // Native proof: naming.namespace-native.active-retirement-cache
    // docs/design/analysis/name-resolution-proofs/namespace-native-active-retirement-cache.md
    // Native proof: naming.namespace-native.missing-parent-cache
    // docs/design/analysis/name-resolution-proofs/namespace-native-missing-parent-cache.md
    for (version, release) in [
        (tcl_dialect::TclVersion::V8_4, "8.4.20"),
        (tcl_dialect::TclVersion::V8_5, "8.5.19"),
        (tcl_dialect::TclVersion::V8_6, "8.6.18"),
        (tcl_dialect::TclVersion::V9_0, "9.0.4"),
        (tcl_dialect::TclVersion::V9_1, "9.1.0"),
    ] {
        counters::reset();
        {
            let mut interp = Interp::new();
            interp.set_runtime_version(version);
            interp.ensure_namespace(b"::a");
            let code = invoke(&mut interp, b"current", None);
            let result = interp.result_obj();
            check(&mut interp, release, "root-current", result, code, None);
            let root = obj::Owned::retain(interp.result_obj());
            let duplicate = obj::Owned::fresh(obj::duplicate(root.as_ptr()));
            check(
                &mut interp,
                release,
                "root-duplicate",
                duplicate.as_ptr(),
                Code::Ok,
                None,
            );
            parent(&mut interp, release, "root-parent", duplicate.as_ptr());
            drop(duplicate);
            drop(root);
            let outer = interp.ensure_namespace(b"::a:");
            push_namespace(&mut interp, outer);
            let child = interp.ensure_namespace(b"q");
            interp.codegen_frame_pop();
            push_namespace(&mut interp, child);
            let code = invoke(&mut interp, b"current", None);
            let result = interp.result_obj();
            check(
                &mut interp,
                release,
                "terminal-colon-child-current",
                result,
                code,
                None,
            );
            let original = obj::Owned::retain(interp.result_obj());
            let duplicate = obj::Owned::fresh(obj::duplicate(original.as_ptr()));
            check(
                &mut interp,
                release,
                "nested-duplicate",
                duplicate.as_ptr(),
                Code::Ok,
                None,
            );
            if version >= tcl_dialect::TclVersion::V9_0 {
                assert!(obj::native_namespace_name::cache(original.as_ptr())
                    .unwrap()
                    .same_descriptor(
                        &obj::native_namespace_name::cache(duplicate.as_ptr()).unwrap()
                    ));
            }
            interp.codegen_frame_pop();
            parent(
                &mut interp,
                release,
                "current-object-parent-from-root",
                original.as_ptr(),
            );
            let written = obj::Owned::fresh(obj::new_string_bytes(b"::a:::q"));
            parent(
                &mut interp,
                release,
                "reported-text-parent-from-root",
                written.as_ptr(),
            );
            interp.delete_namespace_by_id(child);
            parent(
                &mut interp,
                release,
                "deleted-current-object-parent",
                original.as_ptr(),
            );
            parent(
                &mut interp,
                release,
                "deleted-duplicate-parent",
                duplicate.as_ptr(),
            );
            push_namespace(&mut interp, outer);
            let recreated = interp.ensure_namespace(b"q");
            assert_ne!(child, recreated);
            interp.codegen_frame_pop();
            parent(
                &mut interp,
                release,
                "recreated-current-object-parent",
                original.as_ptr(),
            );
            let nul = obj::Owned::fresh(obj::new_string_bytes(b"::a\0z"));
            parent(&mut interp, release, "raw-NUL-parent", nul.as_ptr());
            let modified = obj::Owned::fresh(obj::new_string_bytes(b"::a\xc0\x80z"));
            parent(
                &mut interp,
                release,
                "modified-NUL-negative-parent",
                modified.as_ptr(),
            );
            let binary = obj::Owned::fresh(interp.new_native_byte_array(b"::a\0z").unwrap());
            parent(
                &mut interp,
                release,
                "pure-ByteArray-negative-parent",
                binary.as_ptr(),
            );
            interp.ensure_namespace(b"::a\xc0\x80z");
            parent(
                &mut interp,
                release,
                "modified-NUL-positive-parent",
                modified.as_ptr(),
            );
            parent(
                &mut interp,
                release,
                "pure-ByteArray-positive-parent",
                binary.as_ptr(),
            );
            interp.ensure_namespace(b"::\xff");
            let opaque = obj::Owned::fresh(obj::new_string_bytes(b"::\xff"));
            parent(
                &mut interp,
                release,
                "raw-FF-positive-parent",
                opaque.as_ptr(),
            );
            let retiring = interp.ensure_namespace(b"::Z");
            let name = obj::Owned::fresh(obj::new_string_bytes(b"::Z"));
            parent(&mut interp, release, "active-retire-before", name.as_ptr());
            push_namespace(&mut interp, retiring);
            interp.delete_namespace_by_id(retiring);
            parent(&mut interp, release, "active-retire-cached", name.as_ptr());
            interp.codegen_frame_pop();
            parent(
                &mut interp,
                release,
                "active-retire-after-pop",
                name.as_ptr(),
            );
            let missing = obj::Owned::fresh(obj::new_string_bytes(b"::missing"));
            parent(&mut interp, release, "fresh-missing", missing.as_ptr());
        }
        assert_eq!(
            counters::finalize(),
            0,
            "{release} leaked namespace object leases"
        );
        assert_eq!(counters::double_free_count(), 0);
    }
}
