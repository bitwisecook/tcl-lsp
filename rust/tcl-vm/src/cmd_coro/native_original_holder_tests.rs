// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original coroutine command holders and independently reported names.

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
fn original_coroutine_holder_lifecycle_matches_all_native_public_results() {
    // Native proof: naming.coroutine.original-constructed-holder-lifecycle
    // docs/design/analysis/name-resolution-proofs/coroutine-original-constructed-holder-lifecycle.md
    // The complete originals preserve independent relative resumes/rename/delete;
    // equality of info coroutine reports supplies no physical token identity.
    let providers = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"];
    let cases = [
        ("distinct-original-relative-resumptions", include_bytes!("../../../tcl-registry/tests/data/native_coroutine_original_holder250/distinct-original-relative-resumptions.tcl").as_slice(), [
            include_str!("../../../tcl-registry/tests/data/native_coroutine_original_holder250/8.4.20/distinct-original-relative-resumptions/stdout"),
            include_str!("../../../tcl-registry/tests/data/native_coroutine_original_holder250/8.5.19/distinct-original-relative-resumptions/stdout"),
            include_str!("../../../tcl-registry/tests/data/native_coroutine_original_holder250/8.6.18/distinct-original-relative-resumptions/stdout"),
            include_str!("../../../tcl-registry/tests/data/native_coroutine_original_holder250/9.0.4/distinct-original-relative-resumptions/stdout"),
            include_str!("../../../tcl-registry/tests/data/native_coroutine_original_holder250/9.1.0/distinct-original-relative-resumptions/stdout"),
            include_str!("../../../tcl-registry/tests/data/native_coroutine_original_holder250/jim/distinct-original-relative-resumptions/stdout"),
        ]),
        ("original-relative-deletion-preserves-other-holder", include_bytes!("../../../tcl-registry/tests/data/native_coroutine_original_holder250/original-relative-deletion-preserves-other-holder.tcl").as_slice(), [
            include_str!("../../../tcl-registry/tests/data/native_coroutine_original_holder250/8.4.20/original-relative-deletion-preserves-other-holder/stdout"),
            include_str!("../../../tcl-registry/tests/data/native_coroutine_original_holder250/8.5.19/original-relative-deletion-preserves-other-holder/stdout"),
            include_str!("../../../tcl-registry/tests/data/native_coroutine_original_holder250/8.6.18/original-relative-deletion-preserves-other-holder/stdout"),
            include_str!("../../../tcl-registry/tests/data/native_coroutine_original_holder250/9.0.4/original-relative-deletion-preserves-other-holder/stdout"),
            include_str!("../../../tcl-registry/tests/data/native_coroutine_original_holder250/9.1.0/original-relative-deletion-preserves-other-holder/stdout"),
            include_str!("../../../tcl-registry/tests/data/native_coroutine_original_holder250/jim/original-relative-deletion-preserves-other-holder/stdout"),
        ]),
        ("original-relative-rename-preserves-other-holder", include_bytes!("../../../tcl-registry/tests/data/native_coroutine_original_holder250/original-relative-rename-preserves-other-holder.tcl").as_slice(), [
            include_str!("../../../tcl-registry/tests/data/native_coroutine_original_holder250/8.4.20/original-relative-rename-preserves-other-holder/stdout"),
            include_str!("../../../tcl-registry/tests/data/native_coroutine_original_holder250/8.5.19/original-relative-rename-preserves-other-holder/stdout"),
            include_str!("../../../tcl-registry/tests/data/native_coroutine_original_holder250/8.6.18/original-relative-rename-preserves-other-holder/stdout"),
            include_str!("../../../tcl-registry/tests/data/native_coroutine_original_holder250/9.0.4/original-relative-rename-preserves-other-holder/stdout"),
            include_str!("../../../tcl-registry/tests/data/native_coroutine_original_holder250/9.1.0/original-relative-rename-preserves-other-holder/stdout"),
            include_str!("../../../tcl-registry/tests/data/native_coroutine_original_holder250/jim/original-relative-rename-preserves-other-holder/stdout"),
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
    assert_eq!(comparisons, 18);
}
