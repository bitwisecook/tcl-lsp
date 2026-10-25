// SPDX-License-Identifier: AGPL-3.0-or-later

#[test]
fn interpreter_option_prefixes_preserve_positive_root_and_child_dispatch() {
    crate::counters::reset();
    {
        let mut interp = crate::interp::Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("tcl9.0"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        for (source, expected) in [
            (b"interp cr j".as_slice(), b"j".as_slice()),
            (b"interp ev {} {set x 1}", b"1"),
            (b"llength [interp sl]", b"1"),
        ] {
            assert_eq!(interp.eval_str(source), crate::interp::Code::Ok);
            assert!(!interp.host_refusal_pending());
            assert_eq!(interp.result_bytes(), expected);
        }
        assert_eq!(
            interp.eval_str(b"interp create kid"),
            crate::interp::Code::Ok
        );
        assert!(!interp.host_refusal_pending());
        assert_eq!(
            interp.eval_str(b"kid ev {set x 1}"),
            crate::interp::Code::Ok
        );
        assert!(!interp.host_refusal_pending());
        assert_eq!(interp.result_bytes(), b"1");
    }
    assert_eq!(crate::counters::finalize(), 0);
    assert_eq!(crate::counters::double_free_count(), 0);
}
