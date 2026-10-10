// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original constructed OO holders, reports and collision controls.

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
    // The VM creation interface is measured independently; no oo::copy entry
    // is installed or inferred from these creation and collision observations.
    let providers = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"];
    let cases = [
        ("bare-create-current-colon-holder", include_bytes!("../../../tcl-registry/tests/data/native_oo_original_holder248/bare-create-current-colon-holder.tcl").as_slice(), [
                include_str!("../../../tcl-registry/tests/data/native_oo_original_holder248/8.4.20/bare-create-current-colon-holder/stdout"),
                include_str!("../../../tcl-registry/tests/data/native_oo_original_holder248/8.5.19/bare-create-current-colon-holder/stdout"),
                include_str!("../../../tcl-registry/tests/data/native_oo_original_holder248/8.6.18/bare-create-current-colon-holder/stdout"),
                include_str!("../../../tcl-registry/tests/data/native_oo_original_holder248/9.0.4/bare-create-current-colon-holder/stdout"),
                include_str!("../../../tcl-registry/tests/data/native_oo_original_holder248/9.1.0/bare-create-current-colon-holder/stdout"),
                include_str!("../../../tcl-registry/tests/data/native_oo_original_holder248/jim/bare-create-current-colon-holder/stdout"),
            ]),
        ("colon-tail-create-current-holder", include_bytes!("../../../tcl-registry/tests/data/native_oo_original_holder248/colon-tail-create-current-holder.tcl").as_slice(), [
                include_str!("../../../tcl-registry/tests/data/native_oo_original_holder248/8.4.20/colon-tail-create-current-holder/stdout"),
                include_str!("../../../tcl-registry/tests/data/native_oo_original_holder248/8.5.19/colon-tail-create-current-holder/stdout"),
                include_str!("../../../tcl-registry/tests/data/native_oo_original_holder248/8.6.18/colon-tail-create-current-holder/stdout"),
                include_str!("../../../tcl-registry/tests/data/native_oo_original_holder248/9.0.4/colon-tail-create-current-holder/stdout"),
                include_str!("../../../tcl-registry/tests/data/native_oo_original_holder248/9.1.0/colon-tail-create-current-holder/stdout"),
                include_str!("../../../tcl-registry/tests/data/native_oo_original_holder248/jim/colon-tail-create-current-holder/stdout"),
            ]),
        ("distinct-constructed-slots-with-equal-reports", include_bytes!("../../../tcl-registry/tests/data/native_oo_original_holder248/distinct-constructed-slots-with-equal-reports.tcl").as_slice(), [
                include_str!("../../../tcl-registry/tests/data/native_oo_original_holder248/8.4.20/distinct-constructed-slots-with-equal-reports/stdout"),
                include_str!("../../../tcl-registry/tests/data/native_oo_original_holder248/8.5.19/distinct-constructed-slots-with-equal-reports/stdout"),
                include_str!("../../../tcl-registry/tests/data/native_oo_original_holder248/8.6.18/distinct-constructed-slots-with-equal-reports/stdout"),
                include_str!("../../../tcl-registry/tests/data/native_oo_original_holder248/9.0.4/distinct-constructed-slots-with-equal-reports/stdout"),
                include_str!("../../../tcl-registry/tests/data/native_oo_original_holder248/9.1.0/distinct-constructed-slots-with-equal-reports/stdout"),
                include_str!("../../../tcl-registry/tests/data/native_oo_original_holder248/jim/distinct-constructed-slots-with-equal-reports/stdout"),
            ]),
        ("create-over-ordinary-command-in-original-holder", include_bytes!("../../../tcl-registry/tests/data/native_oo_original_collision249/create-over-ordinary-command-in-original-holder.tcl").as_slice(), [
                include_str!("../../../tcl-registry/tests/data/native_oo_original_collision249/8.4.20/create-over-ordinary-command-in-original-holder/stdout"),
                include_str!("../../../tcl-registry/tests/data/native_oo_original_collision249/8.5.19/create-over-ordinary-command-in-original-holder/stdout"),
                include_str!("../../../tcl-registry/tests/data/native_oo_original_collision249/8.6.18/create-over-ordinary-command-in-original-holder/stdout"),
                include_str!("../../../tcl-registry/tests/data/native_oo_original_collision249/9.0.4/create-over-ordinary-command-in-original-holder/stdout"),
                include_str!("../../../tcl-registry/tests/data/native_oo_original_collision249/9.1.0/create-over-ordinary-command-in-original-holder/stdout"),
                include_str!("../../../tcl-registry/tests/data/native_oo_original_collision249/jim/create-over-ordinary-command-in-original-holder/stdout"),
            ]),
    ];
    let mut comparisons = 0;
    for (case, source, rows) in cases {
        for (engine, rows) in providers.iter().zip(rows) {
            let (expected_code, expected_result) = original_result(rows);
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = crate::native_fixture::interpreter(profile);
            let completion = vm
                .eval_source(std::str::from_utf8(source).unwrap())
                .unwrap();
            assert_eq!(
                completion.code.as_int(),
                expected_code,
                "{engine}/{case}: {completion:?}; host={:?}",
                vm.refused_completion()
            );
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                expected_result.as_slice(),
                "{engine}/{case}"
            );
            assert!(vm.refused_completion().is_none(), "{engine}/{case}");
            comparisons += 1;
        }
    }
    assert_eq!(comparisons, 24);
}
