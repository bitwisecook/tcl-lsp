    #[test]
    fn mutually_entangled_deletion_preserves_the_global_namespace() {
        for version in tcl_dialect::TclVersion::ALL {
            leak_free_with(
                || native_namespace_interp(version),
                |interp| {
                    assert_eq!(
                    interp.eval_str(
                        br#"namespace eval ::testing {
                            namespace eval abc {proc xyz {} {}}
                            namespace eval def {proc xyz {} {}}
                            trace add command abc::xyz delete "namespace delete ::testing::def {}; #"
                            trace add command def::xyz delete "namespace delete ::testing::abc {}; #"
                        }
                        namespace delete ::testing
                        list [namespace exists ::testing] [info commands set]"#,
                    ),
                    Code::Ok,
                    "{version:?}: {:?}",
                    interp.result_bytes(),
                );
                    assert_eq!(interp.result_bytes(), b"0 set", "{version:?}");
                },
            );
        }
    }
