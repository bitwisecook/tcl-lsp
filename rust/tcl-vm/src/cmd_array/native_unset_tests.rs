// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Complete original `ArrayUnset` public results across native releases.

type OriginalControl = (&'static str, &'static [u8], [String; 6]);

fn unset_controls() -> [OriginalControl; 4] {
    [
    (
        "native_array_operational_unset355/array-member-refill",
        include_bytes!("../../../tcl-registry/tests/data/native_array_operational_unset355/array-member-refill.tcl").as_slice(),
        [
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_operational_unset355/captures.tsv"), "8.4.20/array-member-refill/stdout"),
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_operational_unset355/captures.tsv"), "8.5.19/array-member-refill/stdout"),
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_operational_unset355/captures.tsv"), "8.6.18/array-member-refill/stdout"),
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_operational_unset355/captures.tsv"), "9.0.4/array-member-refill/stdout"),
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_operational_unset355/captures.tsv"), "9.1.0/array-member-refill/stdout"),
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_operational_unset355/captures.tsv"), "jim/array-member-refill/stdout")
        ],
    ),
    (
        "native_array_operational_unset355/array-root-refill",
        include_bytes!("../../../tcl-registry/tests/data/native_array_operational_unset355/array-root-refill.tcl").as_slice(),
        [
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_operational_unset355/captures.tsv"), "8.4.20/array-root-refill/stdout"),
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_operational_unset355/captures.tsv"), "8.5.19/array-root-refill/stdout"),
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_operational_unset355/captures.tsv"), "8.6.18/array-root-refill/stdout"),
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_operational_unset355/captures.tsv"), "9.0.4/array-root-refill/stdout"),
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_operational_unset355/captures.tsv"), "9.1.0/array-root-refill/stdout"),
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_operational_unset355/captures.tsv"), "jim/array-root-refill/stdout")
        ],
    ),
    (
        "native_array_unset_relookup356/whole-root-relookup",
        include_bytes!("../../../tcl-registry/tests/data/native_array_unset_relookup356/whole-root-relookup.tcl").as_slice(),
        [
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_unset_relookup356/captures.tsv"), "8.4.20/whole-root-relookup/stdout"),
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_unset_relookup356/captures.tsv"), "8.5.19/whole-root-relookup/stdout"),
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_unset_relookup356/captures.tsv"), "8.6.18/whole-root-relookup/stdout"),
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_unset_relookup356/captures.tsv"), "9.0.4/whole-root-relookup/stdout"),
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_unset_relookup356/captures.tsv"), "9.1.0/whole-root-relookup/stdout"),
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_unset_relookup356/captures.tsv"), "jim/whole-root-relookup/stdout")
        ],
    ),
    (
        "native_array_unset_relookup356/member-held-root",
        include_bytes!("../../../tcl-registry/tests/data/native_array_unset_relookup356/member-held-root.tcl").as_slice(),
        [
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_unset_relookup356/captures.tsv"), "8.4.20/member-held-root/stdout"),
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_unset_relookup356/captures.tsv"), "8.5.19/member-held-root/stdout"),
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_unset_relookup356/captures.tsv"), "8.6.18/member-held-root/stdout"),
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_unset_relookup356/captures.tsv"), "9.0.4/member-held-root/stdout"),
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_unset_relookup356/captures.tsv"), "9.1.0/member-held-root/stdout"),
            tcl_test_support::fixture_tables::text(include_str!("../../../tcl-registry/tests/data/native_array_unset_relookup356/captures.tsv"), "jim/member-held-root/stdout")
        ],
    )
    ]
}

#[test]
fn original_array_unset_matches_native_refills_and_release_selected_lookup() {
    // Whole unchanged originals compare public results only. No native table,
    // original-header or compiled activation equivalence follows from them.
    let providers = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"];
    let mut comparisons = 0;
    for (case, source, columns) in unset_controls() {
        for (engine, column) in providers.iter().zip(columns) {
            let (expected_code, expected_result) = original_result(&column);
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = crate::native_fixture::interpreter(profile);
            for &library in tcl_registry::native_scripted_distribution::NativeScriptedLibrary::ALL {
                vm.install_scripted_library(library);
            }
            let completion = vm
                .eval_source(std::str::from_utf8(source).unwrap())
                .unwrap();
            assert!(
                vm.refused_completion().is_none(),
                "{engine}/{case}: {:?}",
                vm.refused_completion()
            );
            assert_eq!(
                completion.code.as_int(),
                expected_code,
                "{engine}/{case}: {completion:?}"
            );
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                expected_result,
                "{engine}/{case}"
            );
            comparisons += 1;
        }
    }
    assert_eq!(comparisons, 24);
}

fn original_result(rows: &str) -> (i64, Vec<u8>) {
    let (code, result) = rows
        .lines()
        .find_map(|line| line.strip_prefix("ORIGINAL|"))
        .unwrap()
        .split_once('|')
        .unwrap();
    (
        code.parse().unwrap(),
        result
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect(),
    )
}
