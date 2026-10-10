// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original constructed OO holders, reports and collision controls.

use super::*;

fn original_result(rows: &str) -> (i64, Vec<u8>) {
    let fields = rows
        .lines()
        .find(|line| line.starts_with("ORIGINAL|"))
        .unwrap()
        .split('|')
        .collect::<Vec<_>>();
    (
        fields[1].parse().unwrap(),
        fields[2]
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect(),
    )
}

#[test]
fn original_oo_publication_preserves_constructed_holder_and_collision_results() {
    // Native proof: naming.class.original-unaddressable-holder-publication
    // docs/design/analysis/name-resolution-proofs/class-original-unaddressable-holder-publication.md
    // Native proof: naming.class.original-command-slot-collision
    // docs/design/analysis/name-resolution-proofs/class-original-command-slot-collision.md
    // Native proof: naming.class.original-copy-holder-and-collision
    // docs/design/analysis/name-resolution-proofs/class-original-copy-holder-and-collision.md
    // Original failed setup controls remain whole observations, independent from
    // the uniquely named source controls which actually reach oo::copy.
    crate::counters::reset();
    let providers = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"];
    let cases = [
        ("bare-create-current-colon-holder", include_bytes!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/bare-create-current-colon-holder.tcl").as_slice(), [
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/8.4.20/bare-create-current-colon-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/8.5.19/bare-create-current-colon-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/8.6.18/bare-create-current-colon-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/9.0.4/bare-create-current-colon-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/9.1.0/bare-create-current-colon-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/jim/bare-create-current-colon-holder/stdout"),
            ]),
        ("colon-tail-create-current-holder", include_bytes!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/colon-tail-create-current-holder.tcl").as_slice(), [
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/8.4.20/colon-tail-create-current-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/8.5.19/colon-tail-create-current-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/8.6.18/colon-tail-create-current-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/9.0.4/colon-tail-create-current-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/9.1.0/colon-tail-create-current-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/jim/colon-tail-create-current-holder/stdout"),
            ]),
        ("distinct-constructed-slots-with-equal-reports", include_bytes!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/distinct-constructed-slots-with-equal-reports.tcl").as_slice(), [
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/8.4.20/distinct-constructed-slots-with-equal-reports/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/8.5.19/distinct-constructed-slots-with-equal-reports/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/8.6.18/distinct-constructed-slots-with-equal-reports/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/9.0.4/distinct-constructed-slots-with-equal-reports/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/9.1.0/distinct-constructed-slots-with-equal-reports/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/jim/distinct-constructed-slots-with-equal-reports/stdout"),
            ]),
        ("copy-original-relative-constructed-slots", include_bytes!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/copy-original-relative-constructed-slots.tcl").as_slice(), [
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/8.4.20/copy-original-relative-constructed-slots/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/8.5.19/copy-original-relative-constructed-slots/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/8.6.18/copy-original-relative-constructed-slots/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/9.0.4/copy-original-relative-constructed-slots/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/9.1.0/copy-original-relative-constructed-slots/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_holder248/jim/copy-original-relative-constructed-slots/stdout"),
            ]),
        ("create-over-ordinary-command-in-original-holder", include_bytes!("../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/create-over-ordinary-command-in-original-holder.tcl").as_slice(), [
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.4.20/create-over-ordinary-command-in-original-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.5.19/create-over-ordinary-command-in-original-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.6.18/create-over-ordinary-command-in-original-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.0.4/create-over-ordinary-command-in-original-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.1.0/create-over-ordinary-command-in-original-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/jim/create-over-ordinary-command-in-original-holder/stdout"),
            ]),
        ("copy-over-ordinary-command-in-original-holder", include_bytes!("../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/copy-over-ordinary-command-in-original-holder.tcl").as_slice(), [
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.4.20/copy-over-ordinary-command-in-original-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.5.19/copy-over-ordinary-command-in-original-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.6.18/copy-over-ordinary-command-in-original-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.0.4/copy-over-ordinary-command-in-original-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.1.0/copy-over-ordinary-command-in-original-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/jim/copy-over-ordinary-command-in-original-holder/stdout"),
            ]),
        ("copy-original-relative-constructed-slots", include_bytes!("../../../../rust/tcl-registry/tests/data/native_oo_original_copy251/copy-original-relative-constructed-slots.tcl").as_slice(), [
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_copy251/8.4.20/copy-original-relative-constructed-slots/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_copy251/8.5.19/copy-original-relative-constructed-slots/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_copy251/8.6.18/copy-original-relative-constructed-slots/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_copy251/9.0.4/copy-original-relative-constructed-slots/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_copy251/9.1.0/copy-original-relative-constructed-slots/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_copy251/jim/copy-original-relative-constructed-slots/stdout"),
            ]),
        ("copy-over-ordinary-command-in-original-holder", include_bytes!("../../../../rust/tcl-registry/tests/data/native_oo_original_copy251/copy-over-ordinary-command-in-original-holder.tcl").as_slice(), [
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_copy251/8.4.20/copy-over-ordinary-command-in-original-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_copy251/8.5.19/copy-over-ordinary-command-in-original-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_copy251/8.6.18/copy-over-ordinary-command-in-original-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_copy251/9.0.4/copy-over-ordinary-command-in-original-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_copy251/9.1.0/copy-over-ordinary-command-in-original-holder/stdout"),
                include_str!("../../../../rust/tcl-registry/tests/data/native_oo_original_copy251/jim/copy-over-ordinary-command-in-original-holder/stdout"),
            ]),
    ];
    let mut comparisons = 0;
    for (case, source, rows) in cases {
        for (engine, rows) in providers.iter().zip(rows) {
            let (expected_code, expected_result) = original_result(rows);
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            // The native provider starts with its distribution extensions loaded.
            crate::cmd_proc::install_stock_scripted_wrappers(&mut interp);
            let code = interp.eval_str(source);
            assert_eq!(
                code.as_int(),
                expected_code,
                "{engine}/{case}: {:?}",
                interp.result_bytes()
            );
            assert_eq!(interp.result_bytes(), expected_result, "{engine}/{case}");
            assert!(!interp.host_refusal_pending(), "{engine}/{case}");
            comparisons += 1;
        }
    }
    assert_eq!(comparisons, 48);
    assert_eq!(crate::counters::finalize(), 0);
    assert_eq!(crate::counters::double_free_count(), 0);
}
