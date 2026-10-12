    #[test]
    fn child_teardown_distinguishes_cached_names_from_fresh_lookup() {
        for version in tcl_dialect::TclVersion::ALL
            .into_iter()
            .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
        {
            leak_free_with(
                || native_namespace_interp(version),
                |interp| {
                    assert_eq!(
                        interp.eval_str(
                            br#"set seen {}
                        proc observe {tag old new op} {
                            set fresh ::P
                            append fresh ::C
                            lappend ::seen [list $tag [namespace exists ::P::C] \
                                [namespace exists $fresh]]
                        }
                        namespace eval ::P {
                            namespace ensemble create -command ::PE
                            namespace eval C {namespace ensemble create -command ::CE}
                        }
                        trace add command ::PE delete [list observe parent]
                        trace add command ::CE delete [list observe child]
                        namespace delete ::P
                        set seen"#,
                        ),
                        Code::Ok,
                        "{version:?}: {:?}",
                        interp.result_bytes(),
                    );
                    assert_eq!(
                        interp.result_bytes(),
                        b"{parent 1 1} {child 1 0}",
                        "{version:?}",
                    );
                },
            );
        }
    }
