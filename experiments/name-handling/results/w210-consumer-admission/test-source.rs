fn tcltest_shared_frame_keeps_genuine_undefined_reads_and_data_opaque() {
    let source = "package require tcltest\n\
                  namespace import -force ::tcltest::*\n\
                  proc exercise {} {\n\
                  test t shared -setup {set prepared 1} \
                  -body {puts $prepared; puts $absent} \
                  -cleanup {puts $prepared} -result {[puts $data_only]}\n\
                  }\nexercise\n";
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let result = crate::provider_fixtures::analyse(
            source,
            profile,
            &[crate::provider_fixtures::Provider::Tcltest],
        );
        let undefined = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::W210)
            .collect::<Vec<_>>();
        assert_eq!(undefined.len(), 1, "{profile}/{undefined:?}");
        assert!(
            undefined[0].message.contains("absent"),
            "{profile}/{undefined:?}"
        );
        // An explicit driver with no selected loader keeps the callback data
        // opaque. The default C authoring driver selects its audited provider.
        let unattested = crate::provider_fixtures::analyse(source, profile, &[]);
        let unattested_undefined = unattested
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::W210)
            .collect::<Vec<_>>();
        assert!(
            unattested_undefined.is_empty(),
            "{profile}: a package requirement alone cannot authenticate callback entry: {unattested_undefined:?}"
        );
    }
}

