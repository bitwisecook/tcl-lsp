#[test]
fn original_namespace_lifetime_survives_retirement_without_owning_the_interpreter() {
    let original;
    let token;
    {
        let mut interp = Interp::new();
        interp.set_runtime_version(tcl_dialect::TclVersion::V8_4);
        let namespace = interp.ensure_namespace(b"::Z");
        original = obj::Owned::fresh(obj::new_string_bytes(b"::Z"));
        assert_eq!(
            interp
                .native_namespace_object_lookup(original.as_ptr())
                .unwrap(),
            Some(namespace)
        );
        token = obj::native_namespace_name::cache(original.as_ptr())
            .unwrap()
            .namespace()
            .unwrap()
            .clone();
        let duplicate = obj::Owned::fresh(obj::duplicate(original.as_ptr()));
        assert!(obj::native_namespace_name::cache(original.as_ptr())
            .unwrap()
            .same_descriptor(&obj::native_namespace_name::cache(duplicate.as_ptr()).unwrap()));
        obj::invalidate_string(original.as_ptr());
        assert_eq!(
            interp
                .native_string_bytes(&original.as_ptr())
                .unwrap()
                .as_ref(),
            b"::Z"
        );
    }
    assert_eq!(
        token.lifecycle(),
        tcl_syntax::native_namespace_name::NativeNamespaceLifecycle::Dead
    );
    obj::invalidate_string(original.as_ptr());
    let protocol =
        tcl_syntax::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4);
    assert_eq!(
        crate::dict::native_object_bytes(original.as_ptr(), protocol).unwrap(),
        b""
    );
    assert!(obj::has_canonical_empty_string(original.as_ptr()));
}
