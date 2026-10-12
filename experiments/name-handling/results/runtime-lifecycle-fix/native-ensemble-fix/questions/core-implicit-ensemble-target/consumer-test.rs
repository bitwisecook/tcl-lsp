    #[test]
    fn implicit_ensemble_targets_preserve_the_selected_namespace_context() {
        for version in tcl_dialect::TclVersion::ALL
            .into_iter()
            .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
        {
            leak_free_with(
                || native_namespace_interp(version),
                |interp| {
                    assert_eq!(
                        interp.eval_str(br#"namespace eval ::M {
                            namespace ensemble create -command path -subcommands {add remove}
                            proc add {path} {return [list ADD [namespace current] $path]}
                        }
                        proc ::add {path} {return [list GLOBAL $path]}
                        proc ::remove {path} {return [list GLOBAL_REMOVE $path]}
                        namespace eval ::Caller {proc add {path} {return [list CALLER $path]}}
                        list [::M::path add x] [namespace eval ::Caller {::M::path add y}] [::add z]"#),
                        Code::Ok,
                        "{version:?}: {:?}",
                        interp.result_bytes(),
                    );
                    assert_eq!(
                        interp.result_bytes(),
                        b"{ADD ::M x} {ADD ::M y} {GLOBAL z}",
                        "{version:?}"
                    );
                    assert_eq!(interp.eval_str(br#"set code [catch {namespace eval ::Caller {::M::path remove q}} result]; list $code $result"#), Code::Ok);
                    let expected = if version == tcl_dialect::TclVersion::V8_5 {
                        b"1 {invalid command name \"::M::remove\"}".as_slice()
                    } else {
                        b"0 {GLOBAL_REMOVE q}".as_slice()
                    };
                    assert_eq!(interp.result_bytes(), expected, "{version:?}");
                    assert!(interp.native_execution_refusal().is_none());
                },
            );
        }
    }
